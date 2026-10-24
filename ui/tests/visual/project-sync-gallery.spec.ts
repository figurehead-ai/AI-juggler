/**
 * AJ-E1507 gallery: a project's rules files and MCP servers made the same
 * across tools — the project's header line, the sheet with its diff
 * first, the sheet after the master is applied and a server copied, and
 * the project menu's way in. Both themes, axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, projectLens } from "./fixtures";
import { EXAMPLE_PROJECT_SYNC } from "../../src/preview/project-sync";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/82-project-sync");
const T0 = 1_792_700_000_000;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, sync, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_PROJECT_SYNC__ = sync;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: structuredClone(allStatuses), sync: structuredClone(EXAMPLE_PROJECT_SYNC), theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("instance-board")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function openProject(page: Page) {
  await projectLens(page);
  await page.getByTestId("rail").getByTestId("rail-project").first().click();
  await expect(page.getByTestId("project-rules")).toHaveText("1 rules file differs");
}

for (const theme of ["dark", "light"] as const) {
  test(`the project's rules line — ${theme}`, gate("82-project-sync"), async ({ page }) => {
    await seed(page, theme);
    await openProject(page);
    await page.getByTestId("project-root").locator("..").screenshot({ path: resolve(OUT, `header-line-${theme}.png`) });
    await axeClean(page, "the project header with its rules line");
  });

  test(`the sheet, diff first — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openProject(page);
    await page.getByTestId("project-rules").click();
    const sheet = page.getByRole("dialog", { name: "Rules and MCP servers" });
    await expect(sheet.getByTestId("sync-diff")).toContainText("Use two spaces.");
    await expect(sheet.getByTestId("project-sync-servers")).toContainText("GITHUB_TOKEN stays behind");
    await sheet.screenshot({ path: resolve(OUT, `sheet-diff-${theme}.png`) });
    await axeClean(page, "the sync sheet with its diff");
  });

  test(`the sheet after applying and copying — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openProject(page);
    await page.getByTestId("project-rules").click();
    const sheet = page.getByRole("dialog", { name: "Rules and MCP servers" });
    await expect(sheet.getByTestId("sync-diff")).toBeVisible();
    await sheet.getByRole("button", { name: "Apply to AGENTS.md" }).click();
    await expect(sheet.getByTestId("project-sync-summary")).toHaveText("Rules in sync");
    await sheet.getByRole("button", { name: "Copy github to Codex" }).click();
    await expect(sheet.getByText("Every account has the same servers.")).toBeVisible();
    await sheet.screenshot({ path: resolve(OUT, `sheet-done-${theme}.png`) });
    await axeClean(page, "the sync sheet after applying");
    // Focus stayed inside: Escape closes it, and the header says so.
    await page.keyboard.press("Escape");
    await expect(sheet).toHaveCount(0);
    await expect(page.getByTestId("project-rules")).toHaveText("Rules in sync");
  });

  test(`the project menu's way in — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    const row = page.getByTestId("rail").getByTestId("rail-project").first();
    await row.hover();
    await page.getByRole("button", { name: /^Project actions for / }).first().click();
    const menu = page.getByRole("menu");
    await expect(menu.getByRole("menuitem", { name: "Compare rules and servers…" })).toBeVisible();
    await menu.screenshot({ path: resolve(OUT, `menu-${theme}.png`) });
    await menu.getByRole("menuitem", { name: "Compare rules and servers…" }).click();
    await expect(page.getByRole("dialog", { name: "Rules and MCP servers" })).toBeVisible();
  });
}
