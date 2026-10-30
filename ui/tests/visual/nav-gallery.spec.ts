/**
 * 0.3 gallery: finding your way (the redesign, §1, §3, §4).
 *
 * - The header: Home · Projects · Pools · Tasks · Accounts · Usage ·
 *   History, the lit tab the page's title, one row at 1280 and at 1024
 *   with the list open.
 * - Projects: every project with what needs you, its sessions, its open
 *   tasks and this week's tokens; a search with nothing found.
 * - A project's page: Overview (the KPI row), Tasks (its pool's tree),
 *   Pool (its pool in brief), Subagents (the mount point), History.
 * - Tasks: the board across pools, a task open beside it (in review:
 *   Approve, Send back…), the plan, the Holder picker open.
 * - Pools (the placeholder list), and a session's header with its pool
 *   and its tool.
 *
 * Both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, openRailList, type FixtureSet } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/97-nav");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

const T0 = GATE_NOW;
const MIN = 60_000;

const FIXTURE: FixtureSet = {
  ...allStatuses,
  projects: [
    ...allStatuses.projects,
    { id: "proj-4", root: "/Users/dev/docs-site", name: "docs-site", last_activity_ms: T0 - 26 * 3_600_000, active_instance: null, accounts_used: ["Personal"], instance_count: 0, history_sessions: 9, history_last_used_ms: null, repo: null, missing: false },
    { id: "proj-5", root: "/Users/dev/api-gateway", name: "api-gateway", last_activity_ms: T0 - 4 * 86_400_000, active_instance: null, accounts_used: ["Work Max"], instance_count: 0, history_sessions: 3, history_last_used_ms: null, repo: null, missing: false },
  ],
};

type Row = [id: string, pool: string, kind: string, parent: string | null, title: string, status: string, holder: string | null, session: string | null, extra?: Record<string, unknown>];
const ROWS: Row[] = [
  ["aj-e001", "pool-default:proj-2", "epic", null, "Checkout v2", "in_progress", null, null, { items: 4, items_done: 1 }],
  ["aj-s001", "pool-default:proj-2", "story", "aj-e001", "Totals", "in_progress", null, null, { items: 2, items_done: 0 }],
  ["aj-s002", "pool-default:proj-2", "story", "aj-e001", "Copy", "in_review", null, null, { items: 1, items_done: 0 }],
  ["aj-0001", "pool-default:proj-2", "task", "aj-s001", "Round tax per line", "ready", null, null],
  ["aj-0002", "pool-default:proj-2", "task", "aj-s001", "Fix the cart totals", "in_progress", "Builder", "sess-3"],
  ["aj-0003", "pool-default:proj-2", "task", "aj-s002", "Checkout copy pass", "in_review", "Writer", null],
  ["aj-0004", "pool-default:proj-4", "task", null, "Screenshots for step 3", "ready", null, null],
  ["aj-e002", "pool-default:proj-5", "epic", null, "Gateway", "in_progress", null, null, { items: 2, items_done: 0 }],
  ["aj-s003", "pool-default:proj-5", "story", "aj-e002", "Auth", "in_progress", null, null, { items: 2, items_done: 0 }],
  ["aj-0005", "pool-default:proj-5", "task", "aj-s003", "Refresh-token retries", "in_progress", "Planner", "sess-2"],
  ["aj-0006", "pool-default:proj-5", "task", "aj-s003", "Rotate signing keys", "blocked", null, null, { stored_status: "ready", blocked_by: ["aj-0005"] }],
  ["aj-0007", "pool-default:proj-1", "task", null, "Split the read model", "ready", null, null, { priority: "high" }],
];
const TASKS = ROWS.map(([id, pool, kind, parent, title, status, holder, session, extra], n) => ({
  id,
  pool_id: pool,
  kind,
  parent_id: parent,
  title,
  status,
  stored_status: null,
  priority: "normal",
  holder,
  holder_session_id: session,
  blocked_by: [],
  items: 0,
  items_done: 0,
  written_by: "Planner",
  by_agent: true,
  updated_at_ms: T0 - (n + 2) * 7 * MIN,
  issue_label: null,
  ...extra,
}));
const DETAILS = {
  "aj-0003": {
    pool_name: "storefront · all sessions",
    body: "Tighten the checkout's words: the button, the totals line and the error for a declined card.",
    acceptance: "- every string under 40 characters\n- the declined-card error says what to do",
    history: [
      { id: 2, at_ms: T0 - 20 * MIN, who: "Writer", by_agent: true, what: "moved it: In progress → In review" },
      { id: 1, at_ms: T0 - 90 * MIN, who: "Planner", by_agent: true, what: "added it" },
    ],
  },
};

async function open(page: Page, theme: "dark" | "light", size = { width: 1280, height: 800 }) {
  await page.clock.setFixedTime(T0);
  await page.addInitScript(
    ({ theme, fixture, tasks, details }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_TASKS__ = { tasks, details };
      w.__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, fixture: FIXTURE, tasks: TASKS, details: DETAILS },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(size);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${theme}.png`) });
  await axe(page, `${name} ${theme}`);
}

const nav = (page: Page, name: string) => page.getByRole("navigation", { name: "Main" }).getByRole("button", { name, exact: true });

/** The header fits one row: every tab and New session share a line. */
async function oneRow(page: Page) {
  const header = page.locator("header").first();
  const tabs = await nav(page, "Home").boundingBox();
  const action = await header.getByRole("button", { name: "New session" }).boundingBox();
  expect(Math.abs((tabs?.y ?? 0) - (action?.y ?? 999)), "the tabs and New session on one line").toBeLessThan(12);
}

