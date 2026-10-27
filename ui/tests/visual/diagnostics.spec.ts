/**
 * AJ-E1517 gallery: Settings › Report a problem — the bug form, and the
 * diagnostics preview (every file, expandable, and the settings left out)
 * before anything is saved, then where it was saved. Both themes; axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, settingsPane } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/85-diagnostics");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript((theme) => {
    localStorage.setItem("aij.theme", theme);
    // AJ-E1522: a pinned clock, so the capture is the same every run.
    Date.now = () => 1_728_800_000_000;
    localStorage.setItem("aij.onboarding.dismissed", "1");
    localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
    sessionStorage.setItem("aij.ready.skipped", "1");
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
  test(`diagnostics are shown before they're saved — ${theme}`, gate("85-diagnostics"), async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Settings");
    await settingsPane(page, "Advanced");
    const card = page.getByTestId("settings-diagnostics");
    await card.getByRole("button", { name: "Report a bug" }).click();
    expect(await page.evaluate(() => (window as unknown as { __AIJ_BUG_FORM_OPENED__?: number }).__AIJ_BUG_FORM_OPENED__)).toBe(1);
    await card.getByRole("button", { name: "Export diagnostics…" }).click();
    const preview = card.getByTestId("diagnostics-preview");
    await expect(preview.getByTestId("diagnostics-file")).toHaveCount(3);
    await preview.getByText("about.json", { exact: true }).click();
    await expect(preview.getByLabel("about.json (read-only)")).toContainText('"schema_on_disk": 31');
    await expect(card.getByTestId("diagnostics-left-out")).toContainText("3 settings that hold tokens, addresses or paths");
    await card.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `preview-1280x800-${theme}.png`) });
    await axe(page, `diagnostics preview ${theme}`);

    await page.evaluate(() => {
      (window as unknown as { __AIJ_DIAGNOSTICS_DEST__?: string }).__AIJ_DIAGNOSTICS_DEST__ =
        "/Users/someone/Desktop/AI Juggler diagnostics 2026-10-23.zip";
    });
    await card.getByRole("button", { name: "Save…" }).click();
    await expect(card.getByRole("status")).toHaveText(
      "Saved to ~/Desktop/AI Juggler diagnostics 2026-10-23.zip (5 KB). Attach it to your report.",
    );
    await expect(preview).toHaveCount(0);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `saved-1280x800-${theme}.png`) });
    await axe(page, `diagnostics saved ${theme}`);
  });
}
