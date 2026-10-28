/**
 * AJ-E2108 / AJ-E2109 gallery: a session's page for each kind of session
 * (one another app runs that asks you, a Codex thread still working, one
 * that ended with no transcript here, AI Juggler's own on its Overview,
 * last messages turned off), its Usage tab, and Accounts with one, two
 * and six profiles. Both themes, axe 0 critical/serious on each. The
 * conversation is a 30-turn fixture (tests/visual/fixtures/session-chat),
 * captured at 1280×800, 1680×1050, 2000×1100 and 2000×1100 at 200 %.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, terminalDrawn } from "./fixtures";
import { askingTurns, workingTurns } from "./fixtures/session-chat";

const PAGES = resolve(process.cwd(), "../docs/qa/evidence/_gallery/92-session-pages");
const ACCOUNTS = resolve(process.cwd(), "../docs/qa/evidence/_gallery/93-accounts-profiles");
const T0 = 1_728_800_000_000;

test.beforeAll(() => {
  mkdirSync(PAGES, { recursive: true });
  mkdirSync(ACCOUNTS, { recursive: true });
});

const conversation = (id: string, extra: Record<string, unknown>) => ({
  id,
  tool: "claude-code",
  account_id: "claude-work",
  account_label: "Work Max",
  project_id: "proj-1",
  cwd: "/Users/you/Dev/web-app",
  title: null,
  git_branch: "main",
  started_ms: T0 - 3_600_000,
  last_activity_ms: T0 - 180_000,
  tokens: { input: 52_000, output: 9_400, cache_read: 210_000, cache_write: 4_100 },
  live: false,
  session_id: null,
  places: [],
  pid: null,
  state: null,
  viewed_ms: null,
  background: false,
  inactive: false,
  ...extra,
});

const member = (id: string) => ({ id, folder: `~/Library/Application Support/ClaudeCodeProfiles/${id}`, plan: null, renewal_date: null, fee_usd: null });
const account = (key: string, ids: string[], extra: Record<string, unknown> = {}) => ({
  key,
  tool: "claude-code",
  state: "signed_in",
  email: "o…@work.com",
  organisation: null,
  reason: null,
  profiles: ids.map(member),
  plan: "Max",
  renewal_date: null,
  fee_usd: null,
  differs: [],
  ...extra,
});

const profile = (id: string, label: string, signedIn = true, provider = "claude-code") => ({
  id,
  provider,
  label,
  plan: null,
  login_state: signedIn ? "logged_in" : "logged_out",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: true,
});

const asking = conversation("asks-1", {
  title: "Fix the flaky checkout test",
  live: true,
  state: "asks_you",
  places: [{ app: "Claude", app_id: "claude-app", profile: "Work Max", instance: "Work", pid: 4242, label: "Claude · Work" }],
  pid: 4242,
  viewed_ms: T0 - 120_000,
});
const working = conversation("work-1", {
  tool: "codex",
  account_id: "codex-main",
  account_label: "Codex",
  cwd: "/Users/you/Dev/shop-api",
  // A long title: one line, an ellipsis, all of it on hover.
  title: "Add pagination to the orders endpoint with page and limit, a default of 50, a maximum of 200, and the total for page counts",
  live: true,
  state: "working",
  places: [{ app: "ChatGPT", app_id: "chatgpt", profile: "Codex", instance: null, pid: 5151, label: "ChatGPT" }],
  pid: 5151,
  last_activity_ms: T0 - 60_000,
  // Not counted yet: Usage says so, and its tab stays.
  tokens: null,
});
const ended = conversation("ended-1", { title: "Move the orders API to v2", account_id: "claude-personal", account_label: "Personal", last_activity_ms: T0 - 5 * 3_600_000 });

const PAGE_FIXTURE = {
  ...allStatuses,
  // A second profile on Work's account: it shares Work's runway.
  accounts: [...(allStatuses.accounts as unknown[]), profile("claude-work-2", "Work 2"), profile("codex-main", "Codex", true, "codex")],
  conversations: [asking, working, ended],
  subscriptions: [account("work", ["claude-work", "claude-work-2"], { plan: "Max" }), account("me", ["claude-personal"], { email: "o…@me.com", plan: "Pro" })],
  last_messages: {
    "asks-1": { state: "shown", redacted: 1, turns: askingTurns(T0) },
    "work-1": { state: "shown", redacted: 0, turns: workingTurns(T0) },
    "822e4125-e07d-4866-9954-ec9295efb079": {
      state: "shown",
      redacted: 0,
      turns: [
        { speaker: "you", text: "Read the model from the new store and keep the old one as a fallback." },
        { speaker: "assistant", text: "Done: the read model comes from the store, and the old path stays behind a flag until the migration lands." },
      ],
    },
  },
};

const RUNWAY = (t0: number) => [
  {
    account: "claude-work",
    five_hour: { state: "tracking", percentage: 38, resets_at_ms: t0 + 2 * 3_600_000, burned_tokens: 0, measured_at_ms: t0 - 60_000 },
    weekly: { state: "tracking", percentage: 59, resets_at_ms: t0 + 4 * 86_400_000, burned_tokens: 0, measured_at_ms: t0 - 60_000 },
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: { state: "none" },
  },
];

type Size = { width: number; height: number; tag: string };
const BASE: Size = { width: 1280, height: 800, tag: "1280x800" };
const SIZES: Size[] = [
  BASE,
  { width: 1680, height: 1050, tag: "1680x1050" },
  { width: 2000, height: 1100, tag: "2000x1100" },
  // 2000×1100 at 200 % zoom: the same window in half the CSS pixels.
  { width: 1000, height: 550, tag: "2000x1100-zoom200" },
];

async function seed(page: Page, fixture: unknown, theme: "dark" | "light", settings: Record<string, string> = {}, size: Size = BASE) {
  await page.addInitScript(
    ({ fixture, theme, t0, settings, runway }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_APP_SETTINGS__ = settings;
      w.__AIJ_QUOTA_DISPLAY__ = runway;
      const target = (recipe: string, id: string, label: string) => ({
        recipe_id: recipe, provider: "claude-code", account_id: id, account_label: label, badge: "Same tool · Full", validated: true,
        cross_provider: false, drops: [], score: 0.5, filtered_reason: null, top_factors: [], factors: [], confidence: "Computed", tte_min: null, suggested: false,
      });
      // AJ-E2108: a profile on another account, and one on the same.
      w.__AIJ_SWITCH_TARGETS__ = [target("p-2", "claude-personal", "Personal"), target("p-w2", "claude-work-2", "Work 2")];
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: T0, settings, runway: RUNWAY(T0) },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: size.width, height: size.height });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, dir: string, name: string, theme: string, size: Size = BASE) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(dir, `${name}-${size.tag}-${theme}.png`), fullPage: true });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

/** An outside session's page, from its row in the list (a click opens it
 * here; its ↗ goes to its app). */
