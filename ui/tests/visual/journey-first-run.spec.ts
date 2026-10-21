/**
 * D-25 journey: a brand-new install on a Mac that already has Claude Code
 * set up — the default `~/.claude` plus five Claude desktop profiles —
 * walked as the user meets it: what's here → bring them in → sign in →
 * computer access → Home, a screenshot per step.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { resolve } from "node:path";
// The journey's screenshots are evidence: the gallery's 66-first-run.
const OUT = process.env.AIJ_JOURNEY_OUT ?? resolve(process.cwd(), "../docs/qa/evidence/_gallery/66-first-run");
const now = () => Math.floor(Date.now() / 1000);
const install = (path: string, origin: string, purpose: string, days: number) => ({
  harness: "claude-code",
  path,
  origin,
  login: "logged_in",
  bytes: 30_000_000,
  last_used_secs: now() - days * 86_400,
  managed: false,
  notes: [],
  inference: {
    purpose,
    purpose_confidence: "likely",
    login: "signed-in",
    login_confidence: "found",
    activity: days < 7 ? "active" : "dormant",
    activity_confidence: "found",
    activity_words: days < 7 ? "Active this week" : `Dormant ${days} d`,
    keychain_bound: false,
  },
  signals: [origin],
});
const PROFILES = "/Users/you/Library/Application Support/ClaudeCodeProfiles";
const SCAN = {
  installs: [
    install("/Users/you/.claude", "vendor-default", "Main", 2),
    install(`${PROFILES}/work-a`, "manager-tool", "Work a", 0),
    install(`${PROFILES}/work-b`, "manager-tool", "Work b", 0),
    install(`${PROFILES}/work-c`, "manager-tool", "Work c", 1),
    install(`${PROFILES}/client`, "manager-tool", "Client", 3),
    install(`${PROFILES}/personal-max`, "manager-tool", "Personal", 0),
  ],
  probes: [
    { label: "Your standard setup (~/.claude)", health: "ok", detail: "1 found", reason: null },
    { label: "Claude desktop profiles", health: "ok", detail: "5 profiles", reason: null },
    { label: "Other folders named .claude-*", health: "ok", detail: "none", reason: null },
  ],
  seeds: [],
  desktop_apps: ["Claude"],
  cancelled: false,
};

/** axe: no critical or serious violation on this step (AJ-E1013 gate). */
async function axeClean(page: Page, step: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${step}`).toEqual([]);
}

test("first run on a Mac with Claude Code already set up", async ({ page }) => {
  await page.addInitScript((scan) => {
    const w = window as unknown as Record<string, unknown>;
    w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
    w.__AIJ_SCAN_REPORT__ = scan;
    // Anything typed into a field (the budget allows none).
    w.__AIJ_TYPED__ = 0;
    document.addEventListener("input", (event) => {
      const target = event.target as HTMLInputElement | null;
      if (target && (target.type === "text" || target.tagName === "TEXTAREA")) {
        (window as unknown as { __AIJ_TYPED__: number }).__AIJ_TYPED__ += 1;
      }
    }, true);
  }, SCAN);
  // AJ-E305's first-install budget, kept for the new first run: at most
  // five primary clicks and nothing typed, from opening to Home.
  let primaryClicks = 0;
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  const found = page.getByTestId("first-run-found");
  await expect(found).toBeVisible();
  await page.screenshot({ path: `${OUT}/found-1280x800-dark.png` });
  await axeClean(page, "what's here");
  await expect(found).toContainText("You already have 6 Claude Code setups on this Mac.");
  await expect(page.getByTestId("first-run-bring-in")).toHaveText("Bring in 6 accounts");
  const looked = page.getByRole("button", { name: "Where we looked" });
  if (await looked.count()) await looked.click();
  await page.screenshot({ path: `${OUT}/where-we-looked-1280x800-dark.png`, fullPage: true });
  await page.getByTestId("first-run-bring-in").click();
  primaryClicks += 1;
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/after-bring-in-1280x800-dark.png` });
  // ADR-0018: used where they are, signed-in setups stay signed in — no
  // sign-in step.
  await expect(page.getByTestId("first-run-signin")).toHaveCount(0);
  const access = page.getByTestId("first-run-access");
  await expect(access).toBeVisible();
  await page.screenshot({ path: `${OUT}/access-1280x800-dark.png` });
  await axeClean(page, "computer access");
  await page.getByRole("button", { name: /Allow See a live preview/ }).click();
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${OUT}/access-one-allowed-1280x800-dark.png` });
  await page.getByTestId("first-run-start").click();
  primaryClicks += 1;
  await expect(page.getByTestId("first-run")).toHaveCount(0);
  expect(primaryClicks).toBeLessThanOrEqual(5);
  expect(await page.evaluate(() => (window as unknown as { __AIJ_TYPED__: number }).__AIJ_TYPED__)).toBe(0);
  await page.waitForTimeout(600);
  await page.screenshot({ path: `${OUT}/home-after-first-run-1280x800-dark.png` });
  await axeClean(page, "Home");
});

test("first run, choosing separate copies: each copy signs in once", async ({ page }) => {
  await page.addInitScript((scan) => {
    const w = window as unknown as Record<string, unknown>;
    w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
    w.__AIJ_SCAN_REPORT__ = scan;
  }, SCAN);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("first-run-found")).toBeVisible();
  await page.getByRole("radio", { name: "Make separate copies" }).check();
  await expect(page.getByTestId("first-run-found")).toContainText("makes its own copy of each");
  await page.screenshot({ path: `${OUT}/found-copies-1280x800-dark.png` });
  await axeClean(page, "copies chosen");
  await page.getByTestId("first-run-bring-in").click();
  const signin = page.getByTestId("first-run-signin");
  await expect(signin).toBeVisible();
  await page.screenshot({ path: `${OUT}/after-bring-in-copies-1280x800-dark.png` });
  await axeClean(page, "sign-in");
  await page.getByTestId("first-run-signin-next").click();
  await expect(page.getByTestId("first-run-access")).toBeVisible();
});

// ADR-0018 §5: one setup that can't come in never blocks the rest — the
// user decides (Continue, Try again, or use it where it is instead).
for (const theme of ["dark", "light"] as const) {
  test(`first run — one copy fails, the user decides — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ scan, theme }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
        w.__AIJ_SCAN_REPORT__ = scan;
        w.__AIJ_IMPORT_FAIL_PATHS__ = [scan.installs[0].path];
        localStorage.setItem("aij.theme", theme);
      },
      { scan: SCAN, theme },
    );
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("radio", { name: "Make separate copies" }).check();
    await page.getByTestId("first-run-bring-in").click();
    const outcome = page.getByTestId("first-run-outcome");
    await expect(outcome).toContainText("5 brought in. 1 couldn't come in.");
    await expect(outcome.getByRole("button", { name: "Use it where it is instead" })).toBeVisible();
    await expect(outcome.getByRole("button", { name: "Continue with 5 accounts" })).toBeVisible();
    await page.screenshot({ path: `${OUT}/one-failed-1280x800-${theme}.png`, fullPage: true });
    await axeClean(page, `one failed (${theme})`);
  });
}


