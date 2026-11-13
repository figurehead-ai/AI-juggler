/**
 * AJ-E805 (0.4.5, idle CPU): a window nobody touches stops drawing.
 *
 * The 0.4.3 beta idled at about 20 % of a core, and its web view's own
 * processes at as much again: the header's This Mac dot breathed for ever
 * (a box-shadow animation, a repaint every frame), so the window drew a
 * frame every vsync while nothing changed (the lab, window shown: the app
 * 6.0 %, its web content 9.5 % and GPU process 6.6 %; hidden, 2.4 % in
 * all). At rest, with the at-scale fixture (10 Accounts, 50 sessions,
 * running ones among them) and motion allowed, no animation may loop
 * for ever: a loading skeleton pulses only until its answer comes, a
 * live dot breathes a few times when it shows (styles.css `.dot-live`), a
 * terminal's cursor blinks for a while after its last key or output
 * (lib/terminal-caret `blinkWhileUsed`). A session at work may say so
 * while it works: that is something changing.
 */
import { expect, test, type Page } from "@playwright/test";
import { atScale, expectRail } from "./fixtures";

const T0 = 1_728_800_000_000;

async function open(page: Page) {
  await page.addInitScript(
    ({ fixture, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      // Every session's row drawn (the Project lens), as load.spec.ts.
      localStorage.setItem("aij.rail.view.v1", JSON.stringify({ group: "project" }));
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: atScale, t0: T0 },
  );
  // Motion allowed: the reduced-motion rule would hide what loops.
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await expect(page.getByTestId("user-menu-button")).toBeVisible();
}

/** The animations running that never end, each named with what it moves. */
function endless(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    document
      .getAnimations()
      .filter((animation) => animation.playState === "running" && animation.effect?.getComputedTiming().iterations === Infinity)
      .map((animation) => {
        const target = (animation.effect as KeyframeEffect | null)?.target ?? null;
        const name = animation instanceof CSSAnimation ? animation.animationName : "script";
        return `${name} on <${target?.tagName.toLowerCase() ?? "?"} class="${target?.getAttribute("class") ?? ""}">`;
      }),
  );
}

test("Home at rest runs no endless animation", async ({ page }) => {
  await open(page);
  await expectRail(page);
  // What loads by itself settles: every skeleton gives way to its answer.
  await expect.poll(() => endless(page), { timeout: 10_000 }).toEqual([]);
  // The header's This Mac dot breathes a bounded number of times.
  const breaths = await page.evaluate(() =>
    document
      .getAnimations()
      .filter((animation) => animation instanceof CSSAnimation && animation.animationName === "breathe")
      .map((animation) => animation.effect?.getComputedTiming().iterations ?? 0),
  );
  for (const count of breaths) expect(count).toBeLessThanOrEqual(3);
});

test("a live session's page at rest runs no endless animation", async ({ page }) => {
  await open(page);
  await expectRail(page);
  // Live, its terminal open, waiting for you: nothing changes.
  const waiting = atScale.sessions.find((session) => session.status === "waiting_user");
  expect(waiting, "the fixture has a session waiting for you").toBeTruthy();
  // The board settles first (its rows are drawn again once the reads land).
  await expect(page.locator('[data-testid="instance-board"] [data-instance-id]').first()).toBeVisible();
  await page.waitForTimeout(300);
  await page.locator(`[data-instance-id="${waiting!.id}"]`).first().click();
  // The terminal's cursor rests 10 s after its last output.
  await expect.poll(() => endless(page), { timeout: 15_000 }).toEqual([]);
});
