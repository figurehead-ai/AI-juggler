/**
 * E19 vitest — tasks in AI Juggler (AJ-E1903, E1904, E1906).
 *
 * - A pool's Tasks tab: ready work first, how many in each state, a
 *   filter, a page at a time; the plan (epics and stories) behind its own
 *   view; the open task in full: who has it (their session one click
 *   away), what it asks, when it's done, what it waits on, its history.
 * - D-44: the app writes no task (no New task, no field to change, links
 *   read only); a task offers only your four actions, each when its state
 *   allows: Approve or Send back (in review, with a note for its holder),
 *   Free (its holder's session ended), Cancel (open). Each is in its
 *   history as yours. The IPC module has no create, edit, assign or link.
 * - "Start a session for it": you pick the launch settings (only your own
 *   project default is picked for you), then Start; never by usage.
 * - Needs you: an agent's held change is the same approval card (keys),
 *   "Always allow" only from the second time; an agent's request for a
 *   session is a row you start or leave.
 * - A session's Tasks tab; the switch history's task per session; the
 *   Processes link only once that view says it's there.
 * - 200 tasks render in under 100 ms (timed on the load gate; elsewhere
 *   each row renders once, under a sanity bound); nothing smaller than 12 px.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { pick } from "../../test/pick";
import { Profiler } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import PoolTasksSource from "./PoolTasks.tsx?raw";
import TaskViewSource from "./TaskView.tsx?raw";
import TaskRowSource from "./TaskRow.tsx?raw";
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
import * as tasksIpc from "../../lib/ipc/tasks";
import TasksIpcSource from "../../lib/ipc/tasks.ts?raw";
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
  waits_on: [],
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
  title: "«Planner» wants to give «Build the orders API» to Builder",
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

describe("a pool's tasks", () => {
  it("lists ready work first with how many in each state, and opens a task in full", async () => {
    const { findAllByTestId, getByTestId, findByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
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
    const { findAllByTestId, findByTestId, queryByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    expect(await findAllByTestId("task-row")).toHaveLength(40);
    fireEvent.click(await findByTestId("tasks-more"));
    await waitFor(() => expect(document.querySelectorAll("[data-testid='task-row']")).toHaveLength(45));
    const ids = [...document.querySelectorAll<HTMLElement>("[data-testid='task-row']")].map((row) => row.textContent);
    expect(new Set(ids).size).toBe(45);
    expect(queryByTestId("tasks-more")).toBeNull();
  });

  it("a blocked task says what it waits on, honestly", async () => {
    const { findAllByTestId, findByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    const blocked = (await findAllByTestId("task-row")).find((r) => r.dataset.status === "blocked")!;
    // D-25: by its title, never its id.
    expect(blocked.textContent).toContain("Waits on Build the orders API");
    expect(blocked.textContent).not.toMatch(/aj-[0-9a-f]{4}/);
    fireEvent.click(blocked);
    expect((await findByTestId("task-view")).textContent).toContain("Set ready, but it waits on Build the orders API");
  });

  it("a switch is a line of the task's history, and the new session one of its sessions (AJ-E1904)", async () => {
    seams().__AIJ_TASKS__!.details!["aj-1a2b"] = {
      sessions: [
        { session_id: "sess-ended", name: "A session that ended", how: "claimed", live: false, at_ms: NOW - 900_000 },
        { session_id: live.id, name: "Builder", how: "switched", live: true, at_ms: NOW - 600_000 },
      ],
      history: [
        { id: 4, at_ms: NOW - 600_000, who: "Builder", by_agent: true, what: "continued on Personal (from Work Max)" },
        { id: 3, at_ms: NOW - 900_000, who: "Builder", by_agent: true, what: "took it" },
      ],
    };
    const { findAllByTestId, findByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    fireEvent.click((await findAllByTestId("task-row")).find((r) => r.dataset.status === "in_progress")!);
    const sessions = await findByTestId("task-sessions");
    expect(sessions.textContent).toContain("A session that ended took it");
    expect(sessions.textContent).toContain("Builder continued it after a switch");
    const history = await findByTestId("task-history");
    expect(history.querySelector("li")!.textContent).toContain("Builder continued on Personal (from Work Max)");
  });

  it("filters to done ones, and shows the plan", async () => {
    const { findAllByTestId, getByTestId, findByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    await findAllByTestId("task-row");
    fireEvent.click(getByTestId("tasks-filter-closed"));
    await waitFor(() => expect(document.querySelectorAll("[data-testid='task-row']")).toHaveLength(1));
    fireEvent.click(getByTestId("tasks-view-plan"));
    const tree = await findByTestId("task-tree");
    expect(tree.textContent).toContain("Checkout redesign");
    expect(tree.textContent).toContain("1 of 3 done");
  });

  it("writes no task: no New task, no field to change, links read only (D-44)", async () => {
    const { findByTestId, queryByTestId, getByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    const view = await findByTestId("task-view");
    expect(queryByTestId("tasks-new")).toBeNull();
    expect(queryByTestId("task-set-status")).toBeNull();
    expect(queryByTestId("task-set-holder")).toBeNull();
    expect(view.querySelectorAll("input, select")).toHaveLength(0);
    const links = getByTestId("task-links");
    expect(within(links).queryByRole("button", { name: /Remove|Add a link/ })).toBeNull();
    expect(within(links).getByTestId("task-link").textContent).toContain("Totals in cents");
    expect(within(links).getByTestId("task-link").textContent).not.toContain("aj-b10c");
    // A ready task no one holds offers a session and Cancel, nothing more.
    expect(queryByTestId("task-approve")).toBeNull();
    expect(queryByTestId("task-free")).toBeNull();
    expect(getByTestId("task-start-open")).toBeTruthy();
    expect(getByTestId("task-cancel-open")).toBeTruthy();
  });

  it("a task in review: you approve it, and your approval is in its history", async () => {
    seams().__AIJ_TASKS__!.tasks!.push(task({ id: "aj-9e41", title: "Shipping rates", status: "in_review", holder: "Builder", holder_session_id: live.id }));
    const { findAllByTestId, findByTestId, getByTestId, queryByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    fireEvent.click((await findAllByTestId("task-row")).find((r) => r.dataset.status === "in_review")!);
    fireEvent.click(await findByTestId("task-approve"));
    await waitFor(() => expect(calls()).toContain("approve:aj-9e41"));
    await waitFor(() => expect(getByTestId("task-history").textContent).toContain("You approved it: done"));
    expect(queryByTestId("task-approve")).toBeNull();
    expect(queryByTestId("task-cancel-open")).toBeNull();
  });

  it("a task in review: you send it back with a note for its holder", async () => {
    seams().__AIJ_TASKS__!.tasks!.push(task({ id: "aj-9e41", title: "Shipping rates", status: "in_review", holder: "Builder", holder_session_id: live.id }));
    const { findAllByTestId, findByTestId, getByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    fireEvent.click((await findAllByTestId("task-row")).find((r) => r.dataset.status === "in_review")!);
    fireEvent.click(await findByTestId("task-send-back-open"));
    const form = getByTestId("task-send-back");
    expect(form.textContent).toContain("Builder gets it as your message");
    const go = getByTestId("task-send-back-go") as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    fireEvent.change(getByTestId("task-send-back-note"), { target: { value: "Rates must come from the carrier, not a table" } });
    fireEvent.click(go);
    await waitFor(() => expect(calls()).toContain("send-back:aj-9e41:Rates must come from the carrier, not a table"));
    await waitFor(() => expect(getByTestId("task-history").textContent).toContain("You sent it back: Rates must come from the carrier, not a table"));
    expect(getByTestId("task-view").querySelector("[data-testid='task-status']")!.textContent).toBe("In progress");
  });

  it("frees only a task whose holder's session ended; a running holder's has no Free", async () => {
    seams().__AIJ_TASKS__!.tasks!.push(task({ id: "aj-3f88", title: "Flaky test", status: "in_progress", holder: "Reviewer", holder_session_id: null }));
    const { findAllByTestId, findByTestId, getByTestId, queryByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    fireEvent.click((await findAllByTestId("task-row")).find((r) => r.textContent?.includes("Cart endpoints"))!);
    await findByTestId("task-holder");
    expect(queryByTestId("task-free")).toBeNull();
    fireEvent.click((await findAllByTestId("task-row")).find((r) => r.textContent?.includes("Flaky test"))!);
    expect((await findByTestId("task-holder-ended")).textContent).toContain("Reviewer, whose session has ended");
    fireEvent.click(getByTestId("task-free"));
    await waitFor(() => expect(calls()).toContain("free:aj-3f88"));
    await waitFor(() => expect(queryByTestId("task-holder-ended")).toBeNull());
    expect(getByTestId("task-history").textContent).toContain("You freed it");
  });

  it("cancels an open task only after you say so; a closed one offers nothing", async () => {
    const { findByTestId, getByTestId, queryByTestId, findAllByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    await findByTestId("task-view");
    fireEvent.click(getByTestId("task-cancel-open"));
    expect(getByTestId("task-cancel").textContent).toContain("Cancel this task?");
    expect(calls()).not.toContain("cancel:aj-7f3c");
    fireEvent.click(getByTestId("task-cancel-go"));
    await waitFor(() => expect(calls()).toContain("cancel:aj-7f3c"));
    await waitFor(() => expect(getByTestId("task-history").textContent).toContain("You cancelled it"));
    expect(queryByTestId("task-actions")).toBeNull();
    fireEvent.click(getByTestId("tasks-filter-closed"));
    fireEvent.click((await findAllByTestId("task-row")).find((r) => r.textContent?.includes("Write the docs"))!);
    await waitFor(() => expect(getByTestId("task-view").textContent).toContain("Write the docs"));
    expect(queryByTestId("task-actions")).toBeNull();
  });

  it("starts a session for a task only on your pick: your project's default, never by usage", async () => {
    const { findByTestId, getByTestId } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    await findByTestId("task-view");
    fireEvent.click(getByTestId("task-start-open"));
    const picker = await findByTestId("task-start-profile");
    await waitFor(() => expect(picker.textContent).toContain("Pick launch settings"));
    fireEvent.click(picker);
    expect(screen.getAllByRole("menuitemradio").map((o) => o.textContent)).toEqual(["Deep work · Work Max", "Weekend build · Personal"]);
    fireEvent.keyDown(document.activeElement!, { key: "Escape" });
    // This pool's project has no default account: nothing is picked for you.
    expect(picker.getAttribute("data-value")).toBe("");
    expect((getByTestId("task-start-go") as HTMLButtonElement).disabled).toBe(true);
    pick(picker, "Deep work · Work Max");
    fireEvent.click(getByTestId("task-start-go"));
    await waitFor(() => expect(calls()).toContain("start:aj-7f3c:p-1:"));
  });

  it("a held task's session is one click away; Processes shows only once that view is there", async () => {
    const { findAllByTestId, findByTestId, queryByTestId, rerender } = render(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW} />);
    fireEvent.click((await findAllByTestId("task-row")).find((r) => r.dataset.status === "in_progress")!);
    expect((await findByTestId("task-holder")).textContent).toContain("In progress with Builder, running now");
    expect(queryByTestId("task-processes")).toBeNull();
    provideProcessesLink(() => ({ label: "Its processes", open: () => undefined }));
    rerender(<PoolTasks poolId="pool-team" sessions={MOCK_SESSIONS} projects={[project]} now={NOW + 1} />);
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

  // The 100 ms budget is a wall-clock one, so it is held where a clock is
  // reliable: `just load-gate` on the reference Mac (AJ_LOAD_GATE=1, the
  // median of five). Everywhere else (CI, a Mac with every core busy: it
  // failed 2 of 2 so, 2026-10-26) this test holds the work instead (each
  // row rendered once, in one commit) and a 500 ms sanity bound that load
  // can't flip.
  it("renders 200 tasks in under 100 ms", async () => {
    seams().__AIJ_TASKS__!.tasks = Array.from({ length: 200 }, (_, n) => task({ id: `aj-${(0x1000 + n).toString(16)}`, title: `Task ${n}` }));
    const gate = (globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env.AJ_LOAD_GATE === "1";
    const runs: number[] = [];
    for (let run = 0; run < (gate ? 5 : 1); run++) {
      rowRenders = 0;
      let commits = 0;
      const started = performance.now();
      const { findAllByTestId, unmount } = render(
        <Profiler id="tasks" onRender={() => (commits += 1)}>
          <SessionTasksList />
        </Profiler>,
      );
      expect((await findAllByTestId("task-row")).length).toBe(200);
      runs.push(performance.now() - started);
      expect(rowRenders).toBe(200);
      expect(commits).toBe(1);
      unmount();
    }
    const median = [...runs].sort((a, b) => a - b)[Math.floor(runs.length / 2)]!;
    expect(median).toBeLessThan(gate ? 100 : 500);
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
/** How many times a row rendered (the render budget's work). */
let rowRenders = 0;
function TaskRowFor({ task: t }: { task: TaskInfo }) {
  rowRenders += 1;
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
    const launch = need({ id: "tneed-2", kind: "launch", title: "«Planner» asks for a new session for «Build the orders API»", detail: "«Use the Work account»" });
    const { getByTestId, findByTestId } = render(<TaskNeeds needs={[launch]} projects={[project]} />);
    const row = getByTestId("task-need-launch");
    expect(row.textContent).toContain("in shop");
    const picker = await findByTestId("task-start-profile");
    await waitFor(() => expect(picker.getAttribute("data-value")).toBe("p-2"));
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
          { id: "ep-1", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-a", task: "Build the orders API" },
          { id: "ep-2", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-b", task: "Build the orders API" },
        ],
        edges: [{ from: "ep-1", to: "ep-2", label: "Full story, fresh start", switch_back: false }],
        collapsed: 0,
        hop_note: null,
      },
    ];
    const { findAllByTestId } = render(<SwitchGraph projectId="p-shop" onOpenSession={() => undefined} />);
    const tasks = await findAllByTestId("node-task");
    expect(tasks.map((t) => t.textContent)).toEqual(["Build the orders API", "Build the orders API"]);
    delete (window as unknown as { __AIJ_SWITCH_GRAPH__?: unknown }).__AIJ_SWITCH_GRAPH__;
  });

  it("a pool always has its Tasks tab; a session only once it has a task", () => {
    expect(pageTabs("pool").map((t) => t.label)).toEqual(["Members", "Tasks", "Messages", "Agent access"]);
    expect(pageTabs("session").some((t) => t.id === "tasks")).toBe(false);
    expect(pageTabs("session", { hasTasks: true }).some((t) => t.id === "tasks")).toBe(true);
  });
});

describe("the app reads tasks; agents write them (D-44)", () => {
  it("the IPC module offers no way to create, edit, assign or link a task", () => {
    const names = Object.keys(tasksIpc);
    expect(names.filter((name) => /^tasks(Create|Edit|Assign|Link)$/.test(name))).toEqual([]);
    for (const command of ["tasks_create", "tasks_edit", "tasks_assign", "tasks_link"]) {
      expect(TasksIpcSource.includes(`"${command}"`)).toBe(false);
    }
    expect(names).toEqual(expect.arrayContaining(["tasksApprove", "tasksSendBack", "tasksFree", "tasksCancel", "tasksStart"]));
  });
});

describe("readability", () => {
  it("no text smaller than 12 px in the task views", () => {
    const files: Record<string, string> = { PoolTasksSource, TaskViewSource, TaskRowSource, TaskTreeSource, TaskLinksSource, StartSessionSource, TaskNeedsSource, PillSource };
    for (const [file, text] of Object.entries(files)) {
      const small = (text.match(/text-\[(\d+(?:\.\d+)?)px\]/g) ?? []).filter((cls: string) => Number(cls.slice(6, -3)) < 12);
      expect(small, file).toEqual([]);
    }
  });
});
