/**
 * AJ-E2104, AJ-E2105, AJ-E2107 gallery: the rail by place at 1, 8 and 40
 * open sessions (Needs you, Open now's places, the Also line), the View
 * open, and Home empty, typical and busy (heading, Your apps, Needs you,
 * Runway by account). Both themes, axe 0 critical/serious on each.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, gate, scan, test } from "./axe-gallery";
import { placesConversations } from "../../src/preview/places";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/94-places");
const T0 = new Date(2026, 9, 24, 15, 0, 0).getTime();

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROJECTS = [
  { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

/** One of AI Juggler's own sessions. */
function own(id: string, over: Record<string, unknown> = {}) {
  return {
    id, launch_recipe_id: "p-1", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: "opus", launch_model: "opus",
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "feat/read-model", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: T0 - 600_000,
    last_activity_ms: T0 - 60_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  };
}

const SETS = {
  one: { sessions: [], conversations: placesConversations(T0).slice(0, 1) },
  eight: { sessions: [own("sess-1")], conversations: placesConversations(T0) },
  forty: {
    sessions: [own("sess-1"), own("sess-2", { status: "waiting_user", needs_attention: "waiting_user", git_branch: "fix/limits" }), own("sess-3", { status: "quota_blocked", needs_attention: "quota_blocked", git_branch: "chore/deps" })],
    conversations: placesConversations(T0, 30),
  },
  empty: { sessions: [], conversations: [] },
} as const;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, set: keyof typeof SETS, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: { accounts: ACCOUNTS, projects: PROJECTS, ...SETS[set] }, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string, target: "page" | "rail" = "page") {
  await page.mouse.move(0, 0);
  const path = resolve(OUT, `${name}-${theme}.png`);
  if (target === "rail") await page.getByTestId("rail").screenshot({ path });
  else await page.screenshot({ path });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), `axe on ${name}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`the rail with 1 open session — ${theme}`, gate("94-places"), async ({ page }) => {
    await seed(page, "one", theme);
    await expect(page.getByTestId("rail-open-now")).toBeVisible();
    await shoot(page, "rail-1", theme, "rail");
  });

  test(`the rail with 8 open sessions: Needs you, places, Also — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme);
    await expect(page.getByTestId("rail-needs-you")).toContainText("3");
    await expect(page.getByTestId("place-header")).toHaveCount(5);
    await expect(page.getByTestId("rail-also")).toContainText("History");
    // The maintainer's review of 0.1.4: each Needs you row shows its ↗,
    // named for its app and profile (another app runs all three here).
    const needRows = page.getByTestId("place-board").locator("[data-rail-row]").filter({ has: page.getByTestId("need-word") });
    await expect(needRows.getByRole("button", { name: /^Go to it in / })).toHaveCount(3);
    await shoot(page, "rail-8", theme, "rail");
  });

  test(`a row opens its page here; its ↗ goes to its app, saying so — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme);
    const asks = page.locator("[data-rail-row]", { has: page.locator('[data-conversation-id="outside-3"]') });
    await asks.locator("[data-rail-item]").click({ position: { x: 60, y: 10 } });
    await expect(page.getByTestId("conversation-page")).toBeVisible();
    await expect(page.getByRole("status").filter({ hasText: /forward|Going to it/ })).toHaveCount(0);
    await asks.getByRole("button", { name: "Go to it in Claude · Work Max" }).click();
    await expect(page.getByText(/Going to it|forward|Couldn't go to it/).first()).toBeVisible();
    // ⌘↩ on the row does the same; ↩ opens its page.
    await asks.locator("[data-rail-item]").focus();
    await page.keyboard.press("Meta+Enter");
    await expect(page.getByText(/Going to it|forward|Couldn't go to it/).first()).toBeVisible();
  });

  test(`the rail with 40 open sessions, drawn as it scrolls — ${theme}`, async ({ page }) => {
    await seed(page, "forty", theme);
    await expect(page.getByTestId("rail-needs-you")).toBeVisible();
    await shoot(page, "rail-40", theme, "rail");
  });

  test(`the rail's View — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme);
    await page.keyboard.press("Shift+V");
    await expect(page.getByRole("dialog", { name: "View" })).toBeVisible();
    await shoot(page, "view", theme);
  });

  test(`Home, nothing open — ${theme}`, async ({ page }) => {
    await seed(page, "empty", theme);
    await expect(page.getByTestId("home-heading")).toHaveText("Ready when you are");
    await shoot(page, "home-empty", theme);
  });

  test(`Home, typical: what needs you, your apps, runway by account — ${theme}`, async ({ page }) => {
    await seed(page, "eight", theme);
    await expect(page.getByTestId("home-heading")).toHaveText("3 things need you");
    await expect(page.getByTestId("home-app-tile")).toHaveCount(4);
    await shoot(page, "home-typical", theme);
  });

  test(`Home, busy — ${theme}`, async ({ page }) => {
    await seed(page, "forty", theme);
    await expect(page.getByTestId("home-apps-more").or(page.getByTestId("home-app-tile").first())).toBeVisible();
    await shoot(page, "home-busy", theme);
  });
}
