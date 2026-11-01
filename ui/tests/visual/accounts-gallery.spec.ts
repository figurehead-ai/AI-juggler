/**
 * D-25 gallery: Accounts with runway, Usage (empty and with accounts), and
 * the account-limit decision opened from Home — both themes, axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, expectRail, goTo, honestEmpty, openSwitch, readyHome, terminalDrawn, type FixtureSet } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/68-accounts");
const T0 = 1_728_800_000_000;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, fixture: FixtureSet, theme: "dark" | "light", displays: unknown[] | null = null) {
  await page.addInitScript(
    ({ fixture, theme, t0, displays }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_QUOTA_DISPLAY__ = displays ?? [
        {
          account: "claude-work",
          five_hour: { state: "tracking", percentage: 42, resets_at_ms: t0 + 2 * 3_600_000, burned_tokens: 0 },
          weekly: { state: "tracking", percentage: 18, resets_at_ms: t0 + 4 * 86_400_000, burned_tokens: 0 },
          renewal: { state: "none", label: "", entered_by_user: false },
          forecast: { state: "exhausts", tte_min: 95, range_min: 80, range_max: 110, badge: "computed" },
        },
      ];
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: T0, displays },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  for (const [name, fixture] of [["full", allStatuses], ["signed-out", readyHome]] as const) {
    test(`accounts — ${name} — ${theme}`, gate("68-accounts"), async ({ page }) => {
      await seed(page, fixture, theme);
      await goTo(page, "Accounts");
      await expect(page.getByTestId("account-runway").first()).toBeVisible();
      await page.screenshot({ path: resolve(OUT, `accounts-${name}-1280x800-${theme}.png`), fullPage: true });
      await axeClean(page, `accounts ${name}`);
    });
  }

  for (const [name, fixture] of [["full", allStatuses], ["no-accounts", honestEmpty]] as const) {
    test(`usage — ${name} — ${theme}`, async ({ page }) => {
      await seed(page, fixture, theme);
      await goTo(page, "Usage");
      await expect(page.getByTestId("usage-screen")).toBeVisible();
      if (name === "no-accounts") await expect(page.getByTestId("usage-no-accounts")).toBeVisible();
      else await expect(page.getByTestId("usage-account").first()).toBeVisible();
      await page.screenshot({ path: resolve(OUT, `usage-${name}-1280x800-${theme}.png`) });
      await axeClean(page, `usage ${name}`);
    });
  }

  test(`limit decision — from Home — ${theme}`, async ({ page }) => {
    await seed(page, allStatuses, theme);
    await page.getByRole("button", { name: /^Continue elsewhere/ }).click();
    const dialog = page.getByRole("dialog", { name: /hit its limit/ });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("button", { name: /^Continue on / })).toBeFocused();
    await page.screenshot({ path: resolve(OUT, `limit-decision-1280x800-${theme}.png`) });
    await axeClean(page, "limit decision");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });
}

// D-25: Accounts › Add account opens first run's list (closable).
const FOUND = {
  installs: ["work-b", "client"].map((name, i) => ({
    harness: "claude-code",
    path: `/Users/you/Library/Application Support/ClaudeCodeProfiles/${name}`,
    origin: "manager-tool",
    login: "logged_in",
    bytes: 30_000_000,
    last_used_secs: Math.floor(T0 / 1000) - i * 86_400,
    managed: false,
    notes: [],
    signals: ["manager-tool"],
    inference: {
      purpose: name,
      purpose_confidence: "likely",
      login: "signed-in",
      login_confidence: "found",
      activity: "active",
      activity_confidence: "found",
      activity_words: "Active this week",
      keychain_bound: false,
    },
  })),
  probes: [{ label: "Claude desktop profiles", health: "ok", detail: "2 profiles", reason: null }],
  seeds: [],
  desktop_apps: ["Claude"],
  cancelled: false,
};

for (const theme of ["dark", "light"] as const) {
  test(`add accounts — found on this Mac — ${theme}`, async ({ page }) => {
    await page.addInitScript((scan) => {
      (window as unknown as Record<string, unknown>).__AIJ_SCAN_REPORT__ = scan;
    }, FOUND);
    await seed(page, allStatuses, theme);
    await goTo(page, "Accounts");
    await page.getByTestId("add-account").click();
    // AJ-E1601: a found tool with a service it runs through: Add account
    // asks which tool first.
    const chooser = page.getByRole("dialog", { name: "Add an account: which tool?" });
    await chooser.getByTestId("provider-card").filter({ hasText: "Claude Code" }).getByTestId("provider-choose").click();
    const dialog = page.getByRole("dialog", { name: "Add accounts" });
    await expect(dialog.getByTestId("first-run-bring-in")).toHaveText("Bring in 2 accounts");
    await page.screenshot({ path: resolve(OUT, `add-accounts-1280x800-${theme}.png`) });
    await axeClean(page, "add accounts");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });
}

// AJ-E1301: first run on a fresh install lists every tool's setups — a
// Codex home gets its own group beside the Claude ones (and the "Codex
// isn't installed" note while the CLI is missing). Adding an account for
// one tool lists only that tool (#72).
const FOUND_WITH_CODEX = {
  ...FOUND,
  installs: [
    ...FOUND.installs,
    {
      ...FOUND.installs[0],
      harness: "codex",
      path: "/Users/you/.codex",
      origin: "vendor-default",
      signals: ["vendor-default"],
      inference: { ...FOUND.installs[0].inference, purpose: "Codex" },
    },
  ],
};

for (const theme of ["dark", "light"] as const) {
  test(`first run — Claude and Codex found — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ scan, theme, t0 }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
        w.__AIJ_SCAN_REPORT__ = scan;
        localStorage.setItem("aij.theme", theme);
        Date.now = () => t0;
      },
      { scan: FOUND_WITH_CODEX, theme, t0: T0 },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    const found = page.getByTestId("first-run-found");
    await expect(found.getByTestId("first-run-group-codex")).toBeVisible();
    await expect(found.getByTestId("first-run-no-codex")).toBeVisible();
    await expect(found.getByTestId("first-run-bring-in")).toHaveText("Bring in 3 accounts");
    await page.screenshot({ path: resolve(OUT, `first-run-codex-1280x800-${theme}.png`) });
    await axeClean(page, "first run with Codex");
  });
}

// ADR-0018: an account brought in as a copy (0.0.7 and before) can switch
// to its original folder; each card says where its account lives.
for (const theme of ["dark", "light"] as const) {
  test(`accounts — a copy switches to its original folder — ${theme}`, async ({ page }) => {
    const fixture: FixtureSet = {
      ...allStatuses,
      accounts: allStatuses.accounts.map((account, index) =>
        index === 0
          ? { ...account, import_source_path: "/Users/you/Library/Application Support/ClaudeCodeProfiles/work-a", in_place: false }
          : index === 1
            ? { ...account, import_source_path: "/Users/you/.claude-side", in_place: true }
            : account,
      ),
    };
    await seed(page, fixture, theme);
    await goTo(page, "Accounts");
    const lines = page.getByTestId("account-placement");
    await expect(lines.first()).toContainText("a copy of ~/Library/Application Support/ClaudeCodeProfiles/work-a");
    await expect(lines.nth(1)).toContainText("uses ~/.claude-side where it is");
    await page.screenshot({ path: resolve(OUT, `accounts-placement-1280x800-${theme}.png`) });
    await axeClean(page, "accounts placement");
    await page.getByTestId("account-use-original").click();
    const dialog = page.getByTestId("use-original");
    await expect(dialog).toContainText("nothing is deleted");
    await page.screenshot({ path: resolve(OUT, `use-original-1280x800-${theme}.png`) });
    await axeClean(page, "use the original");
    await page.getByTestId("use-original-confirm").click();
    await expect(dialog).toHaveCount(0);
    await expect(lines.first()).toContainText("uses ~/Library/Application Support/ClaudeCodeProfiles/work-a where it is");
    await expect(page.getByTestId("account-use-original")).toHaveCount(0);
  });
}

// ADR-0018 for Codex: a Codex home can be used where it is too, and a
// Codex copy can switch to its original — worded for Codex, not Claude.
for (const theme of ["dark", "light"] as const) {
  test(`accounts — a Codex copy switches to its original home — ${theme}`, async ({ page }) => {
    const fixture: FixtureSet = {
      ...allStatuses,
      accounts: allStatuses.accounts.map((account, index) =>
        index === 0
          ? { ...account, provider: "codex", import_source_path: "/Users/you/.codex-work", in_place: false }
          : index === 1
            ? { ...account, provider: "codex", import_source_path: "/Users/you/.codex", in_place: true }
            : account,
      ),
    };
    await seed(page, fixture, theme);
    await goTo(page, "Accounts");
    const lines = page.getByTestId("account-placement");
    // AJ-E2109: where the profile lives, under its name.
    await expect(lines.first()).toContainText("a copy of ~/.codex-work");
    await expect(lines.nth(1)).toContainText("uses ~/.codex where it is");
    await page.getByTestId("account-use-original").click();
    const dialog = page.getByTestId("use-original");
    await expect(dialog).toContainText("as when you run Codex yourself");
    await expect(dialog).not.toContainText("Claude");
    await page.screenshot({ path: resolve(OUT, `use-original-codex-1280x800-${theme}.png`) });
    await axeClean(page, "use the original (Codex)");
    await page.getByTestId("use-original-confirm").click();
    await expect(dialog).toHaveCount(0);
    await expect(lines.first()).toContainText("uses ~/.codex-work where it is");
  });
}


// The maintainer's report (0.3.2): a Codex account whose week is used up
// while its pre-bought credits carry the work on reads "Using credits ·
// 14,150 left", never "out of room": on its account card and in a
// session's switch list (beside one with room and one truly out, its
// credits at 0).
const CODEX: FixtureSet = {
  ...allStatuses,
  accounts: [...allStatuses.accounts, { id: "codex-work", provider: "codex", label: "Codex", plan: "pro", login_state: "logged_in" }],
  profiles: [...allStatuses.profiles, { id: "p-codex", name: "Main", account_id: "codex-work", account_label: "Codex", provider: "codex", model: null, default_cwd: null }],
} as FixtureSet;
const week = (left: number) => ({
  state: left === 0 ? "exhausted" : "tracking",
  percentage: 100 - left,
  resets_at_ms: T0 + 2 * 86_400_000 + 5 * 3_600_000,
  reset_basis: "measured",
  measured_at_ms: T0 - 10 * 60_000,
  badge: "measured",
  burned_tokens: 0,
});
const ROOM_DISPLAYS = [
  {
    account: "codex-work",
    five_hour: null,
    weekly: week(0),
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "measured" },
    unavailable_reason: null,
    credits: { has_credits: true, unlimited: false, balance: "14150.1635025000", as_of_ms: T0 - 12 * 60_000, source: "Codex" },
  },
  {
    account: "claude-personal",
    five_hour: { state: "tracking", percentage: 30, resets_at_ms: T0 + 2 * 3_600_000, badge: "measured", burned_tokens: 0 },
    weekly: { state: "tracking", percentage: 40, resets_at_ms: T0 + 4 * 86_400_000, badge: "measured", burned_tokens: 0 },
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "measured" },
    unavailable_reason: null,
    credits: null,
  },
  {
    account: "fake-desk",
    five_hour: null,
    weekly: week(0),
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "measured" },
    unavailable_reason: null,
    credits: { has_credits: true, unlimited: false, balance: "0", as_of_ms: T0 - 12 * 60_000, source: "Fake Desk" },
  },
];
const switchTarget = (account_id: string, account_label: string, provider: string, top: string[]) => ({
  recipe_id: `r-${account_id}`,
  provider,
  account_id,
  account_label,
  badge: provider === "claude-code" ? "Continues exactly · Proven" : "Full story, fresh start · Beta",
  validated: provider === "claude-code",
  cross_provider: provider !== "claude-code",
  drops: provider === "claude-code" ? [] : ["Subagents' work"],
  score: 0.4,
  filtered_reason: null,
  top_factors: top,
  factors: [],
  confidence: "Measured",
  tte_min: null,
  suggested: false,
});
const SWITCH_TARGETS = [
  switchTarget("codex-work", "Codex", "codex", ["using its credits", "quiet"]),
  switchTarget("claude-personal", "Personal", "claude-code", ["60% left · measured", "quiet"]),
  switchTarget("fake-desk", "Fake Desk", "fake-desktop", ["0% left · measured", "quiet"]),
];

async function seedCredits(page: Page, theme: "dark" | "light") {
  await page.addInitScript((targets) => ((window as unknown as Record<string, unknown>).__AIJ_SWITCH_TARGETS__ = targets), SWITCH_TARGETS);
  await seed(page, CODEX, theme, ROOM_DISPLAYS);
}

for (const theme of ["dark", "light"] as const) {
  test(`accounts — a Codex account using its credits — ${theme}`, gate("68-accounts"), async ({ page }) => {
    await seedCredits(page, theme);
    await goTo(page, "Accounts");
    const card = page.getByTestId("account-card").filter({ hasText: "Codex" });
    await expect(card.getByTestId("runway-credits")).toContainText("Using credits · 14,150 left");
    await card.scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `accounts-credits-1280x800-${theme}.png`) });
    await axeClean(page, "accounts with credits");
  });

  test(`switch list — a Codex account using its credits — ${theme}`, gate("68-accounts"), async ({ page }) => {
    await seedCredits(page, theme);
    await expectRail(page);
    await page.getByTestId("rail").locator('[data-instance-id="sess-2"]').first().click();
    await terminalDrawn(page);
    await openSwitch(page);
    const rows = page.getByTestId("switch-target-rows");
    await expect(rows.locator('[data-room="credits"]')).toContainText("Using credits · 14,150 left");
    await rows.scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `switch-credits-1280x800-${theme}.png`) });
    await axeClean(page, "switch list with credits");
  });
}

// 0.3.4, the maintainer's report: personal-max read "Ready", then the
// switch said it wasn't set up; and Codex was suggested over a Claude
// account with room. Now: the same tool first while one has room; a
// target the switch would refuse is held back, saying why, with a way to
// finish its setup.
const SETUP: FixtureSet = {
  ...CODEX,
  accounts: [...CODEX.accounts, { id: "claude-lab", provider: "claude-code", label: "Lab", plan: "max-5x", login_state: "logged_in" }],
  profiles: [...CODEX.profiles, { id: "p-lab", name: "Main", account_id: "claude-lab", account_label: "Lab", provider: "claude-code", model: null, default_cwd: null }],
} as FixtureSet;
const shown = (account: string, left: number) => ({
  account,
  five_hour: null,
  weekly: week(left),
  renewal: { state: "none", label: "", entered_by_user: false },
  forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "measured" },
  unavailable_reason: null,
  credits: null,
});
const SETUP_WORDS =
  "Claude Code hasn't done its first-run setup in a terminal on this account. Sessions in the Claude app don't do it, and a sign-in change asks again. Start a session on it once here and answer its setup questions, then switch.";
const SETUP_TARGETS = [
  switchTarget("codex-work", "Codex", "codex", ["100% left · measured", "quiet"]),
  switchTarget("claude-lab", "Lab", "claude-code", ["70% left · measured", "quiet"]),
  { ...switchTarget("claude-personal", "Personal", "claude-code", []), score: null, filtered_reason: SETUP_WORDS, setup_needed: true },
];

for (const theme of ["dark", "light"] as const) {
  test(`switch list — same tool first; one not set up held back — ${theme}`, async ({ page }) => {
    await page.addInitScript((targets) => ((window as unknown as Record<string, unknown>).__AIJ_SWITCH_TARGETS__ = targets), SETUP_TARGETS);
    await seed(page, SETUP, theme, [shown("codex-work", 100), shown("claude-lab", 70), shown("claude-personal", 62)]);
    await expectRail(page);
    await page.getByTestId("rail").locator('[data-instance-id="sess-2"]').first().click();
    await terminalDrawn(page);
    await openSwitch(page);
    const rows = page.getByTestId("switch-target-rows");
    await expect(rows.getByTestId("continue-set-up")).toHaveText("Start a session on Personal ›");
    await expect(rows.getByRole("button", { name: /^Lab/ }).first()).toHaveAttribute("aria-pressed", "true");
    await rows.scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `switch-setup-1280x800-${theme}.png`) });
    await axeClean(page, "switch list with one not set up");
  });
}
