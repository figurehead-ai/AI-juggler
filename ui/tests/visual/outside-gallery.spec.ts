/**
 * AJ-E1529 gallery: sessions started outside AI Juggler, on the replica
 * (tests/visual/fixtures/replica.json, made by the backend's own readers):
 * Home's Running list, the rail under each project, an outside
 * conversation's page, the Accounts cards' counts and a project's page.
 * Both themes, axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, openOutsidePage, projectLens } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/78-outside");
const FIXTURE = JSON.parse(readFileSync(resolve(process.cwd(), "tests/visual/fixtures/replica.json"), "utf8"));

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: FIXTURE, theme, t0: FIXTURE.generated_ms },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-1280x800-${theme}.png`) });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`Home and the rail with sessions started outside AI Juggler — ${theme}`, gate("78-outside"), async ({ page }) => {
    await seed(page, theme);
    // AJ-E2105: Home's apps; the rail's places.
    await expect(page.getByTestId("home-app-tile").first()).toBeVisible();
    await shoot(page, "home-rail", theme);
  });

  test(`an outside conversation's page — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    // A rail row opens its page here; its ↗ goes to its app.
    await openOutsidePage(page);
    await expect(page.getByTestId("conversation-page")).toBeVisible();
    await shoot(page, "conversation-page", theme);
  });

  test(`Accounts, counting them as running — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await goTo(page, "Accounts");
    await expect(page.getByTestId("account-activity").first()).toBeVisible();
    await shoot(page, "accounts", theme);
  });

  test(`a project's page, listing them — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    await page.getByTestId("rail").getByTestId("rail-project").first().click();
    // AJ-E1533: the project's Sessions tab lists every session, outside
    // ones too (0.3: a project opens on its Overview).
    await page.getByRole("tablist", { name: "Project tabs" }).getByRole("tab", { name: "Sessions" }).click();
    await expect(page.getByTestId("page-panel-sessions").locator('[data-testid="page-session-row"][data-outside="true"]').first()).toBeVisible();
    await shoot(page, "project-page", theme);
  });
}
