/**
 * D-25 journey: a brand-new install on a Mac that already has Claude Code
 * set up — the default `~/.claude` plus five Claude desktop profiles —
 * walked as the user meets it: what's here → bring them in → sign in →
 * computer access → Home, a screenshot per step.
 */
import { expect, test } from "@playwright/test";
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
  await expect(found).toContainText("You already have 6 Claude Code setups on this Mac.");
  await expect(page.getByTestId("first-run-bring-in")).toHaveText("Bring in 6 accounts");
  const looked = page.getByRole("button", { name: "Where we looked" });
  if (await looked.count()) await looked.click();
  await page.screenshot({ path: `${OUT}/where-we-looked-1280x800-dark.png`, fullPage: true });
  await page.getByTestId("first-run-bring-in").click();
  primaryClicks += 1;
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/after-bring-in-1280x800-dark.png` });
  const next = page.getByTestId("first-run-signin");
  await expect(next).toBeVisible();
  await page.getByTestId("first-run-signin-next").click();
  const access = page.getByTestId("first-run-access");
  await expect(access).toBeVisible();
  await page.screenshot({ path: `${OUT}/access-1280x800-dark.png` });
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
});
