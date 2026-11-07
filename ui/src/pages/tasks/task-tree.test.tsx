/**
 * AJ-E2806 R3 (amended): the plan is an outline: each item with its
 * level, each fold saying whether it's open, folded below the first level;
 * the arrows move, unfold and fold as the WAI-ARIA treegrid pattern says.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { TaskTree } from "./TaskTree";
import { forgetPaged } from "../../lib/paged";
import type { TaskInfo } from "../../lib/ipc/tasks";

afterEach(() => {
  cleanup();
  forgetPaged();
});

const item = (over: Partial<TaskInfo>): TaskInfo => ({
  id: "aj-e001",
  pool_id: "pool-x",
  kind: "epic",
  parent_id: null,
  title: "Checkout v2",
  status: "in_progress",
  stored_status: null,
  priority: "normal",
  holder: null,
  holder_session_id: null,
  blocked_by: [],
  waits_on: [],
  items: 0,
  items_done: 0,
  written_by: "Planner",
  by_agent: true,
  updated_at_ms: 1,
  issue_label: null,
  ...over,
});

describe("the plan as an outline moved like a treegrid (AJ-E2806 R3)", () => {
  it("names its rows' levels and folds, and moves with the arrows", () => {
    const plan = [item({}), item({ id: "aj-s001", kind: "story", parent_id: "aj-e001", title: "Totals" })];
    const view = render(<TaskTree plan={plan} poolId="pool-x" open={null} now={2} onOpen={() => undefined} />);
    const grid = view.getByRole("list", { name: "The plan" });
    const rows = () => [...grid.querySelectorAll<HTMLElement>(":scope > li")];
    const folded = (n: number) => rows()[n]!.querySelector("[data-testid='tree-fold']")!.getAttribute("aria-expanded");
    expect(rows().map((row) => row.getAttribute("aria-level"))).toEqual(["1", "2"]);
    expect([folded(0), folded(1)]).toEqual(["true", "false"]);
    const button = (n: number) => rows()[n]!.querySelector<HTMLElement>("[data-testid='task-row']")!;
    button(0).focus();
    fireEvent.keyDown(button(0), { key: "ArrowDown" });
    expect(document.activeElement).toBe(button(1));
    // → unfolds the story; ← folds it again, then goes to its epic
    fireEvent.keyDown(button(1), { key: "ArrowRight" });
    expect(folded(1)).toBe("true");
    fireEvent.keyDown(button(1), { key: "ArrowLeft" });
    expect(folded(1)).toBe("false");
    fireEvent.keyDown(button(1), { key: "ArrowLeft" });
    expect(document.activeElement).toBe(button(0));
  });
});
