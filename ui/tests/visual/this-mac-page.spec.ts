/**
 * AJ-E1528 gallery and navigation: the This Mac page, reached from Home's
 * panel ("See all"), from More and from the rail's footer line; with
 * many Claude app windows (`?aij_windows=6`) and a heavy session
 * (`?aij_load=heavy`). Both themes and densities; axe 0 critical/serious.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/81-this-mac");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light", density: "comfortable" | "compact", query = "") {
  await page.addInitScript(
    ({ theme, density }) => {
      localStorage.setItem("aij.theme", theme);
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

test("Home's panel, More and the rail's footer all open the page", async ({ page }) => {
  await open(page, "dark", "comfortable");
  await page.getByTestId("this-mac-open-page").click();
  await expect(page.getByTestId("this-mac-page")).toBeVisible();
  await goTo(page, "Home");
  await goTo(page, "This Mac");
  await expect(page.getByTestId("this-mac-page")).toBeVisible();
  await goTo(page, "Home");
  await page.getByTestId("machine-status").click();
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
    test(`the page, six Claude app windows and a heavy session — ${theme} ${density}`, async ({ page }) => {
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
