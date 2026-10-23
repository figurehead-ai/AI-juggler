/**
 * AJ-E1511 gallery: New session › Start from a context pack — the pack
 * checked and shown before anything starts (where it came from, its
 * fidelity, what carries and doesn't, what was withheld), then launched
 * into the chosen account and folder. Both themes; axe 0 critical/serious.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/86-open-pack");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript((theme) => {
    localStorage.setItem("aij.theme", theme);
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
    (window as unknown as { __AIJ_PACK_DIR__?: string }).__AIJ_PACK_DIR__ = "/Users/dev/exports/pack-1";
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
  test(`a pack is shown before it starts, then launched — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await page.getByRole("button", { name: "New session", exact: true }).first().click();
    const sheet = page.getByTestId("new-session-sheet");
    await sheet.getByRole("button", { name: "Start from a context pack…" }).click();
    const summary = sheet.getByTestId("pack-summary");
    await expect(summary).toContainText("Claude Code on Work Max");
    await expect(summary).toContainText("Full story, fresh start · Beta");
    await expect(sheet.getByTestId("pack-withheld")).toHaveText("2 API keys, when it was made. They stay out.");
    await summary.getByText("The pack's brief").click();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `summary-1280x800-${theme}.png`) });
    await axe(page, `open pack ${theme}`);

    await sheet.getByRole("button", { name: /^Launch/ }).click();
    await expect(sheet).toHaveCount(0);
    const started = await page.evaluate(() => (window as unknown as { __AIJ_PACK_STARTED__?: unknown }).__AIJ_PACK_STARTED__);
    expect(started).toMatchObject({ path: "/Users/dev/exports/pack-1", digest: "preview", folder: "/Users/you/Dev/payments-api" });
    await expect(page.getByTestId("terminal-toolbar")).toBeVisible();
  });
}
