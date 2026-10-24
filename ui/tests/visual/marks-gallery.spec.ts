/**
 * AJ-E1532 gallery: telling sixteen projects apart, on the replica
 * (tests/visual/fixtures/replica.json, made by the backend's own readers,
 * marks and icons included): the rail's marks and account stacks, a
 * folded project as a stripe in its colour, History's rows, and the mark
 * sheet — before and after an emoji is chosen. Both themes, axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { goTo, projectLens } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/80-marks");
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
    { fixture: structuredClone(FIXTURE), theme, t0: FIXTURE.generated_ms },
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

test("the replica has the projects to tell apart: 15+, every one with a mark, icons only where they're real images", () => {
  const projects = FIXTURE.projects as { id: string; name: string; mark?: { shows: string; swatch: number; clone?: number } }[];
  expect(projects.length).toBeGreaterThanOrEqual(15);
  for (const project of projects) expect(project.mark, project.name).toBeTruthy();
  const by = (name: string) => projects.find((project) => project.name === name)!.mark!;
  expect(by("design-system").shows).toBe("letters"); // its logo is over 512 KB
  expect(by("cli-tools").shows).toBe("letters"); // its "logo" isn't an image
  for (const name of ["AIJuggler", "infra", "mobile-app", "marketing-site", "docs-portal", "scratch"]) expect(by(name).shows, name).toBe("icon");
  expect(["acme-web", "acme-web2", "acme-web3"].map((name) => by(name).clone)).toEqual([1, 2, 3]);
  expect(new Set(["acme-web", "acme-web2", "acme-web3"].map((name) => by(name).swatch)).size).toBe(1);
  expect(Object.keys(FIXTURE.project_icons).length).toBeGreaterThanOrEqual(8);
});

for (const theme of ["dark", "light"] as const) {
  test(`the rail: marks, stacks and a folded project — ${theme}`, gate("80-marks"), async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    const rail = page.getByTestId("rail");
    await expect(rail.getByTestId("rail-project").first().getByTestId("mark-project")).toBeVisible();
    await expect(rail.getByTestId("account-stack").first()).toBeVisible();
    await rail.getByTestId("rail-fold").first().click();
    await expect(rail.getByTestId("rail-folded-stripe")).toHaveCount(1);
    await shoot(page, "rail-folded", theme);
  });

  test(`History: each row's tool, project and account — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await goTo(page, "History");
    await expect(page.getByTestId("mark-project").nth(4)).toBeVisible();
    await shoot(page, "history", theme);
  });

  test(`the mark sheet, then an emoji on its colour — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await projectLens(page);
    await page.getByRole("button", { name: /^Project actions for / }).first().click();
    await page.getByRole("menuitem", { name: "Change its mark…" }).click();
    await expect(page.getByTestId("mark-sheet")).toBeVisible();
    await shoot(page, "sheet", theme);
    await page.getByTitle("Pink").click(); // the swatch, as a person clicks it
    await expect(page.getByRole("radio", { name: "Pink" })).toBeChecked();
    await page.getByTestId("mark-shows-emoji").check();
    await page.getByTestId("mark-emoji").fill("🚀");
    await shoot(page, "sheet-emoji", theme);
    await page.getByTestId("mark-save").click();
    await expect(page.getByTestId("mark-sheet")).toHaveCount(0);
    await expect(page.getByTestId("rail").getByTestId("rail-project").first().getByTestId("mark-project")).toHaveAttribute("data-kind", "emoji");
    await shoot(page, "rail-emoji", theme);
  });
}
