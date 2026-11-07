/**
 * E31 (AJ-E3103) gallery: the one related list and the one session row.
 * A CLI App with eleven sessions (one that asks, two working, one open,
 * four from this week, three older) opened from the rail's App header,
 * as a user would: its Sessions tab past 8 rows (search, filter chips,
 * its sections), its App filter's menu, a filter that leaves nothing,
 * an App with no session yet (what would appear, and New session), and
 * Home's "Pick up where you left off" in the same row. 1280 and 1024,
 * both themes; axe 0 critical/serious on each. Functional checks ride
 * along: the count equals the rows, the sections come in order, search
 * and the filters narrow.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, scan, test } from "./axe-gallery";
import { allStatuses } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-related-list");
const T0 = new Date(2026, 10, 3, 15, 0, 0).getTime();
const H = 3_600_000;
const D = 24 * H;

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-side", provider: "claude-code", label: "Side project", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROFILES = [
  { id: "p-1", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: "opus", default_cwd: null },
  { id: "p-9", name: "Main", account_id: "claude-side", account_label: "Side project", provider: "claude-code", model: null, default_cwd: null },
];
const PROJECTS = [
  { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

const template = allStatuses.sessions!.find((session) => session.status === "running")!;
const RUNTIME_IDLE = { kind: "terminal", label: null, beta: false, state: "idle", asks: [], can_steer: false, fell_back: null, error: null, limits: [] };

/** One of the App's sessions: `ago` hours since it last did something. */
function session(n: number, status: string, title: string, ago: number, project: string | null, extra: Record<string, unknown> = {}) {
  const live = !["finished", "stopped", "failed", "handed_over"].includes(status);
  return {
    ...template,
    id: `rel-${n}`,
    launch_recipe_id: "p-1",
    account_id: "claude-work",
    account_label: "Work Max",
    status,
    label: title,
    title,
    project_id: project,
    cwd: project === "proj-payments" ? "/Users/you/Dev/payments-api" : "/Users/you/Dev/ai-juggler",
    started_at_ms: T0 - ago * H - H,
    last_activity_ms: T0 - ago * H,
    needs_attention: status === "waiting_user" ? "waiting_user" : null,
    attention: status === "waiting_user" ? { rule: "waiting_prompt", inputs: ["prompt alive"], since_ms: T0 - 12 * 60_000 } : null,
    quota: live ? template.quota : null,
    tokens: live ? template.tokens : null,
    cpu_pct: live ? template.cpu_pct : null,
    rss_bytes: live ? template.rss_bytes : null,
    child_count: live ? template.child_count : 0,
    conversation_id: `conv-rel-${n}`,
    runtime: null,
    ...extra,
  };
}

const SESSIONS = [
  session(1, "waiting_user", "Approve the migration plan", 0.2, "proj-payments"),
  session(2, "running", "Refactor the quota engine", 0.05, "proj-ai-juggler"),
  session(3, "running", "Write the release notes", 0.4, "proj-ai-juggler"),
  session(4, "running", "Review the pools canvas", 1.5, null, { runtime: RUNTIME_IDLE }),
  session(5, "finished", "Fix the login redirect", 5, "proj-payments"),
  session(6, "finished", "Add retries to the webhook", 26, "proj-payments"),
  session(7, "stopped", "Profile the cold start", 50, "proj-ai-juggler"),
  session(8, "finished", "Tidy the settings pane", 120, null),
  session(9, "finished", "Upgrade to React 19", 9 * 24, "proj-ai-juggler"),
  session(10, "finished", "Draft the pricing page", 14 * 24, null),
  session(11, "failed", "Spike the Fleet sync", 21 * 24, "proj-ai-juggler"),
];

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", viewport = { width: 1280, height: 800 }) {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: { accounts: ACCOUNTS, profiles: PROFILES, projects: PROJECTS, sessions: SESSIONS, conversations: [] }, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(viewport);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string, width = 1280) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}${width === 1280 ? "" : `-${width}`}-${theme}.png`) });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), `axe on ${name}`).toEqual([]);
}

