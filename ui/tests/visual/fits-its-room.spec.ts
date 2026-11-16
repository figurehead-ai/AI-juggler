/**
 * The 0.4.6 RC walks' layout findings, measured in a real renderer (jsdom
 * has no layout): what a walker saw cut or scrolling, at the window sizes
 * walked. A8 (F-A8-3): a CLI App's Launch settings tab scrolled sideways,
 * its list pulled 20 px past the page's column by a negative margin.
 */
import { type Page } from "@playwright/test";
import { expect, test } from "./axe-gallery";
import { knownPlacesPreview, placesConversations } from "../../src/preview/places";

const T0 = new Date(2026, 10, 1, 15, 0, 0).getTime();

const ACCOUNTS = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: "max-20x", login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false },
];
const RECIPES = [{ id: "rec-main", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null }];

async function seed(page: Page, fixture: Record<string, unknown>, size: { width: number; height: number }) {
  await page.addInitScript(
    ({ fixture, known, recipes, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PLACES__ = known;
      w.__AIJ_RECIPES__ = recipes;
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, known: knownPlacesPreview(T0), recipes: RECIPES, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  await page.setViewportSize(size);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

/** Every scrolling box on the page whose content is wider than it. */
function sideways(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    [...document.querySelectorAll<HTMLElement>("main *, [data-testid='app-page'] *")]
      .filter((box) => /(auto|scroll)/.test(getComputedStyle(box).overflowX) && box.scrollWidth > box.clientWidth + 1)
      .map((box) => `${box.getAttribute("data-testid") ?? box.getAttribute("role") ?? box.tagName}: ${box.scrollWidth} px in ${box.clientWidth} px`),
  );
}

for (const size of [
  { width: 1280, height: 800 },
  { width: 1024, height: 700 },
]) {
  test(`A8: a CLI App's Launch settings never scroll sideways — ${size.width}`, async ({ page }) => {
    await seed(page, { accounts: ACCOUNTS, projects: [], sessions: [], conversations: placesConversations(T0) }, size);
    // Its page, by the door every name uses (lib/apps' openApp).
    await page.evaluate(() => window.dispatchEvent(new CustomEvent("aij:open-app", { detail: { id: "cli:claude-work" } })));
    await expect(page.getByTestId("page-name")).toHaveText("Claude Code CLI · Work Max");
    await page.getByRole("tab", { name: /^Launch settings/ }).click();
    await expect(page.getByTestId("profile-launch-settings")).toBeVisible();
    expect(await sideways(page)).toEqual([]);
  });
}
