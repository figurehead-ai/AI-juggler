/**
 * AJ-E1552 — a pool's and a session's Tasks draw a skeleton only while
 * their read loads: the task list, the plan and a session's tasks each
 * show a skeleton while loading (Still loading after 3 s), their own
 * empty words, and an error with Retry that reads again. An open task is
 * one record: loading and an error with Retry, never empty.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MOCK_SESSIONS } from "../../preview/fixtures";
import { forgetPaged } from "../../lib/paged";
import * as ipc from "../../lib/ipc/tasks";
import { expectEmpty, expectErrorRetry, expectSlowSkeleton, never } from "../../test/loading-kit";
import { PoolTasks, SessionTasks } from "./PoolTasks";
import { TaskView } from "./TaskView";

const none = { items: [], next_cursor: null, total: 0 };
const zero = { ready: 0, in_progress: 0, in_review: 0, blocked: 0, todo: 0, done: 0, cancelled: 0 };

beforeEach(() => {
  vi.spyOn(console, "warn").mockImplementation(() => {});
  vi.spyOn(ipc, "tasksCounts").mockResolvedValue(zero);
});
afterEach(() => {
  cleanup();
  forgetPaged();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const pool = () => render(<PoolTasks poolId="pool-a" sessions={[]} projects={[]} now={0} />);
const session = () => render(<SessionTasks session={MOCK_SESSIONS[0]!} now={0} />);
const task = () => render(<TaskView taskId="t-1" sessions={[]} defaultAccountId={null} now={0} onOpenTask={() => {}} />);

describe("a pool's task list", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "tasksPlan").mockResolvedValue(none);
    vi.spyOn(ipc, "tasksList").mockReturnValue(never());
    pool();
    expect(screen.getByLabelText("Loading the tasks")).toBeTruthy();
    await expectSlowSkeleton("loading-state");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "tasksPlan").mockResolvedValue(none);
    vi.spyOn(ipc, "tasksList").mockResolvedValue(none);
    pool();
    await expectEmpty("tasks-empty", "No tasks yet.", "loading-state");
  });
  it("error with Retry", async () => {
    vi.spyOn(ipc, "tasksPlan").mockResolvedValue(none);
    const read = vi.spyOn(ipc, "tasksList").mockRejectedValue(new Error("down"));
    pool();
    await expectErrorRetry("the tasks", read);
    expect(screen.queryByText("down")).toBeNull();
  });
});

describe("a pool's plan", () => {
  const plan = () => {
    pool();
    fireEvent.click(screen.getByTestId("tasks-view-plan"));
  };
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "tasksList").mockResolvedValue(none);
    vi.spyOn(ipc, "tasksPlan").mockReturnValue(never());
    plan();
    expect(screen.getByLabelText("Loading the plan")).toBeTruthy();
    await expectSlowSkeleton("loading-state");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "tasksList").mockResolvedValue(none);
    vi.spyOn(ipc, "tasksPlan").mockResolvedValue(none);
    plan();
    await expectEmpty("tree-empty", /./, "loading-state");
  });
  it("error with Retry", async () => {
    vi.spyOn(ipc, "tasksList").mockResolvedValue(none);
    const read = vi.spyOn(ipc, "tasksPlan").mockRejectedValue(new Error("down"));
    plan();
    await expectErrorRetry("the plan", read);
  });
});

describe("a session's tasks", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "tasksOfSession").mockReturnValue(never());
    session();
    await expectSlowSkeleton("session-tasks-skeleton");
  });
  it("empty", async () => {
    vi.spyOn(ipc, "tasksOfSession").mockResolvedValue(none);
    session();
    await expectEmpty("session-tasks-empty", "It holds no task and hasn't worked one.", "session-tasks-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "tasksOfSession").mockRejectedValue(new Error("down"));
    session();
    await expectErrorRetry("its tasks", read, "session-tasks-skeleton");
  });
});

describe("an open task (one record: never empty)", () => {
  it("loading", async () => {
    vi.useFakeTimers();
    vi.spyOn(ipc, "tasksDetail").mockReturnValue(never());
    task();
    await expectSlowSkeleton("task-view-skeleton");
  });
  it("error with Retry", async () => {
    const read = vi.spyOn(ipc, "tasksDetail").mockRejectedValue(new Error("down"));
    task();
    await expectErrorRetry("this task", read, "task-view-skeleton");
  });
});
