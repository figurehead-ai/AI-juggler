/**
 * 0.4.1: the Tasks screen's project filter offers "No project", last, and
 * choosing it asks for the tasks with no project (`project_id: ""`, as the
 * Sessions list says it), for the columns and their counts alike.
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ActionHostProvider } from "../../components/ItemMenu";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewTasks } from "../../lib/ipc/tasks";
import type { TasksFilterInput } from "../../lib/ipc/tasks-across";
import { forgetPaged } from "../../lib/paged";
import { resetPools } from "../../lib/pools";
import { TasksScreen } from "./TasksScreen";

const asked: { rows: TasksFilterInput[]; counts: TasksFilterInput[] } = { rows: [], counts: [] };

vi.mock("../../lib/ipc/tasks-across", async (importOriginal) => {
  const real = await importOriginal<typeof import("../../lib/ipc/tasks-across")>();
  return {
    ...real,
    tasksAcross: (filter: TasksFilterInput, page: Parameters<typeof real.tasksAcross>[1]) => (asked.rows.push(filter), real.tasksAcross(filter, page)),
    tasksAcrossCounts: (filter: TasksFilterInput) => (asked.counts.push(filter), real.tasksAcrossCounts(filter)),
  };
});

const NOW = 1_792_600_000_000;

beforeEach(() => {
  asked.rows = [];
  asked.counts = [];
  resetPreviewTasks();
  resetPools();
  Object.assign(window, { __AIJ_TASKS_CALLS__: [], __AIJ_TASKS__: { tasks: [] } });
});
afterEach(() => {
  cleanup();
  forgetPaged();
  localStorage.clear();
});

describe("the project filter's No project (0.4.1)", () => {
  it("offers No project, last, and choosing it sends project_id \"\"", async () => {
    const view = render(
      <ActionHostProvider host={{}}>
        <TasksScreen projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} now={NOW} />
      </ActionHostProvider>,
    );
    fireEvent.click(view.getByTestId("tasks-filter-project"));
    const choice = await view.findByTestId("tasks-filter-project-(none)");
    expect(choice.textContent).toContain("No project");
    const lines = view.getAllByTestId(/^tasks-filter-project-/);
    expect(lines[lines.length - 1]).toBe(choice);
    asked.rows = [];
    asked.counts = [];
    fireEvent.click(choice);
    await waitFor(() => expect(asked.counts.some((filter) => filter.project_id === "")).toBe(true));
    await waitFor(() => expect(asked.rows.some((filter) => filter.project_id === "")).toBe(true));
    expect(view.getByTestId("tasks-filter-project").textContent).toContain("No project");
  });
});
