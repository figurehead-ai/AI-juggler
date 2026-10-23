/**
 * AJ-E1509 gallery: keeping work going — Settings › Sessions' "Keep work
 * going" switches, This Mac's line on what keeps the Mac awake (reached
 * from it to Settings in one click), and a launch setting's own restart
 * choice. Both themes; axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { goTo } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/83-keep-going");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, theme: "dark" | "light") {
  await page.addInitScript((theme) => {
    localStorage.setItem("aij.theme", theme);
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
  test(`This Mac says what keeps it awake, one click from its setting — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "This Mac");
    const line = page.getByTestId("this-mac-page").getByTestId("this-mac-awake");
    await expect(line).toContainText("Kept awake while 1 AI Juggler session works");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `this-mac-awake-1280x800-${theme}.png`) });
    await axe(page, `This Mac awake ${theme}`);
    await line.getByRole("button", { name: "Change when the Mac is kept awake" }).click();
    const keep = page.getByTestId("settings-keep-going");
    await expect(keep).toBeVisible();
    await expect(page.getByLabel("Restart sessions that crash")).not.toBeChecked();
    await expect(page.getByLabel("Keep the Mac awake while sessions work")).toBeChecked();
    await expect(page.getByLabel("Reopen AI Juggler if it quits unexpectedly")).toBeChecked();
    await page.getByLabel("Restart sessions that crash").check();
    await expect(page.getByTestId("settings-sessions").getByRole("status")).toHaveText("Sessions that crash are started again.");
    await keep.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `settings-keep-going-1280x800-${theme}.png`) });
    await axe(page, `Settings keep going ${theme}`);
  });

  test(`a launch setting chooses for itself — ${theme}`, async ({ page }) => {
    await open(page, theme);
    await goTo(page, "Accounts");
    await page.getByRole("button", { name: /Edit launch settings/ }).first().click();
    await page.getByTestId("recipe-env-toggle").first().click();
    const choice = page.getByLabel("If a session crashes").first();
    await expect(choice).toHaveValue("settings");
    await choice.selectOption("restart");
    await expect(choice).toHaveValue("restart");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `launch-setting-restart-1280x800-${theme}.png`) });
    await axe(page, `launch setting restart ${theme}`);
  });
}
