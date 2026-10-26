/**
 * E19 vitest — tasks in AI Juggler (AJ-E1903, E1904, E1906).
 *
 * - A pool's Tasks tab: ready work first, how many in each state, a
 *   filter, a page at a time; the plan (epics and stories) behind its own
 *   view; the open task in full: who has it (their session one click
 *   away), what it asks, when it's done, what it waits on, its history.
 * - You add a task, change its status, give it to someone, link it.
 * - "Start a session for it": you pick the launch settings (only your own
 *   project default is picked for you), then Start; never by usage.
 * - Needs you: an agent's held change is the same approval card (keys),
 *   "Always allow" only from the second time; an agent's request for a
 *   session is a row you start or leave.
 * - A session's Tasks tab; the switch history's task per session; the
 *   Processes link only once that view says it's there.
 * - 200 tasks render in under 100 ms; nothing smaller than 12 px.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import PoolTasksSource from "./PoolTasks.tsx?raw";
import TaskViewSource from "./TaskView.tsx?raw";
import TaskRowSource from "./TaskRow.tsx?raw";
import TaskFormSource from "./TaskForm.tsx?raw";
import TaskTreeSource from "./TaskTree.tsx?raw";
import TaskLinksSource from "./TaskLinks.tsx?raw";
import StartSessionSource from "./StartSession.tsx?raw";
import TaskNeedsSource from "../../components/TaskNeeds.tsx?raw";
import PillSource from "../../components/Pill.tsx?raw";
import { PoolTasks, SessionTasks } from "./PoolTasks";
import { TaskRow } from "./TaskRow";
import { TaskNeeds } from "../../components/TaskNeeds";
import { SwitchGraph } from "../../components/SwitchGraph";
import { pageTabs } from "../registry";
import { MOCK_SESSIONS, type ProjectInfo } from "../../lib/bridge";
import { forgetPaged } from "../../lib/paged";
import { provideProcessesLink } from "../../lib/task-links";
import { resetPreviewTasks, type TaskInfo, type TaskNeedInfo, type TasksSeed } from "../../lib/ipc/tasks";

type Seams = { __AIJ_TASKS__?: TasksSeed; __AIJ_TASKS_CALLS__?: string[]; __AIJ_RECIPES__?: unknown[] };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_TASKS_CALLS__ ?? [];
const NOW = 1_728_800_000_000;
const live = MOCK_SESSIONS[0]!;

const task = (over: Partial<TaskInfo>): TaskInfo => ({
  id: "aj-7f3c",
  pool_id: "pool-team",
  kind: "task",
  parent_id: null,
  title: "Build the orders API",
  status: "ready",
  stored_status: null,
  priority: "normal",
  holder: null,
  holder_session_id: null,
  blocked_by: [],
  items: 0,
  items_done: 0,
  written_by: "Planner",
  by_agent: true,
  updated_at_ms: NOW - 60_000,
  issue_label: null,
  ...over,
});

const need = (over: Partial<TaskNeedInfo>): TaskNeedInfo => ({
  id: "tneed-1",
  kind: "ask",
  pool_id: "pool-team",
  pool_name: "Checkout redesign",
  task_id: "aj-7f3c",
  task_title: "Build the orders API",
  title: "«Planner» wants to give aj-7f3c to Builder",
  detail: "Checkout redesign · above what the pool lets agents do",
  offer_always: false,
  project_id: "p-shop",
  at_ms: NOW,
  ...over,
});

const project = { id: "p-shop", name: "shop", default_account_id: "claude-personal" } as unknown as ProjectInfo;

beforeEach(() => {
  resetPreviewTasks();
  seams().__AIJ_TASKS_CALLS__ = [];
  seams().__AIJ_TASKS__ = {
    tasks: [
      task({ id: "aj-0d01", title: "Write the docs", status: "done", updated_at_ms: NOW - 9_000_000 }),
      task({ id: "aj-b10c", title: "Totals in cents", status: "blocked", stored_status: "ready", blocked_by: ["aj-7f3c"] }),
      task({ id: "aj-1a2b", title: "Cart endpoints", status: "in_progress", holder: "Builder", holder_session_id: live.id }),
      task({}),
      task({ id: "aj-e001", kind: "epic", title: "Checkout redesign", items: 3, items_done: 1, written_by: "You", by_agent: false }),
    ],
    details: {
      "aj-7f3c": {
        body: "Orders first, then the cart.",
        acceptance: "- tests pass\n- the docs say how",
        links: [{ kind: "blocks", task_id: "aj-b10c", title: "Totals in cents", status: "blocked" }],
        history: [{ id: 2, at_ms: NOW - 60_000, who: "Planner", by_agent: true, what: "added it" }],
        threads: [{ thread_id: "thr-api", subject: "Start on the API", count: 4, at_ms: NOW - 30_000 }],
      },
      "aj-1a2b": { sessions: [{ session_id: live.id, name: "Builder", how: "claimed", live: true, at_ms: NOW - 600_000 }] },
    },
    sessions: { [live.id]: ["aj-1a2b"] },
  };
  seams().__AIJ_RECIPES__ = [
    { id: "p-2", name: "Weekend build", account_id: "claude-personal", account_label: "Personal", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null },
    { id: "p-1", name: "Deep work", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null },
  ];
  provideProcessesLink(null);
});
// Each test reads its own lists: none remembered from the one before.
afterEach(() => (cleanup(), forgetPaged()));

const members = [
  { session_id: live.id, name: "Builder" },
  { session_id: "sess-done-1", name: "Reviewer" },
];

describe("a pool's tasks", () => {
  it("lists ready work first with how many in each state, and opens a task in full", async () => {
    const { findAllByTestId, getByTestId, findByTestId } = render(<PoolTasks poolId="pool-team" members={members} sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    const rows = await findAllByTestId("task-row");
    // The work: tasks only (the epic is in the plan), ready first.
    expect(rows.map((r) => r.dataset.status)).toEqual(["ready", "in_progress", "blocked"]);
    expect(getByTestId("tasks-filter-open").textContent).toBe("Open3");
    expect(getByTestId("tasks-filter-blocked").textContent).toBe("Blocked1");
    const view = await findByTestId("task-view");
    expect(view.textContent).toContain("Written by Planner (an agent: its words, not yours)");
    expect((await findByTestId("task-acceptance")).textContent).toContain("- tests pass");
    expect(within(getByTestId("task-links")).getByTestId("task-link").textContent).toContain("Blocks");
    expect(getByTestId("task-threads").textContent).toContain("Start on the API");
  });

  it("reads a page at a time: Show more reads the next, none twice", async () => {
    seams().__AIJ_TASKS__!.tasks = Array.from({ length: 45 }, (_, n) => task({ id: `aj-${(0x2000 + n).toString(16)}`, title: `Task ${n}` }));
    const { findAllByTestId, findByTestId, queryByTestId } = render(<PoolTasks poolId="pool-team" members={members} sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    expect(await findAllByTestId("task-row")).toHaveLength(40);
    fireEvent.click(await findByTestId("tasks-more"));
    await waitFor(() => expect(document.querySelectorAll("[data-testid='task-row']")).toHaveLength(45));
    const ids = [...document.querySelectorAll<HTMLElement>("[data-testid='task-row']")].map((row) => row.textContent);
    expect(new Set(ids).size).toBe(45);
    expect(queryByTestId("tasks-more")).toBeNull();
  });

  it("a blocked task says what it waits on, honestly", async () => {
    const { findAllByTestId, findByTestId } = render(<PoolTasks poolId="pool-team" members={members} sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    const blocked = (await findAllByTestId("task-row")).find((r) => r.dataset.status === "blocked")!;
    expect(blocked.textContent).toContain("Waits on aj-7f3c");
    fireEvent.click(blocked);
    expect((await findByTestId("task-view")).textContent).toContain("Set ready, but it waits on aj-7f3c");
  });

  it("filters to done ones, and shows the plan", async () => {
    const { findAllByTestId, getByTestId, findByTestId } = render(<PoolTasks poolId="pool-team" members={members} sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    await findAllByTestId("task-row");
    fireEvent.click(getByTestId("tasks-filter-closed"));
    await waitFor(() => expect(document.querySelectorAll("[data-testid='task-row']")).toHaveLength(1));
    fireEvent.click(getByTestId("tasks-view-plan"));
    const tree = await findByTestId("task-tree");
    expect(tree.textContent).toContain("Checkout redesign");
    expect(tree.textContent).toContain("1 of 3 done");
  });

  it("you add a task, change its status and give it to someone", async () => {
    const { findByTestId, getByTestId } = render(<PoolTasks poolId="pool-team" members={members} sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    await findByTestId("task-view");
    fireEvent.change(getByTestId("task-set-status"), { target: { value: "in_review" } });
    await waitFor(() => expect(calls()).toContain("edit:aj-7f3c:in_review"));
    fireEvent.change(getByTestId("task-set-holder"), { target: { value: "sess-done-1" } });
    await waitFor(() => expect(calls()).toContain("assign:aj-7f3c:sess-done-1"));
    fireEvent.click(getByTestId("tasks-new"));
    fireEvent.change(await findByTestId("task-form-title"), { target: { value: "Refunds in cents" } });
    fireEvent.click(getByTestId("task-form-save"));
    await waitFor(() => expect(calls()).toContain("create:task:Refunds in cents"));
  });

  it("starts a session for a task only on your pick: your project's default, never by usage", async () => {
    const { findByTestId, getByTestId } = render(<PoolTasks poolId="pool-team" members={members} sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    await findByTestId("task-view");
    fireEvent.click(getByTestId("task-start-open"));
    const picker = (await findByTestId("task-start-profile")) as HTMLSelectElement;
    await waitFor(() => expect(picker.options.length).toBe(3));
    expect([...picker.options].map((o) => o.textContent)).toEqual(["Pick launch settings", "Deep work · Work Max", "Weekend build · Personal"]);
    // This pool's project has no default account: nothing is picked for you.
    expect(picker.value).toBe("");
    expect((getByTestId("task-start-go") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(picker, { target: { value: "p-1" } });
    fireEvent.click(getByTestId("task-start-go"));
    await waitFor(() => expect(calls()).toContain("start:aj-7f3c:p-1:"));
  });

  it("a held task's session is one click away; Processes shows only once that view is there", async () => {
    const { findAllByTestId, findByTestId, queryByTestId, rerender } = render(<PoolTasks poolId="pool-team" members={members} sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    fireEvent.click((await findAllByTestId("task-row")).find((r) => r.dataset.status === "in_progress")!);
    expect((await findByTestId("task-holder")).textContent).toContain("In progress with Builder, running now");
    expect(queryByTestId("task-processes")).toBeNull();
    provideProcessesLink(() => ({ label: "Its processes", open: () => undefined }));
    rerender(<PoolTasks poolId="pool-team" members={members} sessions={MOCK_SESSIONS} projects={[project]} now={NOW + 1} />);
    expect((await findByTestId("task-processes")).textContent).toBe("Its processes");
  });

  it("the session page's Processes tab is what a task's Processes link opens", async () => {
    // Loading the session page's tabs registers it (D-41's tab is there).
    await import("../session/session-tabs");
    const { processesLinkFor } = await import("../../lib/task-links");
    const link = processesLinkFor(live.id)!;
    expect(link.label).toBe("Its processes");
    link.open();
    expect(localStorage.getItem("aij.page.tab.session")).toBe("processes");
  });

  it("renders 200 tasks in under 100 ms", async () => {
    seams().__AIJ_TASKS__!.tasks = Array.from({ length: 200 }, (_, n) => task({ id: `aj-${(0x1000 + n).toString(16)}`, title: `Task ${n}` }));
    const started = performance.now();
    const { findAllByTestId } = render(<SessionTasksList />);
    expect((await findAllByTestId("task-row")).length).toBe(200);
    expect(performance.now() - started).toBeLessThan(100);
  });
});

/** All of the preview's tasks as rows (the render budget's list). */
function SessionTasksList() {
  const rows = seams().__AIJ_TASKS__!.tasks!;
  return (
    <ul>
      {rows.map((t) => (
        <TaskRowFor key={t.id} task={t} />
      ))}
    </ul>
  );
}
function TaskRowFor({ task: t }: { task: TaskInfo }) {
  return <TaskRow task={t} now={NOW} selected={false} onOpen={() => undefined} />;
}

