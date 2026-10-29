/**
 * 0.3 vitest — the redesign's doors (§1, §3, §4).
 *
 * - The header: exactly Home · Projects · Pools · Tasks · Accounts ·
 *   Usage · History; the lit tab follows the page (a project's page under
 *   Projects, a pool's under Pools, an account's under Accounts).
 * - Their keys (⌘⇧P, ⌘⇧O, ⌘⇧T) are on the ? sheet and in ⌘K, and clash
 *   with none of D-50's (⌘N, ⌘⇧N, ⌥⌘N) or any other.
 * - Projects: every project, most recently worked first; search by name
 *   or folder; what needs you, its sessions, its tasks and this week's
 *   tokens; a page at a time; a row opens its page; New project.
 * - Tasks: the board's four columns with their counts, across pools;
 *   a card opens the task beside the board with the pool's own actions
 *   (no authoring: D-44); the holder filter; the plan by pool; a card's
 *   project opens its page.
 * - Pools (the placeholder): every pool, each opening its page.
 * - A project's page: its seven tabs; its pool in brief with "Open the
 *   pool ›"; its subagents' mount point.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ActionHostProvider } from "../components/ItemMenu";
import type { ActionHost } from "../lib/action-host";
import { MOCK_PROJECTS, MOCK_SESSIONS, type ProjectInfo } from "../lib/bridge";
import { PRIMARY_NAV, PAGE_UNDER } from "../lib/nav";
import { navLit } from "../components/MainNav";
import { SHORTCUTS } from "../lib/shortcuts";
import { appCommands } from "../lib/palette-commands";
import { forgetPaged } from "../lib/paged";
import { resetPools } from "../lib/pools";
import { resetPreviewTasks, type TaskInfo, type TasksSeed } from "../lib/ipc/tasks";
import { needChips, openWords, taskWords } from "../lib/project-summary";
import { pageTabs } from "../pages/registry";
import { ProjectsScreen, projectsShown, PROJECTS_PAGE } from "./projects/ProjectsScreen";
import { TasksScreen } from "./tasks/TasksScreen";
import { PoolsScreen } from "./pools/PoolsScreen";
import { ProjectPool } from "../pages/project/ProjectPool";
import { ProjectSubagents } from "../pages/project/ProjectSubagents";

type Seams = { __AIJ_TASKS__?: TasksSeed; __AIJ_TASKS_CALLS__?: string[] };
const seams = () => window as unknown as Seams;
const NOW = 1_792_600_000_000;
const SHOP = MOCK_PROJECTS[0]!;
const PAY = MOCK_PROJECTS[1]!;

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
  items: 0,
  items_done: 0,
  written_by: "Planner",
  by_agent: true,
  updated_at_ms: NOW - 60_000,
  issue_label: null,
  ...over,
});

function host(): ActionHost & { calls: string[] } {
  const calls: string[] = [];
  return {
    calls,
    openProjectById: (id) => calls.push(`project:${id}`),
    openPool: (id) => calls.push(`pool:${id}`),
    openSessionById: (id) => calls.push(`session:${id}`),
  };
}

beforeEach(() => {
  resetPreviewTasks();
  resetPools();
  seams().__AIJ_TASKS_CALLS__ = [];
  seams().__AIJ_TASKS__ = {
    tasks: [
      task({ id: "aj-e001", kind: "epic", title: "Checkout v2", items: 2 }),
      task({ id: "aj-s001", kind: "story", title: "Totals", parent_id: "aj-e001", items: 2 }),
      task({ id: "aj-0001", parent_id: "aj-s001" }),
      task({ id: "aj-0002", title: "Fix the cart totals", status: "in_progress", holder: "Builder", holder_session_id: MOCK_SESSIONS[0]!.id, parent_id: "aj-s001" }),
      task({ id: "aj-0003", title: "Checkout copy pass", status: "in_review", holder: "Writer" }),
      task({ id: "aj-0004", title: "Refresh-token retries", pool_id: `pool-default:${PAY.id}`, status: "ready" }),
      task({ id: "aj-0005", title: "Rotate signing keys", pool_id: `pool-default:${PAY.id}`, status: "blocked", stored_status: "ready", blocked_by: ["aj-0004"] }),
    ],
  };
});
afterEach(() => (cleanup(), forgetPaged(), localStorage.clear()));

describe("the header's doors", () => {
  it("are exactly Home · Projects · Pools · Tasks · Accounts · Usage · History", () => {
    expect(PRIMARY_NAV.map((item) => item.label)).toEqual(["Home", "Projects", "Pools", "Tasks", "Accounts", "Usage", "History"]);
  });

  it("light the tab a page sits under: the lit tab is the title", () => {
    expect(navLit("projects", null, "project")).toBe(true);
    expect(navLit("pools", null, "pool")).toBe(true);
    expect(navLit("accounts", null, "account")).toBe(true);
    expect(navLit("home", null, "none")).toBe(true);
    expect(navLit("home", null, "session")).toBe(false);
    expect(navLit("tasks", "tasks", "none")).toBe(true);
    expect(PAGE_UNDER.session).toBeUndefined();
  });

  it("each has a key on the ? sheet and in ⌘K, and no two shortcuts share one", () => {
    const keys = (id: string) => SHORTCUTS.find((entry) => entry.id === id)?.keys[0];
    expect([keys("go-projects"), keys("go-pools"), keys("go-tasks")]).toEqual(["⌘⇧P", "⌘⇧O", "⌘⇧T"]);
    const app = SHORTCUTS.filter((entry) => entry.owner === "app").flatMap((entry) => entry.keys);
    expect(new Set(app).size, "every app shortcut has its own keys").toBe(app.length);
    const commands = appCommands({ openScreen: () => undefined, newSession: () => undefined, newProject: () => undefined, notifications: () => undefined, shortcuts: () => undefined, toggleRail: () => undefined, searchHistory: () => undefined, fleet: false });
    const go = (label: string) => commands.find((command) => command.label === label);
    expect(go("Go to Projects")?.keys).toBe("⌘⇧P");
    expect(go("Go to Pools")?.keys).toBe("⌘⇧O");
    expect(go("Go to Tasks")?.keys).toBe("⌘⇧T");
  });
});

describe("a project in a few facts", () => {
  it("says its needs, sessions and tasks in plain words", () => {
    const chips = needChips({ count: 3, kinds: new Map([["review", 1], ["asks", 2]]) });
    expect(chips.map((chip) => chip.words)).toEqual(["2 ask you", "1 to review"]);
    expect(openWords({ open: 3, working: 2, apps: 2 })).toBe("2 working · 3 open");
    expect(openWords(undefined)).toBe("Nothing open");
    expect(taskWords({ ready: 4, in_progress: 2, in_review: 1, blocked: 0, todo: 0, done: 9, cancelled: 0 })).toBe("4 ready · 1 in review");
    expect(taskWords({ ready: 0, in_progress: 0, in_review: 0, blocked: 0, todo: 0, done: 2, cancelled: 0 })).toBe("No open tasks");
  });
});

describe("Projects", () => {
  it("lists every project, the most recently worked first, with its facts; a row opens its page", async () => {
    const actions = host();
    const view = render(
      <ActionHostProvider host={actions}>
        <ProjectsScreen projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} loading={false} now={NOW} onNewProject={() => undefined} />
      </ActionHostProvider>,
    );
    const rows = view.getAllByTestId("project-row");
    expect(rows.map((row) => row.dataset.projectId)).toEqual(projectsShown(MOCK_PROJECTS, "").map((project) => project.id));
    const shop = rows.find((row) => row.dataset.projectId === SHOP.id)!;
    // Its tasks, read for the page in one call: 1 ready, 1 in review.
    await waitFor(() => expect(within(shop).getByTestId("project-row-tasks").textContent).toBe("1 ready · 1 in review"));
    fireEvent.click(shop);
    expect(actions.calls).toEqual([`project:${SHOP.id}`]);
  });

  it("searches by name or folder, says when nothing matches, and offers New project", () => {
    const newProject = vi.fn();
    const view = render(<ProjectsScreen projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} loading={false} now={NOW} onNewProject={newProject} />);
    fireEvent.change(view.getByTestId("projects-search"), { target: { value: "payments" } });
    expect(view.getAllByTestId("project-row").map((row) => row.dataset.projectId)).toEqual([PAY.id]);
    fireEvent.change(view.getByTestId("projects-search"), { target: { value: "zzz-nothing" } });
    expect(view.getByTestId("projects-empty").textContent).toContain("No project matches");
    fireEvent.click(view.getByTestId("projects-new"));
    expect(newProject).toHaveBeenCalledOnce();
  });

  it("shows a page at a time, and skeleton rows while the projects are read", () => {
    const many: ProjectInfo[] = Array.from({ length: PROJECTS_PAGE + 5 }, (_, n) => ({ ...SHOP, id: `p-${n}`, name: `project ${n}`, last_activity_ms: NOW - n * 1000 }));
    const view = render(<ProjectsScreen projects={many} sessions={[]} loading={false} now={NOW} onNewProject={() => undefined} />);
    expect(view.getAllByTestId("project-row")).toHaveLength(PROJECTS_PAGE);
    fireEvent.click(view.getByTestId("projects-more"));
    expect(view.getAllByTestId("project-row")).toHaveLength(PROJECTS_PAGE + 5);
    cleanup();
    const loading = render(<ProjectsScreen projects={[]} sessions={[]} loading now={NOW} onNewProject={() => undefined} />);
    expect(loading.getByLabelText("Loading projects").getAttribute("aria-busy")).toBe("true");
  });

  it("moves between rows with the arrow keys", () => {
    const view = render(<ProjectsScreen projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} loading={false} now={NOW} onNewProject={() => undefined} />);
    const rows = view.getAllByTestId("project-row");
    rows[0]!.focus();
    fireEvent.keyDown(rows[0]!, { key: "ArrowDown" });
    expect(document.activeElement).toBe(rows[1]);
    fireEvent.keyDown(rows[1]!, { key: "Home" });
    expect(document.activeElement).toBe(rows[0]);
  });
});

describe("Tasks", () => {
  const screen = (actions = host()) =>
    render(
      <ActionHostProvider host={actions}>
        <TasksScreen projects={MOCK_PROJECTS} sessions={MOCK_SESSIONS} now={NOW} />
      </ActionHostProvider>,
    );

  it("shows the board across pools, each column with its count and its epic › story", async () => {
    const view = screen();
    const column = (status: string) => view.getByTestId(`tasks-column-${status}`);
    await waitFor(() => expect(within(column("ready")).getAllByTestId("task-card")).toHaveLength(2));
    expect(within(column("ready")).getByRole("heading").textContent).toBe("Ready2");
    expect(within(column("in_progress")).getAllByTestId("task-card")).toHaveLength(1);
    expect(within(column("in_review")).getByTestId("task-card-review")).toBeTruthy();
    expect(within(column("blocked")).getByTestId("task-card").textContent).toContain("waits on aj-0004");
    const cart = within(column("in_progress")).getByTestId("task-card");
    expect(cart.textContent).toContain("Checkout v2 › Totals");
    // No authoring anywhere on the screen (D-44).
    expect(view.container.textContent).not.toMatch(/New task|Add task|Edit task/);
  });

  it("moves between cards with the arrow keys: down a column, across to the next", async () => {
    const view = screen();
    const ready = await waitFor(() => {
      const cards = within(view.getByTestId("tasks-column-ready")).getAllByTestId("task-card-open");
      expect(cards).toHaveLength(2);
      return cards;
    });
    ready[0]!.focus();
    fireEvent.keyDown(ready[0]!, { key: "ArrowDown" });
    expect(document.activeElement).toBe(ready[1]);
    fireEvent.keyDown(ready[1]!, { key: "ArrowRight" });
    expect(document.activeElement).toBe(within(view.getByTestId("tasks-column-in_progress")).getByTestId("task-card-open"));
  });

  it("opens a card beside the board with the pool's own actions; Esc closes it", async () => {
    const view = screen();
    const review = await waitFor(() => within(view.getByTestId("tasks-column-in_review")).getByTestId("task-card-open"));
    fireEvent.click(review);
    const open = await view.findByTestId("tasks-open");
    await waitFor(() => expect(within(open).getByTestId("task-approve")).toBeTruthy());
    expect(within(open).getByTestId("task-send-back-open")).toBeTruthy();
    fireEvent.click(within(open).getByTestId("task-approve"));
    await waitFor(() => expect(seams().__AIJ_TASKS_CALLS__).toEqual(["approve:aj-0003"]));
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(view.queryByTestId("tasks-open")).toBeNull());
  });

  it("filters by holder from a picker (no system select) and links each card's project", async () => {
    const actions = host();
    const view = screen(actions);
    expect(view.container.querySelector("select")).toBeNull();
    fireEvent.click(view.getByTestId("tasks-filter-holder"));
    fireEvent.click(await view.findByTestId("tasks-filter-holder-nobody"));
    await waitFor(() => expect(within(view.getByTestId("tasks-column-in_progress")).queryAllByTestId("task-card")).toHaveLength(0));
    const ready = within(view.getByTestId("tasks-column-ready")).getAllByTestId("task-card");
    fireEvent.click(within(ready[0]!).getByTestId("task-card-project"));
    expect(actions.calls[0]).toMatch(/^project:/);
  });

  it("shows the plan by pool: each pool's epics and stories", async () => {
    const view = screen();
    fireEvent.click(within(view.getByTestId("tasks-view")).getByRole("radio", { name: "Plan" }));
    const plan = await view.findByTestId("tasks-plan");
    await waitFor(() => expect(within(plan).getAllByTestId("task-row").map((row) => row.dataset.kind)).toContain("epic"));
    expect(localStorage.getItem("aij.tasks.view")).toBe("plan");
  });
});

describe("Pools and a project's page", () => {
  it("Pools lists every pool, each opening its page", async () => {
    const actions = host();
    const view = render(
      <ActionHostProvider host={actions}>
        <PoolsScreen />
      </ActionHostProvider>,
    );
    const rows = await view.findAllByTestId("pools-row");
    fireEvent.click(rows[0]!);
    expect(actions.calls[0]).toMatch(/^pool:/);
  });

  it("a project's page has its seven tabs, Overview first", () => {
    expect(pageTabs("project", { hasUsage: true, counts: { sessions: 3, tasks: 12 } }).map((tab) => [tab.label, tab.count])).toEqual([
      ["Overview", undefined],
      ["Sessions", 3],
      ["Tasks", 12],
      ["Pool", undefined],
      ["Subagents", undefined],
      ["Usage", undefined],
      ["History", undefined],
    ]);
  });

  it("its Pool tab sums its pool up and opens the pool's page", async () => {
    const actions = host();
    const view = render(
      <ActionHostProvider host={actions}>
        <ProjectPool project={SHOP} sessions={MOCK_SESSIONS} now={NOW} />
      </ActionHostProvider>,
    );
    fireEvent.click(await view.findByTestId("project-pool-open"));
    expect(actions.calls).toEqual([`pool:pool-default:${SHOP.id}`]);
  });

  it("its Subagents tab lists its sessions, each opening its own Subagents tab", () => {
    const actions = host();
    const view = render(
      <ActionHostProvider host={actions}>
        <ProjectSubagents project={SHOP} sessions={MOCK_SESSIONS} />
      </ActionHostProvider>,
    );
    const rows = view.getAllByTestId("project-subagents-session");
    fireEvent.click(rows[0]!);
    expect(actions.calls[0]).toMatch(/^session:/);
    expect(localStorage.getItem("aij.page.tab.session")).toBe("subagents");
  });
});
