/**
 * AJ-E2806 R7: a task waiting on your review is in Needs you too, and
 * opens beside its pool's tasks.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { ActionHostProvider } from "./ItemMenu";
import { TaskNeeds } from "./TaskNeeds";
import { takeFocusedTask, type TaskNeedInfo } from "../lib/ipc/tasks";
import { rememberedTabId } from "../pages/registry";

afterEach(cleanup);

const review: TaskNeedInfo = {
  id: "review:aj-0003",
  kind: "review",
  pool_name: "Checkout",
  project_id: null,
  pool_id: "pool-checkout",
  task_title: "Checkout copy pass",
  task_id: "aj-0003",
  title: "«Checkout copy pass» waits on your review",
  detail: "Writer asks you to review it · approve it or send it back",
  offer_always: false,
  at_ms: 1,
};

describe("a task in review in Needs you (AJ-E2806 R7)", () => {
  it("says so by its title and opens it beside its pool's tasks", () => {
    const opened: string[] = [];
    const view = render(
      <ActionHostProvider host={{ openPool: (id) => void opened.push(id) }}>
        <TaskNeeds needs={[review]} projects={[]} />
      </ActionHostProvider>,
    );
    expect(view.getByTestId("task-need-review").textContent).toContain("«Checkout copy pass» waits on your review");
    fireEvent.click(view.getByTestId("task-need-review-open"));
    expect(opened).toEqual(["pool-checkout"]);
    expect(takeFocusedTask()).toBe("aj-0003");
    expect(rememberedTabId("pool")).toBe("tasks");
  });
});
