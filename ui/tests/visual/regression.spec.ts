/**
 * AJ-E803 — the visual regression gate.
 *
 * A deterministic matrix (mocked bridge, frozen clock, no animation)
 * compared pixel by pixel against approved baselines rendered in the
 * pinned Playwright container — the same image locally
 * (`just visual-baselines`) and in CI — so a diff is the code, not the
 * machine. Thresholds: whole screens ≤ 0.1 % differing pixels; text-
 * bearing chrome (the rail's quick filters, a nav button, a status badge)
 * 0 pixels. A failure leaves expected/actual/diff (the triptych) in
 * `test-results/`, which CI uploads and summarises.
 *
 * Run: `just visual-gate` (or `visual-baselines` to approve changes,
 * with a per-screen note in the PR — scripts/check_baseline_notes.py).
 */
import { expect, test, type Page } from "@playwright/test";
import { allStatuses, goTo } from "./fixtures";

const T0 = 1_728_800_000_000;
const THEMES = ["dark", "light"] as const;
const DENSITIES = ["comfortable", "compact"] as const;
const VIEWPORTS = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1024x680", width: 1024, height: 680 },
] as const;

const SCREEN = { maxDiffPixelRatio: 0.001, animations: "disabled", caret: "hide" } as const;
const CHROME = { maxDiffPixels: 0, animations: "disabled", caret: "hide" } as const;

async function open(page: Page, theme: string, density: string, vp: { width: number; height: number }) {
  await page.addInitScript(
    ({ fixture, t, d, t0, selftest }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.density", d);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
      // The gate's self-test: a seeded 2 px shift of every button must
      // fail the comparison (AJ-E803 quality gate).
      if (selftest) {
        document.addEventListener("DOMContentLoaded", () => {
          const style = document.createElement("style");
          style.textContent = "button { transform: translateX(2px); }";
          document.head.append(style);
        });
      }
    },
    { fixture: allStatuses, t: theme, d: density, t0: T0, selftest: process.env.AIJ_VISUAL_SELFTEST === "1" },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme as "dark" | "light" });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Settings" }).first()).toBeVisible();
}

for (const theme of THEMES) {
  for (const density of DENSITIES) {
    for (const vp of VIEWPORTS) {
      const cell = `${theme}-${density}-${vp.name}`;

      test(`rail + home — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page).toHaveScreenshot(`home-${cell}.png`, SCREEN);
        // D-25: the rail's tabs gave way to its quick filters.
        await expect(page.getByRole("group", { name: "Show" })).toHaveScreenshot(`chrome-rail-filters-${cell}.png`, CHROME);
        await expect(page.getByRole("button", { name: "Settings" }).first()).toHaveScreenshot(
          `chrome-nav-button-${cell}.png`,
          CHROME,
        );
      });

      test(`inspector — Instance overview — ${cell}`, async ({ page }) => {
        await open(page, theme, density, vp);
        await expect(page.getByTestId("instance-board")).toBeVisible(); // D-25: one rail, always the list
        await page.getByRole("button", { name: /Work Max/ }).first().click();
        await expect(page.getByRole("tab", { name: "Quality / Performance" })).toBeVisible();
        await expect(page).toHaveScreenshot(`inspector-instance-${cell}.png`, SCREEN);
      });

      // D-25 amendment (AJ-E1021): Triage and Overview folded into Home.
      for (const screen of ["History", "Accounts", "Usage", "Fleet", "Cleanup", "Settings"] as const) {
        test(`${screen.toLowerCase()} — ${cell}`, async ({ page }) => {
          await open(page, theme, density, vp);
          await goTo(page, screen); // D-25: some screens sit under More
          await page.waitForLoadState("networkidle");
          await page.mouse.move(0, 0);
          await expect(page).toHaveScreenshot(`${screen.toLowerCase()}-${cell}.png`, SCREEN);
        });
      }
    }
  }
}

// WCAG 1.4.1: a status shown by colour must also say it in words or an
// accessible name — a colour-only change fails even when its pixel diff
// is small. The fixture holds an Instance in every status.
test("status is never colour-only", async ({ page }) => {
  await open(page, "dark", "comfortable", VIEWPORTS[0]);
  await expect(page.getByTestId("instance-board")).toBeVisible(); // D-25: one rail, always the list
  // D-25: the rail's own words for each state.
  const words = ["Starting", "Running", "Waiting for you", "Looks stuck", "Hit its limit", "Done", "Stopped", "Failed", "Handed over"];
  const missing: string[] = [];
  for (const word of words) {
    const asText = await page.getByText(word, { exact: true }).count();
    const asName = await page.getByRole("button", { name: word }).count();
    if (asText + asName === 0) missing.push(word);
  }
  expect(missing, "statuses with no words or accessible name").toEqual([]);
});