/** The App's page from the rail's App header, on its Sessions tab. */
async function openSessions(page: Page, name: string, id = "") {
  const header = page.locator(`[data-testid="place-page"][title="Open ${name}'s page"]`);
  // An App with sessions has its rail header: wait for it, not a count
  // taken before the rail is drawn (it read none on a 1 000-row seed).
  if (!id) await header.first().waitFor();
  if (await header.count()) await header.first().click();
  else {
    // A CLI App with no session has no rail group or Home tile yet: its
    // page opens the way every App name's door does (lib/apps openApp).
    await page.evaluate((id) => window.dispatchEvent(new CustomEvent("aij:open-app", { detail: { id } })), id);
  }
  await expect(page.getByTestId("page-name")).toHaveText(name);
  await page.getByRole("tab", { name: /^Sessions/ }).click();
}

const keys = (page: Page) => page.getByTestId("related-list").getByTestId("page-session-item").evaluateAll((rows) => rows.map((row) => (row as HTMLElement).dataset.key));

for (const theme of ["dark", "light"] as const) {
  for (const width of [1280, 1024] as const) {
    test(`an App's Sessions past 8: search, filters, sections — ${width} ${theme}`, async ({ page }) => {
      await seed(page, theme);
      await openSessions(page, "Claude Code · Work Max");
      // Opened as at 1280, then the window narrowed (the folded rail).
      if (width === 1024) await page.setViewportSize({ width: 1024, height: 700 });
      await expect(page.getByTestId("related-search")).toBeVisible();
      // The tab says how many; the count line only over a search's or a
      // filter's rows (the 0.4 design review, 45).
      await expect(page.getByRole("tab", { name: /^Sessions/ })).toHaveText("Sessions 11");
      await expect(page.getByTestId("page-sessions-count")).toHaveCount(0);
      expect(await page.getByTestId("page-sessions-group").allTextContents()).toEqual(["Needs you", "Working", "Open", "Last 7 days", "Earlier"]);
      expect((await keys(page)).slice(0, 4)).toEqual(["s/rel-1", "s/rel-2", "s/rel-3", "s/rel-4"]);
      await shoot(page, "app-sessions-long", theme, width);
    });
  }

  test(`its Project filter's menu, then Open now — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openSessions(page, "Claude Code · Work Max");
    await page.getByTestId("related-filter-project").click();
    await expect(page.getByTestId("related-filter-project-proj-ai-juggler")).toContainText("ai-juggler · 5");
    await expect(page.getByTestId("related-filter-project-(none)")).toContainText("No project · 3");
    await shoot(page, "filter-menu", theme);
    await page.getByTestId("related-filter-project-proj-ai-juggler").click();
    await page.getByTestId("related-filter-state").click();
    await page.getByTestId("related-filter-state-open").click();
    await expect(page.getByTestId("page-sessions-count")).toHaveText("2 sessions · 2 open");
    await shoot(page, "filtered", theme);
  });

  test(`a search that finds nothing says so — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openSessions(page, "Claude Code · Work Max");
    await page.getByTestId("related-search").fill("kubernetes");
    await expect(page.getByTestId("related-no-match")).toBeVisible();
    await shoot(page, "no-match", theme);
    await page.getByTestId("related-clear").click();
    // Cleared: every row back, and no count line over the whole list.
    await expect(page.getByTestId("page-sessions-count")).toHaveCount(0);
    await expect.poll(async () => (await keys(page)).length).toBe(11);
  });

  test(`an App with no session yet teaches — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openSessions(page, "Claude Code · Side project", "cli:claude-side");
    await expect(page.getByTestId("related-empty")).toContainText("No sessions in Claude Code · Side project yet");
    await expect(page.getByTestId("related-empty").getByRole("button", { name: "New session" })).toBeVisible();
    await shoot(page, "app-empty", theme);
  });

  test(`Home's "Pick up where you left off", the same row — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await expect(page.getByTestId("home-recent")).toBeVisible();
    // The same row; an ended one carries no repeated state word (the 0.4
    // design review, 45).
    await expect(page.getByTestId("home-recent-item").first()).toBeVisible();
    await expect(page.getByTestId("home-recent-item").first().getByTestId("page-session-state")).toHaveCount(0);
    await page.getByTestId("home-recent").scrollIntoViewIfNeeded();
    await shoot(page, "home-recent", theme);
  });
}

