/**
 * 0.4 (D-62, AJ-E3205) gallery: an Account's page and the Accounts
 * screen that leads to it. The Accounts screen's cards named their own
 * way, with no tool letter; the Account opened from its card's name: its
 * Overview at 1280 and 1024 (limits first, its Apps, what's open), its
 * Apps, its Settings. Then AJ-E3202 req. 4: an Account whose two Apps
 * say different renewal dates asks once (the call on its card, the
 * question on its Settings), and an App's Settings show its Account's
 * plan, renewal and fee with "Set on its account". Both themes, axe 0
 * critical/serious on each. rc31 A9: answering one question leaves the
 * next where it was.
 */
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, scan, test } from "./axe-gallery";
import { knownPlacesPreview, placesConversations } from "../../src/preview/places";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-account-page");
const T0 = new Date(2026, 10, 1, 15, 0, 0).getTime();
const H = 3_600_000;

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-side", provider: "claude-code", label: "Side project", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const PROJECTS = [
  { id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
  { id: "proj-payments", root: "/Users/you/Dev/payments-api", name: "payments-api", last_activity_ms: T0, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

/** The places desktop, plus a CLI app with nothing open: two sessions on
 * the Side project folder, ended yesterday and two days ago. */
function conversations() {
  const rows = placesConversations(T0);
  const ended = (id: string, title: string, hours: number) => ({
    ...rows[0]!,
    id,
    title,
    account_id: "claude-side",
    account_label: "Side project",
    live: false,
    places: [],
    pid: null,
    state: null,
    viewed_ms: null,
    last_activity_ms: T0 - hours * H,
  });
  return [...rows, ended("side-1", "Sketch the landing page", 20), ended("side-2", "Try the new chart library", 50)];
}

/** AJ-E3202 req. 4: Work Max and Work 2 on one Max plan, set up apart in
 * 0.3: the plan and fee agree (moved to the account), the renewal dates
 * don't (12 Nov and 3 Dec), so the account asks once. */
const SPLIT_ACCOUNTS = [...ACCOUNTS, { ...ACCOUNTS[0]!, id: "claude-work-2", label: "Work 2", plan: "max-20x" }];
const member = (id: string, label: string, renewal: string | null) => ({ id, label, folder: `~/.claude-${id}`, plan: "max-20x", renewal_date: renewal, fee_usd: 200 });
const base = { name: null, tool: "claude-code", state: "signed_in", organisation: null, reason: null, plan: null, renewal_date: null, fee_usd: null, differs: [], choices: [] };
const SPLIT = [
  {
    ...base,
    key: "k-work",
    account_id: "acct:anthropic:work",
    email: "w…@example.com",
    profiles: [member("claude-work", "Work Max", "2026-11-12"), member("claude-work-2", "Work 2", "2026-12-03")],
    plan: "max-20x",
    fee_usd: 200,
    differs: ["renewal"],
    choices: [{ term: "renewal", options: [{ value: "2026-11-12", folders: ["claude-work"], account: false }, { value: "2026-12-03", folders: ["claude-work-2"], account: false }] }],
  },
  { ...base, key: "k-me", account_id: "acct:anthropic:me", email: "p…@example.com", profiles: [{ ...member("claude-personal", "Personal", null), plan: null, fee_usd: null }] },
  { ...base, key: "k-side", account_id: "acct:anthropic:side", email: "s…@example.com", profiles: [{ ...member("claude-side", "Side project", null), plan: null, fee_usd: null }] },
];

/** The rc31 A9 walk: Work Max and Work 2 disagree on all three (Max 20x
 * or Pro, 12 Nov or 3 Dec, $200 or $100 a month), so three questions. */
const THREE = [
  {
    ...SPLIT[0]!,
    plan: null,
    fee_usd: null,
    profiles: [member("claude-work", "Work Max", "2026-11-12"), { ...member("claude-work-2", "Work 2", "2026-12-03"), plan: "pro", fee_usd: 100 }],
    differs: ["plan", "renewal", "fee"],
    choices: [
      { term: "plan", options: [{ value: "max-20x", folders: ["claude-work"], account: false }, { value: "pro", folders: ["claude-work-2"], account: false }] },
      SPLIT[0]!.choices[0]!,
      { term: "fee", options: [{ value: "200", folders: ["claude-work"], account: false }, { value: "100", folders: ["claude-work-2"], account: false }] },
    ],
  },
  ...SPLIT.slice(1),
];

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", split: boolean | typeof THREE = false) {
  await page.addInitScript(
    ({ fixture, known, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PLACES__ = known;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: { accounts: split ? SPLIT_ACCOUNTS : ACCOUNTS, projects: PROJECTS, sessions: [], conversations: conversations(), ...(split ? { subscriptions: split === true ? SPLIT : split } : {}) }, known: knownPlacesPreview(T0), theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string) {
  await expect(page.getByTestId(name.startsWith("accounts") ? "account-card" : name.startsWith("app-") ? "app-page" : "account-page").first()).toBeVisible();
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${theme}.png`) });
  const bad = (await scan(page)).filter((finding) => finding.impact === "critical" || finding.impact === "serious");
  expect(bad.map((finding) => `${finding.id}: ${finding.targets[0]}`), `axe on ${name}`).toEqual([]);
}

async function openAccounts(page: Page) {
  await page.getByRole("link", { name: "Accounts" }).or(page.getByRole("button", { name: "Accounts", exact: true })).first().click();
  await expect(page.getByTestId("account-card").first()).toBeVisible();
}

async function openWorkAccount(page: Page) {
  await openAccounts(page);
  await page.getByTestId("account-page").first().click();
  await expect(page.getByTestId("account-page")).toHaveAttribute("data-account-id", /acct:/);
}

async function tab(page: Page, label: RegExp) {
  await page.getByRole("tab", { name: label }).click();
}

for (const theme of ["dark", "light"] as const) {
  test(`the Accounts screen: each account named its own way, no tool letter — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openAccounts(page);
    await expect(page.getByTestId("account-tool-mark")).toHaveCount(0);
    await shoot(page, "accounts-screen", theme);
  });

  test(`an Account's Overview: limits first, its Apps, what's open — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openWorkAccount(page);
    await expect(page.getByTestId("account-limits")).toBeVisible();
    await expect(page.getByTestId("account-app").first()).toBeVisible();
    await shoot(page, "account-overview", theme);
    await page.setViewportSize({ width: 1024, height: 700 });
    await shoot(page, "account-overview-1024", theme);
  });

  test(`its Apps and its Settings — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openWorkAccount(page);
    await tab(page, /^Apps/);
    await expect(page.getByTestId("account-apps-tab")).toBeVisible();
    await shoot(page, "account-apps", theme);
    await tab(page, /^Settings/);
    await expect(page.getByTestId("account-name-row")).toBeVisible();
    await shoot(page, "account-settings", theme);
  });

  test(`two Apps said different renewal dates: the account asks once, then has one — ${theme}`, async ({ page }) => {
    await seed(page, theme, true);
    await openAccounts(page);
    // The call to choose on its card opens its Settings.
    await expect(page.getByTestId("account-choose")).toHaveText(/Choose its renewal date/);
    await shoot(page, "accounts-choose", theme);
    await page.getByTestId("account-choose").click();
    const choice = page.getByTestId("account-term-choice");
    await expect(choice).toContainText("Work Max and Work 2 say different renewal dates: 12 Nov or 3 Dec?");
    await shoot(page, "account-choice", theme);
    await choice.getByTestId("account-term-option").filter({ hasText: "3 Dec" }).click();
    // Answered, it stays in place saying so (rc31 A9), until the page closes.
    await expect(choice).toHaveAttribute("data-answered", "true");
    await expect(choice).toContainText("Its renewal date is 3 Dec, for every app on it.");
    await expect(page.getByTestId("account-renewal-input")).toHaveAttribute("data-value", "2026-12-03");
    await shoot(page, "account-chosen", theme);
  });

  test(`answering one question leaves the next where it was, under the pointer (rc31 A9) — ${theme}`, async ({ page }) => {
    await seed(page, theme, THREE);
    await openAccounts(page);
    await page.getByTestId("account-choose").click();
    const question = (term: string) => page.locator(`[data-testid="account-term-choice"][data-term="${term}"]`);
    await expect(question("fee")).toBeVisible();
    const boxes = async () => Promise.all(["renewal", "fee"].flatMap((term) => [0, 1].map((n) => question(term).getByTestId("account-term-option").nth(n).boundingBox())));
    const before = await boxes();
    await question("plan").getByTestId("account-term-option").filter({ hasText: "Max 20x" }).click();
    // Set on the account (its Plan row stops asking), and nothing below moved.
    await expect(page.getByTestId("account-plan-row")).not.toContainText("choose above");
    expect(await boxes()).toEqual(before);
    await expect(question("plan")).toHaveAttribute("data-answered", "true");
    await expect(question("plan")).toContainText("Its plan is Max 20x, for every app on it.");
    // The walker's quick second click lands where it was aimed: $200/month.
    const [, , fee200] = before;
    await page.mouse.click(fee200!.x + fee200!.width / 2, fee200!.y + fee200!.height / 2);
    await expect(question("fee")).toHaveAttribute("data-answered", "true");
    await expect(question("fee")).toContainText("Its monthly fee is $200/month, for every app on it.");
    await expect(question("renewal")).not.toHaveAttribute("data-answered", "true");
  });

  test(`an App's Settings show its account's plan, renewal and fee, set there — ${theme}`, async ({ page }) => {
    await seed(page, theme, true);
    await openAccounts(page);
    await page.getByTestId("account-choose").click();
    await page.getByTestId("account-term-option").filter({ hasText: "3 Dec" }).click();
    await expect(page.getByTestId("account-term-choice")).toHaveAttribute("data-answered", "true");
    await tab(page, /^Apps/);
    await page.getByTestId("account-apps-tab").locator('[data-app-id="cli:claude-work"]').click();
    await expect(page.getByTestId("app-page")).toHaveAttribute("data-app-id", "cli:claude-work");
    await tab(page, /^Settings/);
    await expect(page.getByTestId("app-terms-renewal")).toContainText("3 Dec");
    await expect(page.getByTestId("renewal-editor")).toHaveCount(0);
    await shoot(page, "app-settings-terms", theme);
    // "Set on its account" opens the account's Settings.
    await page.getByTestId("app-terms-open").click();
    await expect(page.getByTestId("account-settings-tab")).toBeVisible();
  });
}
