/**
 * E19 gallery: tasks shared by a pool (AJ-E1903, E1904, E1906).
 *
 * - A pool's Tasks tab: ready work first, the counts per state, the open
 *   task in full (what it asks, when it's done, what it blocks, the issue
 *   it comes from, the messages that name it, its history) with an
 *   agent's request for a session waiting on you.
 * - The plan: the epic, its stories and their tasks behind folds.
 * - A task someone has: its session one click away (and its Processes
 *   tab, D-41), the sessions that worked it; "Start a session for it"
 *   with the launch settings picker.
 * - D-44: a task in review, its holder's session ended: Approve, Send
 *   back (its note open) and Free. No New task, no fields to change.
 * - Home's Needs you: an agent's held change (the approval card) and an
 *   agent's request for a session (you pick, then Start).
 * - A session's Tasks tab; the switch history with each session's task;
 *   the pool's member card with its task.
 *
 * At 1280, 1680 and 2000 px wide, both themes; axe 0 critical/serious.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { groupBy, pick, projectLens } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/95-tasks");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

/** The gate's one clock (axe-gallery.ts). */
const T0 = GATE_NOW;
const MIN = 60_000;
const POOL = "pool-checkout";

const POOLS = {
  pools: [{ id: POOL, name: "Checkout redesign" }],
  members: [
    { session_id: "sess-live-1", pool_id: POOL, role: "lead" },
    { session_id: "sess-done-1", pool_id: POOL, role: "member" },
    { session_id: "sess-done-2", pool_id: POOL, role: "member" },
  ],
};

type Row = [id: string, kind: string, parent: string | null, title: string, status: string, priority: string, holder: string | null, session: string | null, by: string, ago: number, extra?: Record<string, unknown>];
const ROWS: Row[] = [
  ["aj-e7a1", "epic", null, "Checkout redesign", "in_progress", "normal", null, null, "You", 30, { items: 3, items_done: 0 }],
  ["aj-5c01", "story", "aj-e7a1", "Address step", "in_progress", "normal", null, null, "Planner", 50, { items: 2, items_done: 1 }],
  ["aj-5c02", "story", "aj-e7a1", "Shipping options", "in_review", "normal", null, null, "Planner", 70, { items: 1, items_done: 0 }],
  ["aj-5c03", "story", "aj-e7a1", "Payment step", "ready", "high", null, null, "Planner", 90, { items: 3, items_done: 0 }],
  ["aj-7f3c", "task", "aj-5c03", "Orders API: create and fetch an order", "ready", "high", null, null, "Planner", 12, { issue_label: "figurehead/shop#142" }],
  ["aj-2b9d", "task", "aj-5c01", "Validate postcodes on the address step", "ready", "normal", null, null, "Planner", 40],
  ["aj-1a2b", "task", "aj-5c03", "Cart endpoints return an empty list, not 404", "in_progress", "normal", "Planner", "sess-live-1", "You", 6],
  ["aj-9e41", "task", "aj-5c02", "Shipping rates from the carrier's API", "in_review", "normal", "Builder", null, "Planner", 18],
  ["aj-b10c", "task", "aj-5c03", "Totals move to integer cents", "blocked", "normal", null, null, "Planner", 25, { stored_status: "ready", blocked_by: ["aj-7f3c"] }],
  ["aj-3f88", "task", null, "Flaky test in cart.spec on CI", "blocked", "low", "Reviewer", null, "Reviewer", 140],
  ["aj-c4d2", "task", null, "Refunds table stores floats", "todo", "low", null, null, "Builder", 300],
  ["aj-0d01", "task", "aj-5c01", "Address form copy", "done", "normal", "Builder", null, "Planner", 600],
];
const TASKS = ROWS.map(([id, kind, parent, title, status, priority, holder, session, by, ago, extra]) => ({
  id,
  pool_id: POOL,
  kind,
  parent_id: parent,
  title,
  status,
  stored_status: null,
  priority,
  holder,
  holder_session_id: session,
  blocked_by: [],
  items: 0,
  items_done: 0,
  written_by: by,
  by_agent: by !== "You",
  updated_at_ms: T0 - ago * MIN,
  issue_label: null,
  ...extra,
}));

