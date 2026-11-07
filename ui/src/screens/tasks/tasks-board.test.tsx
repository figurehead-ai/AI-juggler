/**
 * AJ-E2806 R2: a card says who has it and the App they work in, and a
 * task in review offers Approve and Send back… on the card itself.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { ActionHostProvider } from "../../components/ItemMenu";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewTasks, type TaskInfo } from "../../lib/ipc/tasks";
import { sessionAppName } from "../../lib/app-name";
import { forgetPaged } from "../../lib/paged";
import { resetPools } from "../../lib/pools";
import { TasksScreen } from "./TasksScreen";

const NOW = 1_792_600_000_000;
const SHOP = MOCK_PROJECTS[0]!;
const BUILDER = MOCK_SESSIONS[0]!;
const calls = (): string[] => {
  const kept: unknown = Reflect.get(window, "__AIJ_TASKS_CALLS__");
  return Array.isArray(kept) ? kept.map(String) : [];
};

const task = (over: Partial<TaskInfo>): TaskInfo => ({
  id: "aj-0001",
  pool_id: `pool-default:${SHOP.id}`,
  kind: "task",
  parent_id: null,
  title: "Round tax per line",
  status: "ready",
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
  updated_at_ms: NOW - 60_000,
  issue_label: null,
  ...over,
});

beforeEach(() => {
  resetPreviewTasks();
  resetPools();
  Object.assign(window, {
    __AIJ_TASKS_CALLS__: [],
    __AIJ_TASKS__: {
      tasks: [
        task({ id: "aj-0002", title: "Fix the cart totals", status: "in_progress", holder: "Builder", holder_session_id: BUILDER.id }),
        task({ id: "aj-0003", title: "Checkout copy pass", status: "in_review", holder: "Writer" }),
      ],
    },
  });
});
afterEach(() => {
  cleanup();
  forgetPaged();
  localStorage.clear();
});

describe("the board's cards (AJ-E2806 R2)", () => {
  const screen = () =>
    render(
      <ActionHostProvider host={{}}>
        <TasksScreen projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} now={NOW} />
      </ActionHostProvider>,
    );

  it("says who has a task and the App they work in", async () => {
    const view = screen();
    const card = await waitFor(() => within(view.getByTestId("tasks-column-in_progress")).getByTestId("task-card"));
    expect(card.textContent).toContain(`Builder · ${sessionAppName(BUILDER)}`);
  });

  it("offers Approve and Send back… on a card in review", async () => {
    const view = screen();
    const card = await waitFor(() => within(view.getByTestId("tasks-column-in_review")).getByTestId("task-card"));
    fireEvent.click(within(card).getByTestId("task-card-send-back"));
    const open = await view.findByTestId("tasks-open");
    // opened on its note, ready to say what should change
    await waitFor(() => expect(within(open).queryByTestId("task-send-back-open")).toBeNull());
    // Approve, from the card itself
    fireEvent.click(within(card).getByTestId("task-card-approve"));
    await waitFor(() => expect(calls()).toContain("approve:aj-0003"));
  });
});
