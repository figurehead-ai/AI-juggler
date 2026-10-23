/**
 * AJ-E1514 gallery: monthly caps. Usage lists each cap's month so far
 * (one near its cap, one over, one under), the form sets one for a group
 * of projects, and Home's Needs you warns at 80 % and 100 %, saying
 * nothing is stopped. Both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/89-budget-caps");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript((theme) => {
    localStorage.setItem("aij.theme", theme);
    // AJ-E1522: a pinned clock, so the capture is the same every run.
    Date.now = () => 1_728_800_000_000;
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
    const cap = (over: Record<string, unknown>) => ({
      id: "cap-acme", name: "Acme", monthly_usd: 300, account_id: null, project_ids: ["proj-ai-juggler", "proj-payments"],
      spent_usd: 243.4, unpriced: false, level: "near", seen: false, ...over,
    });
    (window as unknown as { __AIJ_BUDGETS__: unknown }).__AIJ_BUDGETS__ = [
      cap({}),
      cap({ id: "cap-side", name: "Side project", monthly_usd: 50, project_ids: ["proj-payments"], spent_usd: 61.2, level: "over", unpriced: true }),
      cap({ id: "cap-work", name: "Work", monthly_usd: 1500, account_id: "claude-work", project_ids: [], spent_usd: 412.75, level: "under" }),
    ];
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

for (const theme of ["dark", "light"] as const) {
  test(`Usage lists the monthly caps — ${theme}`, gate("89-budget-caps"), async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Usage");
    const caps = page.getByTestId("usage-caps");
    await expect(caps.getByTestId("cap-row")).toHaveCount(3);
    await expect(caps.getByTestId("cap-spent").first()).toHaveText("≈$243 of $300 this month");
    await expect(caps).toContainText("Some usage has no price yet");
    await caps.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `usage-caps-1280x800-${theme}.png`) });
    await axe(page, `caps ${theme}`);
  });

  test(`setting a cap for a group of projects — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Usage");
    await page.getByTestId("cap-add").click();
    const sheet = page.getByTestId("cap-sheet");
    await sheet.getByTestId("cap-name").fill("Acme");
    await sheet.getByTestId("cap-amount").fill("300");
    await sheet.getByLabel("A group of projects, on any account").check();
    await sheet.getByTestId("cap-projects").getByRole("checkbox").first().check();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `cap-sheet-1280x800-${theme}.png`) });
    await axe(page, `cap sheet ${theme}`);
  });

  test(`Needs you warns at 80 % and 100 % — ${theme}`, async ({ page }) => {
    await open(page, theme);
    const items = page.getByTestId("home-cap");
    await expect(items).toHaveCount(2);
    await expect(items.first()).toContainText("Acme is at 81% of its monthly cap");
    await expect(items.nth(1)).toContainText("Side project is over its monthly cap");
    await expect(items.first()).toContainText("Nothing is stopped.");
    await items.first().scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `home-needs-1280x800-${theme}.png`) });
    await axe(page, `home caps ${theme}`);
  });
}
