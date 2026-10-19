/**
 * AJ-E804 Req.5 — rail, inspector and the switch wizard at 200 % zoom
 * and with reduced motion, both themes, at 1280x800. 200 % zoom of a
 * 1280x800 window is a 640x400 CSS viewport at device scale 2: nothing
 * may need horizontal page scrolling (WCAG 1.4.10) and the controls stay
 * reachable. With reduced motion no animation may be running, so no
 * information lives only in movement. Captures go to the AJ-E804
 * evidence under `just a11y-gate` (AJ_A11Y_EVIDENCE=1).
 */
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, DOOR_WIZARD } from "./fixtures";

const T0 = 1_728_800_000_000;
const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E804/zoom-motion");
const EVIDENCE = process.env.AJ_A11Y_EVIDENCE === "1";

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, t0, theme: t, preflight }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_SWITCH_PREFLIGHT__ = preflight;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, t0: T0, theme, preflight: DOOR_WIZARD },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.goto("/");
  await expect(page.getByRole("tab", { name: /^Instances/ })).toBeVisible();
}

async function shoot(page: Page, name: string) {
  if (!EVIDENCE) return;
  mkdirSync(OUT, { recursive: true });
  await page.screenshot({ path: resolve(OUT, `${name}.png`) });
}

async function noHorizontalScroll(page: Page) {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow, "the page needs horizontal scrolling").toBeLessThanOrEqual(0);
}

async function runningAnimations(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    document
      .getAnimations()
      .filter((a) => a.playState === "running" && Number(a.effect?.getComputedTiming().duration ?? 0) > 1)
      .map((a) => (a as CSSAnimation).animationName ?? a.constructor.name),
  );
}

/** Every top-bar menu and New are on screen — not clipped away. */
async function menusOnScreen(page: Page) {
  for (const name of ["Triage", "Overview", "History", "Accounts", "Usage", "Fleet", "Cleanup", "Settings"]) {
    await expect(page.getByRole("button", { name, exact: true }).first(), `${name} is off screen`).toBeInViewport({ ratio: 1 });
  }
  await expect(page.getByRole("button", { name: /^New$/ }).first()).toBeInViewport({ ratio: 1 });
}

async function walk(page: Page, theme: string, label: string) {
  await menusOnScreen(page);
  // Rail.
  await page.getByRole("tab", { name: /^Instances/ }).click();
  await expect(page.locator("[data-instance-id]").first()).toBeVisible();
  await noHorizontalScroll(page);
  expect(await runningAnimations(page), "animations under reduced motion").toEqual([]);
  await shoot(page, `rail-${label}-${theme}`);
  // Inspector.
  await page.getByRole("tab", { name: "Sessions" }).click();
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await expect(page.getByRole("tab", { name: "Overview" })).toBeVisible();
  await noHorizontalScroll(page);
  expect(await runningAnimations(page)).toEqual([]);
  await shoot(page, `inspector-${label}-${theme}`);
  // The switch wizard.
  await page.getByTestId("handover-section").locator("button[data-row]:not(:disabled)").first().scrollIntoViewIfNeeded();
  await page.getByTestId("handover-section").locator("button[data-row]:not(:disabled)").first().click();
  await expect(page.getByTestId("switch-flow")).toBeVisible();
  await expect(page.getByTestId("switch-continue")).toBeInViewport();
  await noHorizontalScroll(page);
  expect(await runningAnimations(page)).toEqual([]);
  await shoot(page, `wizard-${label}-${theme}`);
}

for (const theme of ["dark", "light"] as const) {
  test.describe(`200 % zoom — ${theme}`, () => {
    test.use({ viewport: { width: 640, height: 400 }, deviceScaleFactor: 2 });
    test(`rail, inspector, wizard reflow at 200 % — ${theme}`, async ({ page }) => {
      await open(page, theme);
      await walk(page, theme, "zoom200");
    });
  });
  test.describe(`reduced motion — ${theme}`, () => {
    test.use({ viewport: { width: 1280, height: 800 } });
    test(`rail, inspector, wizard hold still — ${theme}`, async ({ page }) => {
      await open(page, theme);
      await walk(page, theme, "reduced-motion");
    });
  });
}