describe("Needs you", () => {
  it("an agent's held change is the same approval card, answered from the keyboard", async () => {
    const { getByTestId, queryByText } = render(<TaskNeeds needs={[need({})]} projects={[project]} />);
    const card = getByTestId("ask-card");
    expect(card.textContent).toContain("An agent asks about a task");
    expect(card.textContent).toContain("«Build the orders API»");
    expect(queryByText("Always allow in this pool")).toBeNull();
    fireEvent.keyDown(card, { key: "1" });
    await waitFor(() => expect(calls()).toContain("answer:tneed-1:once"));
  });

  it("offers the rule from the second time", () => {
    const { getAllByTestId } = render(<TaskNeeds needs={[need({ offer_always: true })]} projects={[project]} />);
    expect(getAllByTestId("ask-option-always")).toHaveLength(1);
  });

  it("an agent's request for a session: you start it on your pick, or leave it", async () => {
    const launch = need({ id: "tneed-2", kind: "launch", title: "«Planner» asks for a new session for aj-7f3c «Build the orders API»", detail: "«Use the Work account»" });
    const { getByTestId, findByTestId } = render(<TaskNeeds needs={[launch]} projects={[project]} />);
    const row = getByTestId("task-need-launch");
    expect(row.textContent).toContain("in shop");
    const picker = (await findByTestId("task-start-profile")) as HTMLSelectElement;
    await waitFor(() => expect(picker.value).toBe("p-2"));
    fireEvent.click(within(row).getByTestId("task-start-go"));
    await waitFor(() => expect(calls()).toContain("start:aj-7f3c:p-2:tneed-2"));
    fireEvent.click(within(row).getByRole("button", { name: /^Not now/ }));
    await waitFor(() => expect(calls()).toContain("dismiss:tneed-2"));
  });
});

