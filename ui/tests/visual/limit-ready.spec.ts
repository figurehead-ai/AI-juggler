/**
 * AJ-E1513 gallery: the limit dialog when the switch was prepared before
 * the limit — "Ready to continue on Work Max · prepared 4 min ago" under
 * the pick. Both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses } from "./fixtures";

/** The fixtures' pinned clock: the session's window resets 2 h after. */
const T0 = 1_728_800_000_000;
const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/89-limit-ready");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ theme, fixture, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      Date.now = () => t0;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_STAGED__ = {
        session_id: "sess-5",
        target_recipe_id: "recipe-work-max",
        target_account_id: "acct-work",
        target_account_label: "Work Max",
        prepared_at_ms: t0 - 4 * 60_000,
        fidelity_words: "Full story, fresh start",
      };
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { theme, fixture: allStatuses, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  test(`the limit dialog says the prepared switch is ready — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await page.getByTestId("home-needs").getByRole("button", { name: /^Continue elsewhere/ }).first().click();
    const dialog = page.getByTestId("limit-dialog");
    await expect(dialog.getByTestId("limit-ready")).toHaveText("Ready to continue on Work Max · prepared 4 min ago");
    await expect(dialog.getByRole("button", { name: "Continue on Work Max" })).toBeVisible();
    await page.mouse.move(0, 0);
    const results = await new AxeBuilder({ page }).analyze();
    const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
    expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`)).toEqual([]);
    await page.screenshot({ path: `${OUT}/limit-ready-${theme}.png` });
  });
}
