/**
 * AJ-E1014 (D-25) gallery: Home in each of its states, the rail's Filters
 * open, and the More menu — each scanned with axe (0 critical/serious).
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, honestEmpty, openFilters, type FixtureSet } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/67-home");
const T0 = 1_728_800_000_000;

const allClear: FixtureSet = {
  ...allStatuses,
  sessions: allStatuses.sessions.filter((s) => s.status === "running" && !s.attention),
};
const nothingYet: FixtureSet = { ...allStatuses, sessions: [] };

const STATES: Array<{ name: string; fixture: FixtureSet; expect: string }> = [
  { name: "needs-you", fixture: allStatuses, expect: "need you" },
  { name: "all-clear", fixture: allClear, expect: "All clear" },
  { name: "no-sessions", fixture: nothingYet, expect: "Ready when you are" },
  { name: "no-accounts", fixture: honestEmpty, expect: "All your AI coding accounts, in one calm place." },
];
const VIEWPORTS = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "390x844", width: 390, height: 844 },
] as const;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, fixture: FixtureSet, theme: "dark" | "light", vp: { width: number; height: number }) {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  for (const vp of VIEWPORTS) {
    for (const state of STATES) {
      test(`home — ${state.name} — ${vp.name} — ${theme}`, async ({ page }) => {
        await seed(page, state.fixture, theme, vp);
        await expect(page.getByRole("main")).toContainText(state.expect);
        await page.screenshot({ path: resolve(OUT, `${state.name}-${vp.name}-${theme}.png`) });
        await axeClean(page, `home ${state.name}`);
      });
    }
  }

  // AJ-E1524: the welcome for a new install, 390 px to 1440 px, both
  // densities, and with How it works open.
  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x680", width: 1024, height: 680 },
  ]) {
    test(`home — welcome — ${vp.name} — ${theme}`, async ({ page }) => {
      await seed(page, honestEmpty, theme, vp);
      await expect(page.getByTestId("welcome-found")).not.toContainText("Looking");
      await page.screenshot({ path: resolve(OUT, `welcome-${vp.name}-${theme}.png`) });
      await axeClean(page, "home welcome");
    });
  }

  test(`home — welcome, compact rail — ${theme}`, async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("aij.density", "compact"));
    await seed(page, honestEmpty, theme, { width: 1280, height: 800 });
    await expect(page.getByTestId("welcome-found")).not.toContainText("Looking");
    await page.screenshot({ path: resolve(OUT, `welcome-compact-1280x800-${theme}.png`) });
  });

  test(`home — welcome, How it works open — ${theme}`, async ({ page }) => {
    await seed(page, honestEmpty, theme, { width: 1280, height: 800 });
    await page.getByTestId("welcome-how").locator("summary").click();
    await expect(page.getByTestId("welcome-how").locator("ol > li")).toHaveCount(3);
    await page.getByTestId("welcome-how").locator("ol > li").last().scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `welcome-how-1280x800-${theme}.png`) });
    await axeClean(page, "home welcome, how it works");
  });

  test(`rail — Filters open — ${theme}`, async ({ page }) => {
    await seed(page, allStatuses, theme, { width: 1280, height: 800 });
    await openFilters(page);
    await page.screenshot({ path: resolve(OUT, `rail-filters-open-1280x800-${theme}.png`) });
    await axeClean(page, "rail filters");
  });

  test(`header — More menu — ${theme}`, async ({ page }) => {
    await seed(page, allStatuses, theme, { width: 1280, height: 800 });
    await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "More", exact: true }).click();
    const menu = page.locator("#nav-more");
    // Every item is reachable — the menu paints above the page (D-25 fix).
    for (const name of ["Usage", "Fleet", "Cleanup"]) {
      await expect(menu.getByRole("button", { name: new RegExp(`^${name}\\b`) })).toBeInViewport({ ratio: 1 });
    }
    await page.screenshot({ path: resolve(OUT, `more-menu-1280x800-${theme}.png`) });
    await axeClean(page, "more menu");
    await page.mouse.click(5, 790);
    await expect(menu).toHaveCount(0);
  });
}

// 0.0.6 dogfood: the rail's "?" opened a legend that was clipped out of
// sight — the suites checked it opened, never that it could be seen.
// AJ-E403 Req.6: the same legend from the rail footer and from Home's
// header (its "All clear" explainer since Overview folded into Home).
for (const theme of ["dark", "light"] as const) {
  for (const where of ["rail", "home"] as const) {
    test(`legend — "?" on ${where} shows it on top, in view, and closes — ${theme}`, async ({ page }) => {
      await seed(page, allStatuses, theme, { width: 1280, height: 800 });
      await page.getByTestId(where).getByRole("button", { name: "Indicator legend" }).click();
      const legend = page.getByRole("dialog", { name: "What the icons mean" });
      await expect(legend).toBeInViewport({ ratio: 1 });
      // Nothing paints over it: the topmost element at its centre is inside it.
      const onTop = await legend.evaluate((node) => {
        const box = node.getBoundingClientRect();
        const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
        return hit !== null && node.contains(hit);
      });
      expect(onTop, "the legend is covered by another layer").toBe(true);
      await expect(legend).toContainText("Waiting for you");
      await page.screenshot({ path: resolve(OUT, where === "rail" ? `legend-1280x800-${theme}.png` : `legend-home-1280x800-${theme}.png`) });
      await axeClean(page, `legend from ${where}`);
      await page.keyboard.press("Escape");
      await expect(legend).toHaveCount(0);
      await expect(page.getByTestId(where).getByRole("button", { name: "Indicator legend" })).toBeFocused();
    });
  }
}