/** The budgets (AJ-E3103): the tab opens within 150 ms (from what's in
 * memory: the second open); 1 000 rows scroll at 60 fps (frames timed
 * while the list scrolls its whole length; only the rows in view are
 * drawn). Printed as `[perf]` lines for the evidence. */
test("budgets: a tab from memory ≤ 150 ms, 1 000 rows scroll at 60 fps", async ({ page }) => {
  const many = Array.from({ length: 1_000 }, (_, n) => session(100 + n, "finished", `Ended session ${n + 1}`, 2 + n * 3, n % 3 === 0 ? null : n % 3 === 1 ? "proj-ai-juggler" : "proj-payments"));
  await page.addInitScript(
    ({ fixture, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: { accounts: ACCOUNTS, profiles: PROFILES, projects: PROJECTS, sessions: [...SESSIONS, ...many], conversations: [] }, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await openSessions(page, "Claude Code · Work Max");
  await expect(page.getByRole("tab", { name: /^Sessions/ })).toHaveText("Sessions 1011");
  // Away and back: the second open is from memory.
  await page.getByRole("tab", { name: /^Overview/ }).click();
  const opened = await page.evaluate(async () => {
    const start = performance.now();
    (document.querySelector('[data-testid="page-tab-sessions"]') as HTMLElement | null)?.click();
    for (;;) {
      // The Sessions tab's own rows, drawn and visible (the Overview has rows too).
      const row = document.querySelector<HTMLElement>("#page-panel-sessions [data-testid='related-list'] [data-testid='page-session-item']");
      if (row && row.getClientRects().length > 0) return performance.now() - start;
      await new Promise((done) => requestAnimationFrame(done));
      if (performance.now() - start > 2_000) return -1;
    }
  });
  console.log(`[perf] Sessions tab from memory: ${opened.toFixed(1)} ms`);
  expect(opened).toBeGreaterThan(0);
  expect(opened).toBeLessThanOrEqual(150);
  const frames = await page.evaluate(async () => {
    const list = document.querySelector("[data-testid='page-sessions-list']") as HTMLElement;
    let scroller: HTMLElement | null = list.parentElement;
    while (scroller && !/(auto|scroll)/.test(getComputedStyle(scroller).overflowY)) scroller = scroller.parentElement;
    const box = scroller ?? document.scrollingElement!;
    const times: number[] = [];
    let last = performance.now();
    // About 60 rows a second down the whole list, then back up.
    for (let step = 0; step < 240; step++) {
      box.scrollTop += step < 120 ? 400 : -400;
      await new Promise((done) => requestAnimationFrame(done));
      const now = performance.now();
      times.push(now - last);
      last = now;
    }
    return { times, drawn: document.querySelectorAll("[data-testid='page-session-item']").length, end: box.scrollTop };
  });
  const sorted = [...frames.times].sort((a, b) => a - b);
  const p95 = sorted[Math.floor(sorted.length * 0.95)]!;
  const median = sorted[Math.floor(sorted.length / 2)]!;
  console.log(`[perf] 1 000 rows scrolled: median frame ${median.toFixed(1)} ms, p95 ${p95.toFixed(1)} ms, ${frames.drawn} rows drawn`);
  expect(frames.drawn).toBeLessThan(60);
  expect(median).toBeLessThanOrEqual(17.5);
});
