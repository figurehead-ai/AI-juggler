/**
 * X2 (D-62.20): a 50-member pool opens at once (first paint ≤ 100 ms).
 * The rc26 X2 walk measured 120–140 ms from the click to its full
 * canvas, all in one frame: its members, then the canvas read for them,
 * read one after the other once the page had opened. Hovering or
 * focusing a pool's card reads both ahead, so the click paints from them.
 *
 * Measured with the browser's clock, from the click to the 50th node
 * drawn; the median of five opens. The preview's reads answer at once,
 * so this holds the page's own share (render and layout); the reads
 * ahead are held by `canvas-prefetch.test.tsx`.
 */
import { expect, test, type Page } from "@playwright/test";
import { goTo } from "./fixtures";
import { bigCanvasSeed } from "./fixtures/pool-canvas";

const NOW = Date.UTC(2026, 10, 6, 10, 0, 0);
const BUDGET_MS = 100;

async function start(page: Page) {
  await page.addInitScript((seed) => {
    Object.assign(window, seed);
    localStorage.setItem("aij.theme", "dark");
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
  }, bigCanvasSeed(NOW));
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

test("a 50-member pool opens from its card in ≤ 100 ms", async ({ page }) => {
  await start(page);
  const runs: number[] = [];
  for (let n = 0; n < 5; n += 1) {
    await goTo(page, "Pools");
    const card = page.getByTestId("pool-card-tile").filter({ hasText: "Fifty agents" });
    await card.hover();
    await page.waitForTimeout(150);
    const before = await page.evaluate(() => performance.now());
    await card.click();
    await expect(page.getByTestId("canvas-node")).toHaveCount(50);
    runs.push((await page.evaluate(() => performance.now())) - before);
  }
  runs.sort((a, b) => a - b);
  const median = runs[Math.floor(runs.length / 2)]!;
  console.log(`[perf] a 50-member pool opens: median ${median.toFixed(0)} ms (budget ${BUDGET_MS} ms) — ${runs.map((r) => r.toFixed(0)).join(", ")}`);
  expect(median).toBeLessThanOrEqual(BUDGET_MS);
});
