/**
 * The 0.4.6 RC walks' layout findings, measured in a real renderer (jsdom
 * has no layout): what a walker saw cut or scrolling, at the window sizes
 * walked. A8 (F-A8-3): a CLI App's Launch settings tab scrolled sideways,
 * its list pulled 20 px past the page's column by a negative margin. A9
 * (F-A9-1): an Account card's name, its masked sign-in, was cut
 * ("w…@exam…") beside a long "Choose its plan, renewal date and monthly
 * fee" chip while the card had room for it on a line of its own.
 * Measurements, no captures: CI runs them in the accessibility gate's job
 * (scripts/a11y_gate.sh), beside its other layout checks.
 */
import { expect, test, type Page } from "@playwright/test";
import { knownPlacesPreview, placesConversations } from "../../src/preview/places";

const T0 = new Date(2026, 10, 1, 15, 0, 0).getTime();

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const RECIPES = [{ id: "rec-main", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null }];

async function seed(page: Page, fixture: Record<string, unknown>, size: { width: number; height: number }) {
  await page.addInitScript(
    ({ fixture, known, recipes, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PLACES__ = known;
      w.__AIJ_RECIPES__ = recipes;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, known: knownPlacesPreview(T0), recipes: RECIPES, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize(size);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

/** Every scrolling box on the page whose content is wider than it. */
function sideways(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    [...document.querySelectorAll<HTMLElement>("main *, [data-testid='app-page'] *")]
      .filter((box) => /(auto|scroll)/.test(getComputedStyle(box).overflowX) && box.scrollWidth > box.clientWidth + 1)
      .map((box) => `${box.getAttribute("data-testid") ?? box.getAttribute("role") ?? box.tagName}: ${box.scrollWidth} px in ${box.clientWidth} px`),
  );
}

for (const size of [
  { width: 1280, height: 800 },
  { width: 1024, height: 700 },
]) {
  test(`A8: a CLI App's Launch settings never scroll sideways — ${size.width}`, async ({ page }) => {
    await seed(page, { accounts: ACCOUNTS, projects: [], sessions: [], conversations: placesConversations(T0) }, size);
    // Its page, by the door every name uses (lib/apps' openApp).
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-app", { detail: { id: "cli:claude-work" } })));
    await expect(page.getByTestId("page-name")).toHaveText("Claude Code CLI · Work Max");
    await page.getByRole("tab", { name: /^Launch settings/ }).click();
    await expect(page.getByTestId("profile-launch-settings")).toBeVisible();
    expect(await sideways(page)).toEqual([]);
  });
}

/** A9: one sign-in whose two Apps disagree on plan, renewal and fee (a
 * 0.3 data folder upgraded), named by its masked sign-in; and another. */
const A9_FOLDERS = [...ACCOUNTS, { ...ACCOUNTS[0]!, id: "claude-work-2", label: "Work 2", plan: "pro" }];
const member = (id: string, label: string, plan: string | null, renewal: string | null, fee: number | null) => ({ id, label, folder: `~/.claude-${id}`, plan, renewal_date: renewal, fee_usd: fee });
const base = { name: null, tool: "claude-code", state: "signed_in", organisation: null, reason: null, plan: null, renewal_date: null, fee_usd: null, differs: [], choices: [] };
const options = (work: string, work2: string) => [
  { value: work, folders: ["claude-work"], account: false },
  { value: work2, folders: ["claude-work-2"], account: false },
];
const A9_SUBSCRIPTIONS = [
  { ...base, key: "k-personal", account_id: "acct:anthropic:me", email: "p…@example.invalid", plan: "pro", fee_usd: 20, profiles: [member("claude-personal", "Personal", "pro", null, 20)] },
  {
    ...base,
    key: "k-work",
    account_id: "acct:anthropic:work",
    email: "w…@example.invalid",
    profiles: [member("claude-work", "Work Max", "max-20x", "2026-11-12", 200), member("claude-work-2", "Work 2", "pro", "2026-12-03", 100)],
    differs: ["plan", "renewal", "fee"],
    choices: [
      { term: "plan", options: options("max-20x", "pro") },
      { term: "renewal", options: options("2026-11-12", "2026-12-03") },
      { term: "fee", options: options("200", "100") },
    ],
  },
];

for (const size of [
  { width: 1701, height: 1000 },
  { width: 1280, height: 800 },
  { width: 1024, height: 700 },
]) {
  test(`A9: an Account card's name is whole while the card has room for it — ${size.width}`, async ({ page }) => {
    await seed(page, { accounts: A9_FOLDERS, projects: [], sessions: [], conversations: [], subscriptions: A9_SUBSCRIPTIONS }, size);
    await page.getByRole("link", { name: "Accounts" }).or(page.getByRole("button", { name: "Accounts", exact: true })).first().click();
    const work = page.getByTestId("account-card").filter({ hasText: "w…@example.invalid" });
    await expect(work.getByTestId("account-choose")).toHaveText(/Choose its plan, renewal date and monthly fee/);
    await expect(work.getByTestId("account-profiles-count")).toHaveText(/^\d apps share this account$/);
    // The title and the button in it (the button is what truncates).
    const cut = await page.locator("[data-testid='account-name'], [data-testid='account-name'] button").evaluateAll((names) =>
      names.filter((name) => name.scrollWidth > name.clientWidth + 1).map((name) => `${name.textContent}: ${name.scrollWidth} px in ${name.clientWidth} px`),
    );
    expect(cut).toEqual([]);
    expect(await sideways(page)).toEqual([]);
  });
}