const DETAILS = {
  "aj-7f3c": {
    pool_name: "Checkout redesign",
    body: "POST /orders takes the cart and returns the order with its total; GET /orders/:id returns it. The schema is in the shared doc api-schema.md. Keep totals in integer cents.",
    acceptance: "- POST and GET pass the contract tests\n- a cart with no items is a 422 with a reason\n- the API reference says how",
    links: [{ kind: "blocks", task_id: "aj-b10c", title: "Totals move to integer cents", status: "blocked" }],
    threads: [{ thread_id: "thr-api", subject: "Start on the API", count: 6, at_ms: T0 - 8 * MIN }],
    issue: { url: "https://github.com/figurehead/shop/issues/142", service: "github", label: "figurehead/shop#142", title: "Orders API for the new checkout", state: "open", checked_at_ms: T0 - 20 * MIN, can_look_up: true, look_up_note: null },
    history: [
      { id: 4, at_ms: T0 - 3 * MIN, who: "Planner", by_agent: true, what: "asked you for a new session for it: Builder is on the cart; this needs its own session" },
      { id: 3, at_ms: T0 - 12 * MIN, who: "Planner", by_agent: true, what: "linked aj-7f3c blocks aj-b10c" },
      { id: 2, at_ms: T0 - 20 * MIN, who: "You", by_agent: false, what: "looked up the issue: open · Orders API for the new checkout" },
      { id: 1, at_ms: T0 - 25 * MIN, who: "Planner", by_agent: true, what: "added it" },
    ],
  },
  "aj-9e41": {
    pool_name: "Checkout redesign",
    body: "Shipping options show live rates from the carrier's API for the cart's weight and the address's postcode.",
    acceptance: "- rates for the three carriers we ship with\n- a carrier that times out shows the others",
    history: [
      { id: 3, at_ms: T0 - 18 * MIN, who: "Builder", by_agent: true, what: "moved it: In progress → In review" },
      { id: 2, at_ms: T0 - 75 * MIN, who: "Builder", by_agent: true, what: "took it" },
      { id: 1, at_ms: T0 - 140 * MIN, who: "Planner", by_agent: true, what: "added it" },
    ],
  },
  "aj-1a2b": {
    pool_name: "Checkout redesign",
    body: "An empty cart is a normal state for the mobile client: GET /cart returns { items: [] }, never a 404.",
    acceptance: "- GET /cart on an empty cart is 200 with no items\n- the mobile client's contract test passes",
    sessions: [
      { session_id: "sess-done-1", name: "Builder", how: "claimed", live: false, at_ms: T0 - 90 * MIN },
      { session_id: "sess-live-1", name: "Planner", how: "worked", live: true, at_ms: T0 - 40 * MIN },
    ],
    threads: [{ thread_id: "thr-api", subject: "Start on the API", count: 6, at_ms: T0 - 8 * MIN }],
    history: [
      { id: 7, at_ms: T0 - 6 * MIN, who: "Planner", by_agent: true, what: "moved it: Ready → In progress" },
      { id: 6, at_ms: T0 - 7 * MIN, who: "Planner", by_agent: true, what: "took it" },
      { id: 5, at_ms: T0 - 45 * MIN, who: "Builder", by_agent: true, what: "gave it back: Orders are done; the cart's 404 still needs the mobile team's answer." },
      { id: 4, at_ms: T0 - 90 * MIN, who: "Builder", by_agent: true, what: "took it" },
      { id: 1, at_ms: T0 - 120 * MIN, who: "You", by_agent: false, what: "added it" },
    ],
  },
};

const NEEDS = [
  { id: "tneed-1", kind: "ask", pool_id: POOL, pool_name: "Checkout redesign", task_id: "aj-9e41", task_title: "Shipping rates from the carrier's API", title: "«Planner» wants to give «Shipping rates from the carrier's API» to Reviewer", detail: "Checkout redesign · above what the pool lets agents do", offer_always: true, project_id: null, at_ms: T0 - 4 * MIN },
  { id: "tneed-2", kind: "launch", pool_id: POOL, pool_name: "Checkout redesign", task_id: "aj-7f3c", task_title: "Orders API: create and fetch an order", title: "«Planner» asks for a new session for «Orders API: create and fetch an order»", detail: "«Builder is on the cart; this needs its own session»", offer_always: false, project_id: "proj-payments", at_ms: T0 - 3 * MIN },
];

