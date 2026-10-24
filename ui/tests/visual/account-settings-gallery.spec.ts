/**
 * AJ-E1508 gallery: Add account's fresh account starts with the usual
 * settings of an account of the same tool — the line, collapsed and open
 * (what comes with it, ticked), and the sign-in after it's made; and an
 * account's "Copy settings from…" (an account brought in as a copy, or
 * any account later). Both themes, axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/83-account-settings");

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function openFresh(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_SCAN_REPORT__ = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false };
      localStorage.setItem("aij.theme", theme);
      // AJ-E1522: a pinned clock, so the capture is the same every run.
      Date.now = () => 1_728_800_000_000;
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: structuredClone(allStatuses), theme },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await goTo(page, "Accounts");
  await page.getByTestId("add-account").click();
  await expect(page.getByTestId("first-run-fresh")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`the line, collapsed — ${theme}`, gate("83-account-settings"), async ({ page }) => {
    await openFresh(page, theme);
    const line = page.getByTestId("start-with-settings");
    await expect(line).toContainText("Start with the same settings as");
    await expect(line.getByRole("checkbox").first()).toBeChecked();
    await expect(line.getByRole("button", { name: /Its MCP servers, permissions and status line/ })).toBeVisible();
    await page.getByTestId("first-run-fresh").screenshot({ path: resolve(OUT, `collapsed-${theme}.png`) });
    await axeClean(page, "the fresh account with its settings line");
  });

  test(`the line, open — ${theme}`, async ({ page }) => {
    await openFresh(page, theme);
    const line = page.getByTestId("start-with-settings");
    await line.getByRole("button", { name: /Its MCP servers/ }).click();
    await expect(line).toContainText("claude plugin install tidy@market");
    await expect(line).toContainText("Never its sign-in, history or caches");
    await page.getByTestId("first-run-fresh").screenshot({ path: resolve(OUT, `open-${theme}.png`) });
    await axeClean(page, "the settings line, open");
    // Another account: Personal has nothing to copy, and says so.
    await line.getByRole("combobox", { name: "Account to copy the settings from" }).selectOption({ label: "Personal" });
    await expect(line).toContainText("Personal has no settings to copy");
    await page.getByTestId("first-run-fresh").screenshot({ path: resolve(OUT, `nothing-${theme}.png`) });
  });

  test(`made, then its sign-in — ${theme}`, async ({ page }) => {
    await openFresh(page, theme);
    await expect(page.getByTestId("start-with-settings").getByRole("button", { name: /Its MCP servers/ })).toBeVisible();
    await page.getByTestId("first-run-fresh-create").click();
    await expect(page.getByTestId("first-run-signin-row")).toBeVisible();
    await page.screenshot({ path: resolve(OUT, `signin-${theme}.png`) });
  });

  test(`an account's "Copy settings from…" — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", theme);
        // AJ-E1522: a pinned clock, so the capture is the same every run.
        Date.now = () => 1_728_800_000_000;
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: structuredClone(allStatuses), theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await goTo(page, "Accounts");
    await page.getByRole("button", { name: "Profile actions for Personal", exact: true }).click();
    await page.getByRole("menuitem", { name: "Copy settings from…" }).click();
    const sheet = page.getByRole("dialog", { name: "Copy settings into Personal" });
    await expect(sheet.getByRole("button", { name: "Copy 3 settings" })).toBeVisible();
    await sheet.screenshot({ path: resolve(OUT, `account-sheet-${theme}.png`) });
    await axeClean(page, "Copy settings into an account");
    await sheet.getByRole("button", { name: "Copy 3 settings" }).click();
    await expect(sheet.getByRole("button", { name: "Done" })).toBeVisible();
    await sheet.screenshot({ path: resolve(OUT, `account-sheet-done-${theme}.png`) });
    await axeClean(page, "Copy settings, done");
  });
}
