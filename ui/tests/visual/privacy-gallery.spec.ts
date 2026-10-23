/**
 * AJ-E1518 gallery: the Privacy page (from More; a line opened to its
 * tests; the tools' terms at the end) and the terms line while an account
 * is added. Both themes, axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/84-privacy");

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: structuredClone(allStatuses), theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`the Privacy page — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Privacy");
    const privacy = page.getByTestId("privacy-page");
    await expect(privacy.getByRole("heading", { level: 1 })).toHaveText("Privacy");
    await expect(page.getByTestId("privacy-claim").first()).toBeVisible();
    await page.screenshot({ path: resolve(OUT, `page-${theme}.png`) });
    await axeClean(page, "the Privacy page");

    // A line opened: the tests that check it, each linked.
    const line = page.getByTestId("privacy-claim").filter({ hasText: "no requests to" });
    await line.locator("summary").click();
    await expect(line.getByTestId("privacy-tests")).toContainText("only_the_listed_crates_can_reach_the_network");
    await line.scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `tests-open-${theme}.png`) });
    await axeClean(page, "a line opened to its tests");

    // The tools' terms, at the end.
    const terms = page.getByTestId("privacy-terms");
    await terms.scrollIntoViewIfNeeded();
    await expect(page.getByTestId("privacy-terms-row")).toHaveCount(2);
    await page.screenshot({ path: resolve(OUT, `terms-${theme}.png`) });
  });

  test(`the terms line in Add account — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Accounts");
    await page.getByTestId("add-account").click();
    const line = page.getByTestId("terms-line");
    await expect(line).toBeVisible();
    // The setups found, not the look for them.
    await expect(page.getByText(/^Looking for /)).toHaveCount(0);
    await expect(line.getByRole("button", { name: /Terms/ })).toBeVisible();
    await page.screenshot({ path: resolve(OUT, `add-account-${theme}.png`) });
    await axeClean(page, "Add account with the terms line");
  });

  test(`Settings links to it — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Settings");
    await page.getByTestId("privacy-open-page").click();
    await expect(page.getByTestId("privacy-page")).toBeVisible();
  });
}
