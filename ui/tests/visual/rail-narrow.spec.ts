/**
 * D-25 stage 4 (AJ-E1020): below 900 px the rail folds to its icon
 * column by itself and comes back when the window widens. In a narrow
 * window ⌘B and the density shortcut only open or close the list for now
 * and never write Settings › Appearance, whatever is stored.
 * Gallery 69-rail-narrow, each state scanned with axe (0 critical/serious).
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, atScale } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/69-rail-narrow");
const T0 = 1_728_800_000_000;

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", width: number, height: number, density = "comfortable", fixture: unknown = allStatuses) {
  await page.addInitScript(
    ({ fixture, theme, t0, density }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.density", density);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: T0, density },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height });
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

const rail = (page: Page) => page.getByTestId("rail");

test("the rail folds below 900 px and comes back when wider; the stored density stays", async ({ page }) => {
  await seed(page, "dark", 1280, 800);
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  await page.setViewportSize({ width: 899, height: 800 });
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  await expect(rail(page)).toHaveCSS("width", "60px");
  await page.setViewportSize({ width: 900, height: 800 });
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  expect(await page.evaluate(() => localStorage.getItem("aij.density"))).toBe("comfortable");
});

test("compact is restored as compact, not comfortable", async ({ page }) => {
  await seed(page, "dark", 390, 844, "compact");
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  await page.setViewportSize({ width: 1280, height: 800 });
  await expect(rail(page)).toHaveAttribute("data-density", "compact");
});

test("⌘B opens the list while narrow, without changing the setting; widening forgets it", async ({ page }) => {
  await seed(page, "dark", 860, 800);
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  await page.keyboard.press("ControlOrMeta+b");
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  expect(await page.evaluate(() => localStorage.getItem("aij.density"))).toBe("comfortable");
  await page.keyboard.press("ControlOrMeta+b");
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  await page.keyboard.press("ControlOrMeta+b");
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  await page.setViewportSize({ width: 1280, height: 800 });
  // Let the wide width paint (two frames), as a real resize does.
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  await page.setViewportSize({ width: 860, height: 800 });
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
});

test("a chosen Minimal stays minimal at every width", async ({ page }) => {
  await seed(page, "dark", 1280, 800, "minimal");
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  await page.setViewportSize({ width: 600, height: 800 });
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
});

for (const theme of ["dark", "light"] as const) {
  for (const vp of [
    { name: "390x844", width: 390, height: 844 },
    { name: "860x700", width: 860, height: 700 },
  ]) {
    test(`gallery — folded rail — ${vp.name} — ${theme}`, gate("69-rail-narrow"), async ({ page }) => {
      await seed(page, theme, vp.width, vp.height);
      await expect(rail(page)).toHaveAttribute("data-density", "minimal");
      await page.screenshot({ path: resolve(OUT, `folded-${vp.name}-${theme}.png`) });
      await axeClean(page, `folded rail ${vp.name}`);
    });
  }
  test(`gallery — opened with ⌘B while narrow — 860x700 — ${theme}`, async ({ page }) => {
    await seed(page, theme, 860, 700);
    await page.keyboard.press("ControlOrMeta+b");
    await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
    await page.screenshot({ path: resolve(OUT, `opened-860x700-${theme}.png`) });
    await axeClean(page, "opened rail 860");
  });
}

test("compact: folds when narrow, ⌘B opens it as compact, and widening shows compact", async ({ page }) => {
  await seed(page, "dark", 860, 800, "compact");
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  await page.keyboard.press("ControlOrMeta+b");
  await expect(rail(page)).toHaveAttribute("data-density", "compact");
  await page.setViewportSize({ width: 1280, height: 800 });
  await expect(rail(page)).toHaveAttribute("data-density", "compact");
  expect(await page.evaluate(() => localStorage.getItem("aij.density"))).toBe("compact");
});

const stored = (page: Page) => page.evaluate(() => localStorage.getItem("aij.density"));

test("the density shortcut while narrow only opens and closes the list; Settings is never written", async ({ page }) => {
  await seed(page, "dark", 860, 800);
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  await page.keyboard.press("Control+d");
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  expect(await stored(page)).toBe("comfortable");
  await page.keyboard.press("Control+d");
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  expect(await stored(page)).toBe("comfortable");
});

test("a stored Minimal, narrow: ⌘B opens the list as Comfortable and closes it again; Settings keeps Minimal", async ({ page }) => {
  await seed(page, "dark", 860, 800, "minimal");
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  await page.keyboard.press("ControlOrMeta+b");
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  expect(await stored(page)).toBe("minimal");
  await page.keyboard.press("ControlOrMeta+b");
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  expect(await stored(page)).toBe("minimal");
  await page.setViewportSize({ width: 1280, height: 800 });
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
});

test("a stored Minimal, narrow: the density shortcut opens and closes the list; Settings keeps Minimal", async ({ page }) => {
  await seed(page, "dark", 860, 800, "minimal");
  await page.keyboard.press("Control+d");
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  expect(await stored(page)).toBe("minimal");
  await page.keyboard.press("Control+d");
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  expect(await stored(page)).toBe("minimal");
});

test("at full width the density shortcut still changes the setting", async ({ page }) => {
  await seed(page, "dark", 1280, 800);
  await page.keyboard.press("Control+d");
  await expect(rail(page)).toHaveAttribute("data-density", "compact");
  expect(await stored(page)).toBe("compact");
});

test("narrow: double-clicking the rail's edge closes the list it opened; Settings is never written", async ({ page }) => {
  await seed(page, "dark", 860, 800);
  await page.keyboard.press("ControlOrMeta+b");
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  await page.getByRole("separator", { name: "Resize navigator" }).dblclick();
  await expect(rail(page)).toHaveAttribute("data-density", "minimal");
  expect(await stored(page)).toBe("comfortable");
});

test("Ctrl+D typed in the terminal or a text field is the typing's, not the rail's", async ({ page }) => {
  await seed(page, "dark", 1280, 800);
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  // A text field: the rail's search.
  await page.getByRole("searchbox").first().focus();
  await page.keyboard.press("Control+d");
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  // The terminal (Ctrl+D is end-of-input there).
  await page.getByRole("button", { name: /Work Max/ }).first().click();
  await page.locator(".xterm-helper-textarea").first().focus();
  await page.keyboard.press("Control+d");
  await expect(rail(page)).toHaveAttribute("data-density", "comfortable");
  expect(await stored(page)).toBe("comfortable");
  // Elsewhere it still cycles.
  await page.locator(".xterm-helper-textarea").first().blur();
  await page.keyboard.press("Control+d");
  await expect(rail(page)).toHaveAttribute("data-density", "compact");
});

// 0.0.11 walk, Q1: a first open (nothing stored) starts the rail at its
// default width, not the 208 px minimum.
test("the rail starts at 264 px when nothing is stored", async ({ page }) => {
  await seed(page, "dark", 1280, 800);
  await expect(rail(page)).toHaveCSS("width", "264px");
});

// 0.0.11 walk: with counts on the chips ("Needs you 5", "Running 3") the
// Filters button overflowed the rail at its starting width and was cut.
for (const [name, fixture] of [["one-digit counts", allStatuses], ["two-digit counts", atScale]] as const) {
  test(`the filter row stays inside the rail — ${name}`, async ({ page }) => {
    await seed(page, "dark", 1280, 800, "comfortable", fixture);
    const filters = page.getByRole("button", { name: /^Filters/ });
    await expect(filters).toBeVisible();
    const [box, edge] = await Promise.all([filters.boundingBox(), rail(page).boundingBox()]);
    expect(box!.x + box!.width).toBeLessThanOrEqual(edge!.x + edge!.width - 8);
    for (const chip of ["Needs you", "Running", "All"]) {
      const chipBox = await page.getByRole("group", { name: "Show" }).getByRole("button", { name: new RegExp(`^${chip}`) }).boundingBox();
      expect(chipBox!.x + chipBox!.width, chip).toBeLessThanOrEqual(edge!.x + edge!.width - 8);
    }
  });
}
