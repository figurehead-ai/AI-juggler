/**
 * AJ-E2808 R1, R5, R8 (vitest): the pool canvas's answers and moves.
 *
 * - An agent's ask on its node: Always allow here and Deny answer it.
 * - Tidy up lays the pool out again, and Undo puts every node back.
 * - The pool's inspector says when a change of what agents may do applies.
 * - Adding a session from the palette only moves it into the pool: it
 *   never starts, stops or types into a session.
 */
import { act, cleanup, fireEvent, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { poolCanvasPlace } from "../../lib/ipc/pool-canvas";
import { asElement } from "../../test/fixtures";
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