async function openConversation(page: Page, title: string) {
  await page.getByTestId("rail").locator("[data-conversation-id]").filter({ hasText: title }).first().click();
  await expect(page.getByTestId("conversation-page")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  for (const size of SIZES) {
    test(`a session another app runs that asks you — ${size.tag} — ${theme}`, size === BASE ? gate("92-session-pages") : {}, async ({ page }) => {
      await seed(page, PAGE_FIXTURE, theme, {}, size);
      await openConversation(page, "Fix the flaky checkout test");
      await expect(page.getByTestId("session-state-chip")).toContainText("Asks you · 3 min");
      await expect(page.getByTestId("session-where-chip")).toContainText("Running in the Claude app · viewed 2 min ago");
      await expect(page.getByTestId("last-turn")).toHaveCount(30);
      await expect(page.locator("[data-testid='last-turn'][data-asks='true']")).toContainText("asks you");
      await expect(page.getByTestId("last-messages-question")).toContainText("rebase onto main first");
      await expect(page.getByTestId("md-code").first()).toBeVisible();
      await expect(page.getByTestId("session-go-to")).toBeVisible();
      await shoot(page, PAGES, "outside-asks", theme, size);
    });

    test(`a Codex thread still working — ${size.tag} — ${theme}`, async ({ page }) => {
      await seed(page, PAGE_FIXTURE, theme, {}, size);
      await openConversation(page, "Add pagination to the orders endpoint with page");
      await expect(page.getByTestId("last-messages-working")).toContainText("Codex is working");
      await expect(page.getByTestId("page-tab-usage")).toBeVisible();
      await shoot(page, PAGES, "outside-working", theme, size);
    });
  }

  test(`a session's steps open to their lines, and earlier messages load — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme);
    await openConversation(page, "Fix the flaky checkout test");
    await page.getByTestId("last-messages-scroll").evaluate((box) => (box.scrollTop = 0));
    await expect(page.getByTestId("last-messages-earlier")).toBeVisible();
    await page.getByTestId("last-turn-steps").first().getByRole("button").click();
    await expect(page.getByTestId("last-turn-step-lines").first()).toContainText('ran "pnpm vitest run checkout --repeat 40"');
    await page.getByTestId("last-messages-earlier").click();
    await expect(page.getByTestId("last-turn")).toHaveCount(32);
    await page.getByTestId("last-messages-scroll").evaluate((box) => (box.scrollTop = 0));
    await shoot(page, PAGES, "outside-steps-open", theme);
  });

  test(`a session page's Usage — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme);
    await openConversation(page, "Fix the flaky checkout test");
    await page.getByTestId("page-tab-usage").click();
    // 0.1.5: its charts (turn by turn, by kind, by model, its share of
    // the limits), with its account's window beside them.
    await expect(page.getByTestId("session-usage-visual")).toBeVisible();
    await expect(page.getByTestId("session-usage-turns")).toBeVisible();
    await expect(page.getByTestId("session-usage-runway")).toBeVisible();
    await shoot(page, PAGES, "usage", theme);
  });

  test(`a session page's Usage, with no counts of its own — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme);
    await openConversation(page, "Add pagination to the orders endpoint with page");
    await page.getByTestId("page-tab-usage").click();
    await expect(page.getByTestId("session-usage-visual")).toBeVisible();
    // The usage history has its turns even when the app's own record
    // has no counts (the preview's history answers for every session).
    await expect(page.getByTestId("session-usage-turns")).toContainText("Tokens per turn");
    await shoot(page, PAGES, "usage-not-measured", theme);
  });

  test(`an ended session with no transcript here — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme);
    // Its project's page lists it (AJ-E1533); its row opens its page.
    await openConversation(page, "Fix the flaky checkout test");
    await page.getByTestId("crumb-project").click();
    await page.getByTestId("page-panel-sessions").locator('[data-testid="page-session-row"][data-outside="true"]').filter({ hasText: "Move the orders API to v2" }).first().click();
    await expect(page.getByTestId("conversation-page")).toBeVisible();
    await expect(page.getByTestId("last-messages-none")).toBeVisible();
    await shoot(page, PAGES, "outside-ended", theme);
  });

  test(`AI Juggler's own session on its Overview — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme);
    await page.getByTestId("rail").getByRole("button", { name: "Working, feat/read-model, ai-juggler · 1 h ago" }).click();
    // Its terminal is its Overview (the conversation), the side column beside it.
    await expect(page.getByTestId("page-tab-terminal")).toHaveText("Overview");
    await page.getByTestId("page-tab-terminal").click();
    await expect(page.getByTestId("own-session-overview")).toBeVisible();
    await terminalDrawn(page);
    await expect(page.getByTestId("where-it-runs")).toContainText("AI Juggler's terminal");
    await expect(page.getByTestId("session-runway")).toBeVisible();
    await expect(page.getByTestId("handover-teaser")).toBeVisible();
    await shoot(page, PAGES, "own-overview", theme);
  });

  test(`last messages turned off — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme, { show_last_messages: "0" });
    await openConversation(page, "Fix the flaky checkout test");
    await expect(page.getByTestId("last-messages-off")).toBeVisible();
    await shoot(page, PAGES, "last-messages-off", theme);
  });

  for (const [name, accounts, subscriptions] of [
    ["one-profile", [profile("w1", "Work")], [account("work", ["w1"])]],
    [
      "two-profiles",
      [profile("w1", "Work"), profile("w2", "Work 2")],
      [account("work", ["w1", "w2"])],
    ],
    [
      "six-profiles",
      [profile("w1", "Work"), profile("p1", "Personal"), profile("w2", "Work 2"), profile("w3", "Client Acme"), profile("p2", "Weekend"), profile("n1", "Fresh", false)],
      [
        account("work", ["w1", "w2", "w3"], { organisation: "Acme", plan: "Team" }),
        account("me", ["p1", "p2"], { email: "o…@me.com" }),
        account("out", ["n1"], { state: "signed_out", email: null, plan: null }),
      ],
    ],
  ] as const) {
    test(`Accounts with ${name.replace("-", " ")} — ${theme}`, name === "one-profile" ? gate("93-accounts-profiles") : {}, async ({ page }) => {
      await seed(page, { accounts, profiles: [], sessions: [], projects: [], conversations: [], subscriptions }, theme);
      await goTo(page, "Accounts");
      await expect(page.getByTestId("account-card").first()).toBeVisible();
      await expect(page.getByTestId("profile-row")).toHaveCount(accounts.length);
      await shoot(page, ACCOUNTS, `accounts-${name}`, theme);
    });
  }
}
