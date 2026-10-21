/**
 * AJ-E1702 gallery: pools — the rail grouped by pool (fold, name, count;
 * an empty pool keeps its place), a session's menu (Move to…, New
 * pool…), the New pool sheet, a pool's sheet (roles, what each role may
 * do, delete with its consequence said first), "All sessions" (no
 * rename or delete; what agents will see, D-33), the pool filter in the
 * rail and on Home. Both themes, axe 0 critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, openFilters } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/82-pools");
const T0 = 1_728_800_000_000;

/** A planner and two builders in "Checkout redesign", the planner as
 * lead; "Docs" made, nobody in it yet. */
export const POOLS = {
  pools: [
    { id: "pool-checkout", name: "Checkout redesign" },
    { id: "pool-docs", name: "Docs" },
  ],
  members: [
    { session_id: "sess-1", pool_id: "pool-checkout", role: "lead" },
    { session_id: "sess-2", pool_id: "pool-checkout", role: "member" },
    { session_id: "sess-4", pool_id: "pool-checkout", role: "member" },
  ],
};

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", pools: unknown = POOLS) {
  await page.addInitScript(
    ({ fixture, theme, t0, pools }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      if (pools) w.__AIJ_POOLS__ = pools;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: structuredClone(allStatuses), theme, t0: T0, pools },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.getByTestId("home")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

async function groupByPool(page: Page) {
  await openFilters(page);
  await page.getByRole("combobox", { name: "Group by" }).selectOption("pool");
  await page.getByRole("button", { name: /^Filters/ }).click();
}

async function rowMenu(page: Page, id: string) {
  const row = page.locator(`[data-instance-id="${id}"]`).locator("..");
  await row.hover();
  await row.getByRole("button", { name: /^Row actions/ }).click();
  return page.getByRole("menu");
}

for (const theme of ["dark", "light"] as const) {
  test(`the rail by pool — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await groupByPool(page);
    const headers = page.getByTestId("rail-pool");
    // Needs-me order decides which group comes first, as with projects.
    await expect(headers).toHaveText(["All sessions9", "Checkout redesign3", "Docs0"]);
    await expect(page.getByTestId("rail-new-pool")).toBeVisible();
    await page.mouse.move(0, 0);
    await page.getByTestId("rail").screenshot({ path: resolve(OUT, `rail-by-pool-${theme}.png`) });
    await page.screenshot({ path: resolve(OUT, `rail-by-pool-1280x800-${theme}.png`) });
    await axeClean(page, "the rail by pool");

    await page.getByRole("button", { name: "Fold Checkout redesign" }).click();
    await expect(page.locator('[data-instance-id="sess-1"]')).toBeHidden();
    await page.getByTestId("rail").screenshot({ path: resolve(OUT, `rail-pool-folded-${theme}.png`) });
  });

  test(`a session's menu — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    const menu = await rowMenu(page, "sess-3");
    await expect(menu.getByTestId("menu-move-pool")).toHaveText(["Checkout redesign", "Docs"]);
    await expect(menu.getByTestId("menu-new-pool")).toHaveText("New pool…");
    await page.screenshot({ path: resolve(OUT, `row-menu-1280x800-${theme}.png`) });
    await axeClean(page, "the row menu");
    await menu.getByRole("group", { name: "Move to pool" }).getByRole("menuitem", { name: "Docs" }).click();
    await groupByPool(page);
    await expect(page.getByTestId("rail-pool")).toContainText(["Docs1"]);
  });

  test(`new pool — ${theme}`, async ({ page }) => {
    await seed(page, theme, null);
    const menu = await rowMenu(page, "sess-2");
    await expect(menu.getByTestId("menu-move-pool")).toHaveCount(0);
    await menu.getByRole("menuitem", { name: "New pool…" }).click();
    const form = page.getByTestId("pool-create");
    await expect(page.getByTestId("pool-create-name")).toBeFocused();
    await expect(page.getByTestId("pool-create-session")).toContainText("goes in it.");
    await page.getByTestId("pool-create-name").fill("all sessions");
    await page.getByTestId("pool-create-save").click();
    await expect(form.getByRole("alert")).toHaveText("There's already a pool called «All sessions» — pick another name");
    await page.screenshot({ path: resolve(OUT, `new-pool-refused-1280x800-${theme}.png`) });
    await axeClean(page, "new pool (refused)");
    await page.getByTestId("pool-create-name").fill("Checkout redesign");
    await page.getByTestId("pool-create-save").click();
    // The new pool opens, the session in it; the rail groups by pool.
    await expect(page.getByTestId("pool-sheet")).toBeVisible();
    await expect(page.getByTestId("pool-member")).toHaveCount(1);
    await page.getByTestId("pool-done").click();
    await expect(page.getByTestId("rail-pool")).toHaveText(["All sessions11", "Checkout redesign1"]);
    await page.getByTestId("rail").screenshot({ path: resolve(OUT, `first-pool-rail-${theme}.png`) });
  });

  test(`a pool's sheet — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await groupByPool(page);
    await page.getByTestId("rail-pool").filter({ hasText: "Checkout redesign" }).click();
    const sheet = page.getByTestId("pool-sheet");
    expect(await sheet.getByTestId("pool-member-role").evaluateAll((selects) => selects.map((select) => (select as HTMLSelectElement).value))).toEqual(["lead", "member", "member"]);
    await expect(sheet.getByTestId("pool-member")).toHaveCount(3);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `pool-sheet-1280x800-${theme}.png`) });
    await axeClean(page, "a pool's sheet");

    await sheet.getByText("What each role may do").click();
    await expect(sheet.getByRole("checkbox", { name: "Observer: See who's here and what they're doing" })).toBeDisabled();
    await sheet.getByRole("checkbox", { name: "Member: Message the whole pool" }).check();
    await expect(sheet.getByRole("checkbox", { name: "Member: Message the whole pool" })).toBeChecked();
    await sheet.getByRole("checkbox", { name: "Member: Message the whole pool" }).scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `pool-grants-1280x800-${theme}.png`) });
    await axeClean(page, "what each role may do");

    await sheet.getByTestId("pool-delete").click();
    await expect(sheet.getByTestId("pool-delete-confirm")).toContainText("Its 3 sessions go back to All sessions; nothing else changes.");
    await page.screenshot({ path: resolve(OUT, `pool-delete-1280x800-${theme}.png`) });
    await axeClean(page, "delete a pool");
    await sheet.getByTestId("pool-delete-yes").click();
    await expect(page.getByTestId("rail-pool")).toHaveText(["All sessions12", "Docs0"]);
  });

  test(`all sessions — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await groupByPool(page);
    await page.getByTestId("rail-pool").filter({ hasText: "All sessions" }).click();
    const sheet = page.getByTestId("pool-sheet");
    await expect(sheet.getByTestId("pool-scope")).toHaveText("Agents here will see only sessions on their own project and account.");
    await expect(sheet.getByTestId("pool-delete")).toHaveCount(0);
    await expect(sheet.getByTestId("pool-name")).toHaveCount(0);
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `all-sessions-1280x800-${theme}.png`) });
    await axeClean(page, "All sessions");
  });

  test(`the pool filter — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openFilters(page);
    await page.getByRole("combobox", { name: "Filter by pool" }).selectOption("pool-checkout");
    await expect(page.locator("[data-instance-id]:visible")).toHaveCount(3);
    await page.getByTestId("rail").screenshot({ path: resolve(OUT, `rail-pool-filter-${theme}.png`) });
    await axeClean(page, "the rail's pool filter");

    await page.getByTestId("home-pool-filter").selectOption("pool-checkout");
    await expect(page.getByTestId("home-pool-filter")).toHaveValue("pool-checkout");
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `home-pool-filter-1280x800-${theme}.png`) });
    await axeClean(page, "Home's pool filter");
  });
}