const GRAPH = [
  {
    title: "Cart endpoints",
    epochs: [
      { id: "ep-1", provider: "claude-code", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-done-1", task: "Cart endpoints return an empty list, not 404" },
      { id: "ep-2", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-live-1", task: "Cart endpoints return an empty list, not 404" },
    ],
    edges: [{ from: "ep-1", to: "ep-2", label: "Continues exactly", switch_back: false }],
    collapsed: 0,
    hop_note: null,
  },
];

const SIZES = [
  [1280, 800],
  [1680, 1050],
  [2000, 1250],
] as const;

async function open(page: Page, theme: "dark" | "light", width: number, height: number, withNeeds = true) {
  // The gate's clock seam (axe-gallery.ts): Date.now() and new Date()
  // both read T0, in the gate and in the evidence gallery alike.
  await page.clock.setFixedTime(T0);
  await page.addInitScript(
    ({ theme, pools, seed, graph }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_POOLS__ = pools;
      w.__AIJ_TASKS__ = seed;
      w.__AIJ_SWITCH_GRAPH__ = graph;
      w.__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, pools: POOLS, seed: { tasks: TASKS, details: DETAILS, needs: withNeeds ? NEEDS : [], sessions: { "sess-live-1": ["aj-1a2b"], "sess-done-1": ["aj-1a2b"] } }, graph: GRAPH },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function poolTab(page: Page, tab: string) {
  await groupBy(page, "Pool");
  await page.getByTestId("place-header").getByRole("button", { name: /^Open Checkout redesign/ }).click();
  await expect(page.getByTestId("pool-page")).toBeVisible();
  // 0.3: the members are the canvas's List; Tasks and Messages open over it.
  if (tab === "Members") await page.getByTestId("pool-mode-list").click();
  else await page.getByTestId(tab === "Tasks" ? "pool-panel-tasks" : "pool-panel-messages").click();
}

async function poolTasks(page: Page) {
  await poolTab(page, "Tasks");
  await expect(page.getByTestId("task-view")).toBeVisible();
  await expect(page.getByTestId("task-acceptance")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  for (const [width, height] of SIZES) {
    const size = `${width}x${height}`;
    test(`a pool's tasks — ${size} ${theme}`, width === 1280 ? gate("95-tasks") : {}, async ({ page }) => {
      await open(page, theme, width, height);
      await poolTasks(page);
      await expect(page.getByTestId("task-row").first()).toHaveAttribute("data-status", "ready");
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `pool-tasks-${size}-${theme}.png`) });
      await axe(page, `pool tasks ${size} ${theme}`);
    });

    test(`the plan — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      await poolTasks(page);
      await page.getByTestId("tasks-view-plan").click();
      const tree = page.getByTestId("task-tree");
      await expect(tree).toBeVisible();
      for (const story of ["Payment step", "Address step"]) {
        await tree.getByRole("button", { name: `Unfold ${story}` }).click();
      }
      await expect(tree.getByTestId("task-row").filter({ hasText: "Orders API" })).toBeVisible();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `pool-plan-${size}-${theme}.png`) });
      await axe(page, `plan ${size} ${theme}`);
    });

    test(`Needs you with tasks — ${size} ${theme}`, async ({ page }) => {
      await open(page, theme, width, height);
      const needs = page.getByTestId("task-needs");
      await expect(needs.getByTestId("ask-card")).toHaveCount(1);
      await expect(needs.getByTestId("ask-option-always")).toHaveCount(1);
      await expect(needs.getByTestId("task-start-profile")).toBeVisible();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `needs-you-${size}-${theme}.png`) });
      await axe(page, `needs you ${size} ${theme}`);
    });
  }

  test(`a task someone has — 1680x1050 ${theme}`, async ({ page }) => {
    await open(page, theme, 1680, 1050, false);
    await poolTasks(page);
    await page.getByTestId("task-row").filter({ hasText: "Cart endpoints" }).click();
    await expect(page.getByTestId("task-holder")).toContainText("In progress with Planner, running now");
    await expect(page.getByTestId("task-sessions")).toContainText("Builder");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `task-held-1680x1050-${theme}.png`) });
    await axe(page, `task held ${theme}`);
  });

  test(`start a session for a task — 1680x1050 ${theme}`, async ({ page }) => {
    await open(page, theme, 1680, 1050, false);
    await poolTasks(page);
    await page.getByTestId("task-start-open").click();
    await expect(page.getByTestId("task-start")).toBeVisible();
    await pick(page, page.getByTestId("task-start-profile"), "Deep work · Work Max");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `task-start-1680x1050-${theme}.png`) });
    await axe(page, `task start ${theme}`);
  });

  test(`a task in review — 1680x1050 ${theme}`, async ({ page }) => {
    await open(page, theme, 1680, 1050, false);
    await poolTasks(page);
    await expect(page.getByTestId("tasks-new")).toHaveCount(0);
    await page.getByTestId("tasks-filter-in_review").click();
    await page.getByTestId("task-row").filter({ hasText: "Shipping rates" }).click();
    await expect(page.getByTestId("task-approve")).toBeVisible();
    await expect(page.getByTestId("task-holder-ended")).toContainText("Builder, whose session has ended");
    await expect(page.getByTestId("task-free")).toBeVisible();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `task-review-1680x1050-${theme}.png`) });
    await axe(page, `task review ${theme}`);
    await page.getByTestId("task-send-back-open").click();
    await page.getByTestId("task-send-back-note").fill("Rates must come from the carrier's API, not the fixed table");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `task-send-back-1680x1050-${theme}.png`) });
    await axe(page, `task send back ${theme}`);
  });

  test(`a session's tasks — 1680x1050 ${theme}`, async ({ page }) => {
    await open(page, theme, 1680, 1050, false);
    await page.locator('[data-instance-id="sess-live-1"]').first().click();
    await page.getByRole("tab", { name: "Tasks" }).click();
    await expect(page.getByTestId("session-tasks").getByTestId("task-row")).toHaveCount(1);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `session-tasks-1680x1050-${theme}.png`) });
    await axe(page, `session tasks ${theme}`);
  });

  test(`the switch history with tasks — 1680x1050 ${theme}`, async ({ page }) => {
    await open(page, theme, 1680, 1050, false);
    await projectLens(page);
    await page.getByTestId("rail").getByTestId("rail-project").filter({ hasText: "ai-juggler" }).first().click();
    await page.getByRole("tablist", { name: "Project tabs" }).getByRole("tab", { name: "Overview" }).click();
    await expect(page.getByTestId("node-task")).toHaveCount(2);
    const card = page.getByTestId("project-switch-graph-card");
    await card.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `switch-graph-tasks-1680x1050-${theme}.png`) });
    await axe(page, `switch graph ${theme}`);
  });

  test(`a member's card with its task — 1680x1050 ${theme}`, async ({ page }) => {
    await open(page, theme, 1680, 1050, false);
    await poolTab(page, "Members");
    await page.getByTestId("pool-card-more").first().click();
    await expect(page.getByTestId("pool-card-detail")).toContainText("Cart endpoints return an empty list, not 404 (in progress)");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `pool-card-task-1680x1050-${theme}.png`) });
    await axe(page, `card task ${theme}`);
  });
}

