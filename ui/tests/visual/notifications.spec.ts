/**
 * AJ-E1510 gallery: Settings › Notifications — which kinds may become a
 * macOS notification (all off at first), and the quiet hours and Accounts
 * that apply to them as to the digest; and a clicked notification opening
 * its session, or This Mac. The click is the preview's stand-in event (the
 * system's own click is the backend's test). Both themes; axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/84-notifications");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript((theme) => {
    localStorage.setItem("aij.theme", theme);
    // AJ-E1522: a pinned clock, so the capture is the same every run.
    Date.now = () => 1_728_800_000_000;
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
  }, theme);
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function click(page: Page, open: { session_id: string | null; page: string | null }) {
  await page.evaluate((detail) => window.dispatchEvent(new CustomEvent("aij:notification-click", { detail })), open);
}

for (const theme of ["dark", "light"] as const) {
  test(`Settings › Notifications: every kind off until chosen — ${theme}`, gate("84-notifications"), async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Settings");
    await page.getByRole("navigation", { name: "Settings sections" }).getByRole("button", { name: "Notifications" }).click();
    const card = page.getByTestId("settings-notifications");
    await expect(card.getByTestId("system-notifications")).toBeVisible();
    for (const label of [
      "A session waits for you",
      "A session looks stuck",
      "A session hits its usage limit",
      "A session finishes",
      "This Mac is getting tight",
    ]) {
      await expect(card.getByLabel(label)).not.toBeChecked();
    }
    await card.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `settings-off-1280x800-${theme}.png`) });
    await axe(page, `notifications off ${theme}`);

    await card.getByLabel("A session waits for you").check();
    await expect(card.getByRole("status")).toHaveText("You'll get a notification when a session waits for you.");
    await card.getByLabel("This Mac is getting tight").check();
    await expect(card.getByRole("status")).toHaveText("You'll get a notification when This Mac gets tight.");
    expect(await page.evaluate(() => (window as unknown as { __AIJ_NOTIFY__?: unknown }).__AIJ_NOTIFY__)).toEqual({
      waiting: true,
      stuck: false,
      limit: false,
      finished: false,
      this_mac: true,
    });
    await expect(card.getByTestId("quiet-hours")).toContainText("The digest and notifications wait until the quiet hours end");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `settings-chosen-1280x800-${theme}.png`) });
    await axe(page, `notifications chosen ${theme}`);
  });
}

test("a clicked notification opens its session, or This Mac", async ({ page }) => {
  await open(page, "dark");
  await goTo(page, "Settings");
  await click(page, { session_id: "sess-live-1", page: null });
  await expect(page.getByTestId("terminal-toolbar")).toBeVisible();
  await click(page, { session_id: null, page: "this_mac" });
  await expect(page.getByTestId("this-mac-page")).toBeVisible();
});