describe("a session's tasks and the pages", () => {
  it("a session shows what it holds", async () => {
    const { findAllByTestId } = render(<SessionTasks session={live} now={NOW} />);
    const rows = await findAllByTestId("task-row");
    expect(rows.map((r) => r.textContent)).toEqual([expect.stringContaining("Cart endpoints")]);
  });

  it("the switch history shows each session's task", async () => {
    (window as unknown as { __AIJ_SWITCH_GRAPH__?: unknown }).__AIJ_SWITCH_GRAPH__ = [
      {
        title: "Orders API",
        epochs: [
          { id: "ep-1", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-a", task: "aj-7f3c · Build the orders API" },
          { id: "ep-2", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-b", task: "aj-7f3c · Build the orders API" },
        ],
        edges: [{ from: "ep-1", to: "ep-2", label: "Full story, fresh start", switch_back: false }],
        collapsed: 0,
        hop_note: null,
      },
    ];
    const { findAllByTestId } = render(<SwitchGraph projectId="p-shop" onOpenSession={() => undefined} />);
    const tasks = await findAllByTestId("node-task");
    expect(tasks.map((t) => t.textContent)).toEqual(["aj-7f3c · Build the orders API", "aj-7f3c · Build the orders API"]);
    delete (window as unknown as { __AIJ_SWITCH_GRAPH__?: unknown }).__AIJ_SWITCH_GRAPH__;
  });

  it("a pool always has its Tasks tab; a session only once it has a task", () => {
    expect(pageTabs("pool").map((t) => t.label)).toEqual(["Members", "Tasks", "Messages", "Agent access"]);
    expect(pageTabs("session").some((t) => t.id === "tasks")).toBe(false);
    expect(pageTabs("session", { hasTasks: true }).some((t) => t.id === "tasks")).toBe(true);
  });
});

describe("readability", () => {
  it("no text smaller than 12 px in the task views", () => {
    const files: Record<string, string> = { PoolTasksSource, TaskViewSource, TaskRowSource, TaskFormSource, TaskTreeSource, TaskLinksSource, StartSessionSource, TaskNeedsSource, PillSource };
    for (const [file, text] of Object.entries(files)) {
      const small = (text.match(/text-\[(\d+(?:\.\d+)?)px\]/g) ?? []).filter((cls: string) => Number(cls.slice(6, -3)) < 12);
      expect(small, file).toEqual([]);
    }
  });
});
