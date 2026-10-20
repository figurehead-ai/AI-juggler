/**
 * D-25 gallery: Accounts with runway, Usage (empty and with accounts), and
 * the account-limit decision opened from Home — both themes, axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, honestEmpty, readyHome, type FixtureSet } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/68-accounts");
const T0 = 1_728_800_000_000;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, fixture: FixtureSet, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_QUOTA_DISPLAY__ = [
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
    { fixture, theme, t0: T0 },
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
    test(`accounts — ${name} — ${theme}`, async ({ page }) => {
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