// AJ-E1602: OpenCode beside Claude Code on a fresh install — its own
// group with its install state — and Add account's tool chooser.
const WITH_OPENCODE = {
  ...SCAN,
  installs: [
    install("/Users/you/.claude", "vendor-default", "Main", 2),
    { ...install("/Users/you/.local/share/opencode", "vendor-default", "Main", 0), harness: "opencode" },
  ],
};
const INSTALLS = [
  { provider: "claude-code", display_name: "Claude Code", install_url: "https://claude.com/download", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [] },
  { provider: "opencode", display_name: "OpenCode", install_url: "https://opencode.ai/download", status: "unverified", version: "1.19.3", binary: "/opt/homebrew/bin/opencode", notes: [], verified_against: [] },
];
for (const theme of ["dark", "light"] as const) {
  test(`OpenCode in first run — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ scan, installs, theme }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = { accounts: [], profiles: [], sessions: [], projects: [] };
        w.__AIJ_SCAN_REPORT__ = scan;
        w.__AIJ_PROVIDER_INSTALLS__ = installs;
        localStorage.setItem("aij.theme", theme);
      },
      { scan: WITH_OPENCODE, installs: INSTALLS, theme },
    );
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    const group = page.getByTestId("first-run-group-opencode");
    await expect(group).toContainText("OpenCode 1.19.3 installed");
    await expect(group).toContainText("~/.local/share/opencode");
    await page.screenshot({ path: `${OUT}/opencode-found-1280x800-${theme}.png`, fullPage: true });
    await axeClean(page, `OpenCode found (${theme})`);
  });
}
