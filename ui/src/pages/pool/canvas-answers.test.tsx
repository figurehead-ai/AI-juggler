/**
 * AJ-E2808 R1, R5, R8 (vitest): the pool canvas's answers and moves.
 *
 * - An agent's ask on its node: Always allow here and Deny answer it.
 * - Tidy up lays the pool out again, and Undo puts every node back.
 * - The pool's inspector says when a change of what agents may do applies.
 * - An edge's line says whether something waits on it (dashed) and its
 *   name says what it carries; a node's menu makes a lead and takes one
 *   out of the pool.
 * - Adding a session from the palette only moves it into the pool: it
 *   never starts, stops or types into a session.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { poolCanvasPlace } from "../../lib/ipc/pool-canvas";
import { asElement } from "../../test/fixtures";
import { defaultPoolIdOf } from "../../lib/pools";
import { CanvasEdges, edgeWords, ownLineRules } from "./CanvasEdges";
import { first, openPage, resetCanvasTests, second, seams, seedTeam, third } from "./canvas.test-kit";

beforeEach(resetCanvasTests);
afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe("an agent's ask on its node", () => {
  it.each([
    ["node-ask-always", "always"],
    ["node-ask-deny", "deny"],
  ])("%s answers it %s", async (testId, answer) => {
    seedTeam(true);
    const view = await openPage();
    const asker = asElement(view.container.querySelector(`[data-node="${second.id}"]`));
    fireEvent.click(await within(asker).findByTestId(testId));
    await waitFor(() => expect(seams().__AIJ_MESSAGES_CALLS__).toContain(`answer:ask-1:${answer}`));
  });

  it("offers Always allow here only when the ask allows it", async () => {
    seedTeam();
    const view = await openPage();
    const asker = asElement(view.container.querySelector(`[data-node="${second.id}"]`));
    await within(asker).findByTestId("node-ask-once");
    expect(within(asker).queryByTestId("node-ask-always")).toBeNull();
    expect(within(asker).getByTestId("node-ask-deny")).toBeTruthy();
  });
});

describe("Tidy up", () => {
  it("forgets the places you gave, and Undo puts them back", async () => {
    seedTeam();
    await act(async () => {
      await poolCanvasPlace("pool-team", [{ session_id: first.id, x: 400, y: 300 }]);
    });
    seams().__AIJ_POOL_CANVAS_CALLS__ = [];
    const view = await openPage();
    fireEvent.click(view.getByTestId("canvas-tidy"));
    await waitFor(() => expect(seams().__AIJ_POOL_CANVAS_CALLS__).toContain("tidy:pool-team"));
    fireEvent.click(await view.findByTestId("toast-undo"));
    await waitFor(() => expect(seams().__AIJ_POOL_CANVAS_CALLS__).toContain(`place:pool-team:${first.id}@400,300`));
  });
});

describe("adding a session", () => {
  // rc31 (D-62 decision 10): it joins, keeping its other pools.
  it("puts it in the pool, keeps its other pools and starts nothing", async () => {
    seedTeam();
    const view = await openPage("pool-team", [first, second, { ...third, status: "running" }]);
    const palette = view.getByTestId("canvas-palette");
    const header = within(palette).getAllByRole("button")[0]!;
    if (header.getAttribute("aria-expanded") !== "true") fireEvent.click(header);
    const item = (await view.findAllByTestId("canvas-palette-item")).find((row) => row.dataset.session === third.id);
    expect(item).toBeTruthy();
    fireEvent.click(within(item!).getByTestId("canvas-palette-add"));
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain(`join:pool-team:session:${third.id}:member:`));
    expect(seams().__AIJ_POOL_CALLS__!.some((called) => called.startsWith("move:"))).toBe(false);
    expect(seams().__AIJ_STARTED__).toBeUndefined();
    expect(seams().__AIJ_MESSAGES_CALLS__ ?? []).toEqual([]);
  });
});

describe("the pool's inspector", () => {
  it("says a change of what agents may do applies from their next call (D-28)", async () => {
    seedTeam();
    const view = await openPage();
    fireEvent.click(view.getByTestId("inspect-open-settings"));
    expect((await view.findByTestId("pool-applies-next")).textContent).toContain("next call");
  });
});

describe("edges and a node's menu (AJ-E2808 R2, R3)", () => {
  it("an edge waiting on you is dashed, a plain one solid, and each has words for a screen reader", () => {
    const at = Date.now();
    const waiting = [{ message_id: "m-1", thread_id: "thr", subject: "Totals", recipient: "session:b", kind: "waiting" as const, ask_id: null, at_ms: at }];
    const edges = [
      { from_session_id: "a", to_session_id: "b", messages: 2, tasks: 1, last_at_ms: at, thread_id: "thr", pending: waiting },
      { from_session_id: "b", to_session_id: "c", messages: 1, tasks: 0, last_at_ms: at, thread_id: "thr2", pending: [] },
    ];
    const points = new Map([["a", { x: 0, y: 0 }], ["b", { x: 400, y: 0 }], ["c", { x: 400, y: 300 }]]);
    const view = render(<CanvasEdges edges={edges} points={points} names={new Map([["a", "Planner"], ["b", "Fix the cart totals"], ["c", "Tests"]])} selected={null} pulses={new Map()} reducedMotion scale={1} onSelect={() => {}} />);
    const line = (key: string) => [...view.container.querySelectorAll(`g[data-edge="${key}"] path`)].find((path) => path.getAttribute("marker-end"));
    expect(line("a>b")?.getAttribute("stroke-dasharray")).toBe("6 5");
    expect(line("b>c")?.getAttribute("stroke-dasharray")).toBeNull();
    expect(edgeWords(edges[0]!, "Planner", "Fix the cart totals").aria).toBe("2 messages · 1 task, Planner to Fix the cart totals");
  });

  it("a line with its own talk rule says so on its label, and a blocked one is drawn apart (P2 step 3)", () => {
    const own = ownLineRules([
      { from: "role:member", to: "role:member", rule: "ask" },
      { from: "session:b", to: "session:c", rule: "blocked" },
    ]);
    expect([...own]).toEqual([["b>c", "blocked"]]);
    const at = Date.now();
    const edges = [{ from_session_id: "b", to_session_id: "c", messages: 1, tasks: 0, last_at_ms: at, thread_id: "thr2", pending: [] }];
    const points = new Map([["b", { x: 400, y: 0 }], ["c", { x: 400, y: 300 }]]);
    const view = render(<CanvasEdges edges={edges} points={points} names={new Map([["a", "Planner"], ["b", "Fix the cart totals"], ["c", "Tests"]])} selected={null} pulses={new Map()} reducedMotion scale={1} lineRules={own} onSelect={() => {}} />);
    const label = view.container.querySelector('[data-testid="edge-label"][data-edge="b>c"]');
    expect(label?.textContent).toContain("Blocked here");
    const line = [...view.container.querySelectorAll('g[data-edge="b>c"] path')].find((path) => path.getAttribute("marker-end"));
    expect(line?.getAttribute("stroke-dasharray")).toBe("2 4");
  });

  it("Make lead from a member's menu, and Take out of the pool from it", async () => {
    seedTeam();
    const view = await openPage();
    const menuOf = async () => {
      const name = asElement(view.container.querySelector(`[data-node-name="${second.id}"]`));
      fireEvent.keyDown(name, { key: "F10", shiftKey: true });
      return within(await view.findByRole("menu"));
    };
    fireEvent.click((await menuOf()).getByTestId("node-menu-lead"));
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain(`role:${second.id}:lead`));
    fireEvent.click((await menuOf()).getByTestId("node-menu-remove"));
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain(`move:${second.id}:${defaultPoolIdOf(second.project_id)}`));
  });
});

describe("the pool's header chips (AJ-E3104 R1)", () => {
  it("a pool you made shows its scope as a chip that opens the scope sheet, and sealed when it's kept apart", async () => {
    seams().__AIJ_POOLS__ = {
      pools: [{ id: "pool-team", name: "Team", sealed: true }],
      members: [
        { session_id: first.id, pool_id: "pool-team", role: "lead" },
        { session_id: second.id, pool_id: "pool-team", role: "member" },
      ],
    };
    const view = await openPage();
    const scope = view.getByTestId("pool-header-scope");
    expect(scope.textContent).toBe("Scopeevery project");
    expect(view.getByTestId("pool-header-sealed").textContent).toContain("sealed");
    // Said once: the facts line no longer repeats where it works.
    expect(view.getByTestId("pool-page-count").textContent).not.toContain("every project");
    fireEvent.click(scope);
    expect(await view.findByTestId("scope-sheet-save")).toBeTruthy();
  });

  it("a project's own pool has neither: its title says its project", async () => {
    seedTeam();
    const view = await openPage(defaultPoolIdOf(third.project_id));
    expect(view.queryByTestId("pool-header-scope")).toBeNull();
    expect(view.queryByTestId("pool-header-sealed")).toBeNull();
  });
});