async function projectPage(page: Page, tab: string) {
  await nav(page, "Projects").click();
  await page.getByTestId("project-row").filter({ hasText: "storefront" }).click();
  await expect(page.getByTestId("crumb-projects")).toBeVisible();
  await page.getByRole("tab", { name: new RegExp(`^${tab}`) }).click();
}

for (const theme of ["dark", "light"] as const) {
  test(`the header at 1280 — ${theme}`, gate("97-nav"), async ({ page }) => {
    await open(page, theme);
    await expect(nav(page, "Home")).toHaveAttribute("aria-current", "page");
    await oneRow(page);
    await shoot(page, "header-home-1280x800", theme);
  });

  test(`the header at 1024 with the list open — ${theme}`, async ({ page }) => {
    await open(page, theme, { width: 1024, height: 700 });
    await openRailList(page);
    await oneRow(page);
    await shoot(page, "header-home-1024x700", theme);
  });

  test(`Projects — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await nav(page, "Projects").click();
    await expect(page.getByTestId("project-row")).toHaveCount(5);
    await expect(page.getByTestId("project-row").filter({ hasText: "storefront" }).getByTestId("project-row-tasks")).toHaveText("1 ready · 1 in review");
    await shoot(page, "projects-1280x800", theme);
    await page.getByTestId("projects-search").fill("nothing-like-this");
    await expect(page.getByTestId("projects-empty")).toBeVisible();
    await shoot(page, "projects-search-empty-1280x800", theme);
  });

  test(`Projects at 1024 — ${theme}`, async ({ page }) => {
    await open(page, theme, { width: 1024, height: 700 });
    await openRailList(page);
    await nav(page, "Projects").click();
    await expect(page.getByTestId("project-row").first()).toBeVisible();
    await shoot(page, "projects-1024x700", theme);
  });

  for (const tab of ["Overview", "Tasks", "Pool", "Subagents", "History"]) {
    test(`a project's ${tab} — ${theme}`, async ({ page }) => {
      await open(page, theme);
      await projectPage(page, tab);
      await expect(nav(page, "Projects")).toHaveAttribute("aria-current", "page");
      await expect(page.getByTestId(`page-panel-${tab.toLowerCase()}`)).toBeVisible();
      await shoot(page, `project-${tab.toLowerCase()}-1280x800`, theme);
    });
  }

  test(`Tasks: the board — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await nav(page, "Tasks").click();
    await expect(page.getByTestId("tasks-column-ready").getByTestId("task-card")).toHaveCount(3);
    await shoot(page, "tasks-board-1280x800", theme);
  });

  test(`Tasks: a task in review open beside the board — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await nav(page, "Tasks").click();
    await page.getByTestId("tasks-column-in_review").getByTestId("task-card-open").first().click();
    await expect(page.getByTestId("tasks-open").getByTestId("task-approve")).toBeVisible();
    await shoot(page, "tasks-open-review-1280x800", theme);
  });

  test(`Tasks: the plan and the Holder picker — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await nav(page, "Tasks").click();
    await page.getByTestId("tasks-view").getByRole("radio", { name: "Plan" }).click();
    await expect(page.getByTestId("tasks-plan")).toBeVisible();
    await shoot(page, "tasks-plan-1280x800", theme);
    // The plan's epics and stories have no holder: its picker is the board's.
    await expect(page.getByTestId("tasks-filter-holder")).toHaveCount(0);
    await page.getByTestId("tasks-view").getByRole("radio", { name: "Board" }).click();
    await page.getByTestId("tasks-filter-holder").click();
    await expect(page.getByRole("menu", { name: "Holder" })).toBeVisible();
    await shoot(page, "tasks-holder-menu-1280x800", theme);
  });

  test(`Pools and a session's pool chip — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await nav(page, "Pools").click();
    await expect(page.getByTestId("pools-row").first()).toBeVisible();
    await shoot(page, "pools-1280x800", theme);
    await page.locator('[data-instance-id="sess-2"]').first().click();
    await expect(page.getByTestId("session-pool-chip")).toBeVisible();
    await shoot(page, "session-pool-chip-1280x800", theme);
  });
}
