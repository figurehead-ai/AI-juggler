/**
 * AJ-E1528 gallery and navigation: the This Mac page, reached from More
 * and from the top bar's This Mac line; with
 * many Claude app windows (`?aij_windows=6`) and a heavy session
 * (`?aij_load=heavy`); and the 0.1.4 review's Mac, other apps holding most
 * of the memory (`?aij_machine=others`). Both themes and densities; axe 0
 * critical/serious.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/81-this-mac");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light", density: "comfortable" | "compact", query = "") {
  await page.addInitScript(
    ({ theme, density }) => {
      localStorage.setItem("aij.theme", theme);
      // AJ-E1522: a pinned clock, so the capture is the same every run.
      Date.now = () => 1_728_800_000_000;
      localStorage.setItem("aij.density", density);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, density },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto(`/${query}`);
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

// Home's This Mac panel went with AJ-E2105 (This Mac's line moved to the
// top bar); More and that line are the ways to the page.
test("More and the top bar's This Mac line open the page", async ({ page }) => {
  await open(page, "dark", "comfortable");
  await goTo(page, "This Mac");
  await expect(page.getByTestId("this-mac-page")).toBeVisible();
  await goTo(page, "Home");
  await page.getByTestId("machine-status").first().click();
  await expect(page.getByTestId("this-mac-page")).toBeVisible();
});

// Screen-reader words are absolutely placed: with no positioned box
// around them they hung below the window, and the whole app could
// scroll (a Home row's "Open" took focus and the shell slid up).
test("the window itself never scrolls, on Home or the page", async ({ page }) => {
  await open(page, "dark", "comfortable", "?aij_windows=6&aij_load=heavy");
  await page.setViewportSize({ width: 1024, height: 768 });
  const overflow = () => page.evaluate(() => document.documentElement.scrollHeight - window.innerHeight);
  expect(await overflow()).toBeLessThanOrEqual(0);
  await goTo(page, "This Mac");
  await expect(page.getByTestId("this-mac-page")).toBeVisible();
  expect(await overflow()).toBeLessThanOrEqual(0);
});

for (const theme of ["dark", "light"] as const) {
  for (const density of ["comfortable", "compact"] as const) {
    test(`the page, six Claude app windows and a heavy session — ${theme} ${density}`, gate("81-this-mac"), async ({ page }) => {
      await open(page, theme, density, "?aij_windows=6&aij_load=heavy");
      await goTo(page, "This Mac");
      const page_ = page.getByTestId("this-mac-page");
      await expect(page_.getByTestId("this-mac-group")).toHaveCount(2);
      await expect(page_.getByTestId("this-mac-heavy")).toBeVisible();
      // The windows of a tool AI Juggler didn't start: behind the
      // disclosure, every one listed, none with a control.
      const theirs = page_.locator('[data-testid="this-mac-group"][data-ours="false"]');
      await theirs.locator("summary").click();
      await expect(theirs.getByTestId("this-mac-tree")).toHaveCount(6);
      await expect(theirs.getByRole("button")).toHaveCount(0);
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `page-1280x800-${theme}-${density}.png`), fullPage: true });
      await axe(page, `This Mac ${theme} ${density}`);
    });
  }
}

// The 0.1.4 review: "Under memory pressure" beside "AI tools: about 16 GB
// of 103 GB" read as plenty left. The whole Mac's memory in named parts,
// pressure explained, and what else uses it, by name, with no control.
for (const theme of ["dark", "light"] as const) {
  test(`the page, other apps holding most of the memory — ${theme}`, async ({ page }) => {
    await open(page, theme, "comfortable", "?aij_machine=others");
    await goTo(page, "This Mac");
    const page_ = page.getByTestId("this-mac-page");
    await expect(page_.getByTestId("this-mac-memory-parts")).toContainText("Other apps and macOS");
    await expect(page_.getByTestId("this-mac-memory-parts")).toContainText("Kept compressed 47 GB");
    const note = page_.getByTestId("this-mac-pressure-note");
    await note.getByText("What it means").click();
    await expect(note).toContainText("Pressure can be high while some memory still looks free");
    const others = page_.getByTestId("this-mac-other-apps");
    await others.locator("summary").click();
    await expect(others.getByTestId("this-mac-other-app")).toHaveCount(6);
    await expect(others.getByTestId("this-mac-other-app").first()).toContainText("next-server");
    await expect(others.getByRole("button")).toHaveCount(0);
    await expect(page_.getByTestId("this-mac-alert").first()).toContainText("Other apps are using most of the memory");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `page-other-apps-1280x800-${theme}.png`), fullPage: true });
    await axe(page, `This Mac, other apps, ${theme}`);
  });
}