test("keyboard: a held change answered with 1; a launch is started only by its button", async ({ page }) => {
  await open(page, "dark", 1280, 800);
  const card = page.getByTestId("task-needs").getByTestId("ask-card");
  await card.focus();
  await page.keyboard.press("1");
  await expect(page.getByTestId("task-needs").getByTestId("ask-card")).toHaveCount(0);
  const start = page.getByTestId("task-needs").getByTestId("task-start-go");
  await expect(start).toBeDisabled();
  await pick(page, page.getByTestId("task-needs").getByTestId("task-start-profile"), "Weekend build · Personal");
  await start.focus();
  await page.keyboard.press("Enter");
  const calls = await page.evaluate(() => (window as unknown as { __AIJ_TASKS_CALLS__?: string[] }).__AIJ_TASKS_CALLS__ ?? []);
  expect(calls).toContain("answer:tneed-1:once");
  expect(calls).toContain("start:aj-7f3c:p-3:tneed-2");
});

test("a held task's Processes link opens its session on the Processes tab (D-41)", async ({ page }) => {
  await open(page, "dark", 1680, 1050, false);
  await poolTasks(page);
  await page.getByTestId("task-row").filter({ hasText: "Cart endpoints" }).click();
  await page.getByTestId("task-processes").click();
  await expect(page.getByRole("tab", { name: "Processes" })).toHaveAttribute("aria-selected", "true");
});
