/**
 * 0.5.1 F63 — a task card's place line: its parts are joined only when
 * there are two, and the "·" travels with who has it (or what it waits
 * on), so no line ends on a dangling "·".
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TaskCard } from "./TaskCard";
import type { TaskBoardRow } from "../../lib/ipc/tasks-across";
import type { TaskInfo } from "../../types/generated/TaskInfo";

afterEach(cleanup);

const task = (over: Partial<TaskInfo> = {}): TaskInfo => ({
  id: "t1", pool_id: "p1", kind: "task", parent_id: null, title: "Fix the cart totals", status: "in_progress", stored_status: null, priority: "normal",
  holder: "Builder", holder_session_id: null, blocked_by: [], waits_on: [], items: 0, items_done: 0, written_by: "Planner", by_agent: true, updated_at_ms: 0,
  issue_label: null, ...over,
});

const row = (over: Partial<TaskBoardRow> = {}, of: Partial<TaskInfo> = {}): TaskBoardRow => ({ task: task(of), pool_name: "Checkout v2", project_id: "storefront", path: [], ...over });

function card(r: TaskBoardRow, projectName: string | null = "storefront") {
  return render(
    <ul>
      <TaskCard row={r} projectName={projectName} holderApp="Claude Code CLI · Personal" selected={false} onOpen={vi.fn()} onOpenTask={vi.fn()} onOpenProject={vi.fn()} />
    </ul>,
  );
}

describe("a task card's place line (F63)", () => {
  it("the · goes with who has it, in one line with the project", () => {
    card(row());
    const line = screen.getByTestId("task-card-place");
    expect(line.className).not.toContain("flex-wrap");
    const who = screen.getByTestId("task-card-who");
    expect(who.textContent).toBe("·Builder · Claude Code CLI · Personal");
    expect(screen.getByTestId("task-card-project").textContent).toBe("storefront");
    // The place itself ends on its name, never on a separator.
    expect(line.firstElementChild?.textContent).toBe("storefront");
  });

  it("the · goes with what it waits on", () => {
    card(row({}, { status: "blocked", waits_on: [{ id: "t2", title: "Refresh-token retries" }] }));
    expect(screen.getByTestId("task-card-who").textContent).toBe("·Waits on:Refresh-token retries");
  });

  it("no place, no ·: an empty part is never joined", () => {
    card(row({ pool_name: "", project_id: null }), null);
    const who = screen.getByTestId("task-card-who");
    expect(who.textContent).toBe("Builder · Claude Code CLI · Personal");
    expect(screen.getByTestId("task-card-place").textContent).not.toContain("·Builder");
    expect(screen.getByTestId("task-card-place").textContent.startsWith("·")).toBe(false);
  });
});
