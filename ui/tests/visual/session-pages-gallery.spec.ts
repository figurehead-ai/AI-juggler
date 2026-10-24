/**
 * AJ-E2108 / AJ-E2109 gallery: a session's page for each kind of session
 * (one another app runs that asks you, one that ended with no transcript
 * here, AI Juggler's own on its Overview, last messages turned off), and
 * Accounts with one, two and six profiles. Both themes, axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo } from "./fixtures";

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
  cwd: "/Users/you/Dev/ai-juggler",
  title: null,
  git_branch: "main",
  started_ms: T0 - 3_600_000,
  last_activity_ms: T0 - 180_000,
  tokens: { input: 52_000, output: 9_400, cache_read: 210_000, cache_write: 0 },
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

const profile = (id: string, label: string, signedIn = true) => ({
  id,
  provider: "claude-code",
  label,
  plan: null,
  login_state: signedIn ? "logged_in" : "logged_out",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: true,
});

const asking = conversation("asks-1", {
  title: "Move Settings to panes",
  live: true,
  state: "asks_you",
  places: [{ app: "Claude", app_id: "claude-app", profile: "Work Max", instance: "Work", pid: 4242, label: "Claude · Work" }],
  pid: 4242,
  viewed_ms: T0 - 120_000,
});
const ended = conversation("ended-1", { title: "Add pagination to the orders endpoint", account_id: "claude-personal", account_label: "Personal", last_activity_ms: T0 - 5 * 3_600_000 });

const PAGE_FIXTURE = {
  ...allStatuses,
  // A second profile on Work's account: it shares Work's runway.
  accounts: [...(allStatuses.accounts as unknown[]), profile("claude-work-2", "Work 2")],
  conversations: [asking, ended],
  subscriptions: [account("work", ["claude-work", "claude-work-2"], { plan: "Max" }), account("me", ["claude-personal"], { email: "o…@me.com", plan: "Pro" })],
  last_messages: {
    "asks-1": {
      state: "shown",
      redacted: 1,
      turns: [
        { speaker: "assistant", text: "Seven panes are in, and every setting has a home. Two rows had no footer, so I wrote one for each." },
        { speaker: "you", text: "Good. Keep the old shortcuts working; the test key is •••" },
        { speaker: "assistant", text: "Should the Privacy pane stay last, or go after Notifications as in the design?" },
      ],
    },
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

async function seed(page: Page, fixture: unknown, theme: "dark" | "light", settings: Record<string, string> = {}) {
  await page.addInitScript(
    ({ fixture, theme, t0, settings }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_APP_SETTINGS__ = settings;
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
    { fixture, theme, t0: T0, settings },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, dir: string, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(dir, `${name}-1280x800-${theme}.png`), fullPage: true });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

/** An outside session's page, from Home's list. */
async function openConversation(page: Page, title: string) {
  await page.getByTestId("home-outside").filter({ hasText: title }).first().click();
  await expect(page.getByTestId("conversation-page")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  test(`a session another app runs that asks you — ${theme}`, gate("92-session-pages"), async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme);
    await openConversation(page, "Move Settings to panes");
    await expect(page.getByTestId("session-state-chip")).toContainText("Asks you · 3 min");
    await expect(page.getByTestId("session-where-chip")).toContainText("Running in the Claude app · viewed 2 min ago");
    await expect(page.locator("[data-testid='last-turn'][data-asks='true']")).toContainText("Claude asks");
    await expect(page.getByTestId("session-go-to")).toBeVisible();
    await shoot(page, PAGES, "outside-asks", theme);
  });

  test(`an ended session with no transcript here — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme);
    // Its project's page lists it (AJ-E1533); its row opens its page.
    await page.getByTestId("rail").getByTestId("rail-project").filter({ hasText: "ai-juggler" }).first().click();
    await page.getByTestId("page-panel-sessions").locator('[data-testid="page-session-row"][data-outside="true"]').filter({ hasText: "Add pagination" }).first().click();
    await expect(page.getByTestId("conversation-page")).toBeVisible();
    await expect(page.getByTestId("last-messages-none")).toBeVisible();
    await shoot(page, PAGES, "outside-ended", theme);
  });

  test(`AI Juggler's own session on its Overview — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme);
    await page.getByTestId("rail").getByRole("button", { name: /^feat\/read-model Running/ }).first().click();
    await page.getByTestId("page-tab-overview").click();
    await expect(page.getByTestId("session-overview")).toBeVisible();
    await expect(page.getByTestId("last-turn").first()).toBeVisible();
    await expect(page.getByTestId("continue-target")).toHaveCount(1);
    await expect(page.getByTestId("continue-no-room")).toHaveText("Work 2 shares this account, so it gives no new room.");
    await shoot(page, PAGES, "own-overview", theme);
  });

  test(`last messages turned off — ${theme}`, async ({ page }) => {
    await seed(page, PAGE_FIXTURE, theme, { show_last_messages: "0" });
    await openConversation(page, "Move Settings to panes");
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
