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
import { CanvasEdges, edgeWords } from "./CanvasEdges";
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
  it("moves it into the pool and starts nothing", async () => {
    seedTeam();
    const view = await openPage("pool-team", [first, second, { ...third, status: "running" }]);
    const palette = view.getByTestId("canvas-palette");
    const header = within(palette).getAllByRole("button")[0]!;
    if (header.getAttribute("aria-expanded") !== "true") fireEvent.click(header);
    const item = (await view.findAllByTestId("canvas-palette-item")).find((row) => row.dataset.session === third.id);
    expect(item).toBeTruthy();
    fireEvent.click(within(item!).getByTestId("canvas-palette-add"));
    await waitFor(() => expect(seams().__AIJ_POOL_CALLS__).toContain(`move:${third.id}:pool-team`));
    expect(seams().__AIJ_POOL_CALLS__!.every((called) => called.startsWith("move:"))).toBe(true);
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
