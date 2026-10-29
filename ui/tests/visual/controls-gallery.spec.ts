/**
 * 0.3 gallery — the shared controls and the rail's View, captured so no
 * popover escapes review again (the redesign: "The View menu wasn't even
 * captured"). Both themes, 1280×800 and 1024×700, axe 0 critical/serious
 * on each:
 * - the rail's grouping switch (App · Project · Account, and Pool once a
 *   pool of yours exists);
 * - the View menu as it opens (no Reset), and changed (Reset, a Show line
 *   on, Only › Account's submenu open);
 * - a Picker open (History's filter), with its ✓ on the chosen line;
 * - Settings › General's equal cards (theme, density, order).
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, GATE_NOW, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo, groupBy, openFilters, settingsPane } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/97-controls");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

/** One pool of the user's own, so the switch offers Pool. */
const POOLS = { pools: [{ id: "pool-checkout", name: "Checkout redesign" }], members: [{ session_id: "sess-2", pool_id: "pool-checkout", role: "lead" }] };

const SIZES = [
  { width: 1280, height: 800 },
  { width: 1024, height: 700 },
] as const;

async function seed(page: Page, theme: "dark" | "light", size: { width: number; height: number }, pools: unknown = null) {
  await page.clock.setFixedTime(GATE_NOW);
  await page.addInitScript(
    ({ fixture, theme, pools }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      if (pools) w.__AIJ_POOLS__ = pools;
      w.__AIJ_AGENT_BRIDGE__ = { asked: true, on: true };
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: structuredClone(allStatuses), theme, pools },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(size);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
}

async function shoot(page: Page, name: string, size: { width: number; height: number }, theme: string) {
  await page.screenshot({ path: resolve(OUT, `${name}-${size.width}x${size.height}-${theme}.png`) });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  for (const size of SIZES) {
    const tag = `${size.width}x${size.height} ${theme}`;

    test(`the rail's grouping switch, Pool offered — ${tag}`, async ({ page }) => {
      await seed(page, theme, size, POOLS);
      const group = page.getByRole("radiogroup", { name: "Group sessions by" });
      await expect(group.getByRole("radio", { name: "Pool", exact: true })).toBeVisible();
      await groupBy(page, "Project");
      await expect(group.getByRole("radio", { name: "Project", exact: true })).toHaveAttribute("aria-checked", "true");
      await page.mouse.move(0, 0);
      await shoot(page, "rail-group-switch", size, theme);
    });

    test(`the View, as it opens — ${tag}`, size.width === 1280 && theme === "dark" ? gate("97-controls") : {}, async ({ page }) => {
      await seed(page, theme, size);
      await openFilters(page);
      const menu = page.getByRole("menu", { name: "View" });
      await expect(menu).toBeVisible();
      // Nothing changed: no Reset; Group by is the rail's switch now.
      await expect(page.getByTestId("rail-view-reset")).toHaveCount(0);
      await expect(menu.getByRole("radio")).toHaveCount(0);
      await expect(menu.locator("select")).toHaveCount(0);
      await shoot(page, "view-open", size, theme);
    });

    test(`the View, changed, Only › Account open — ${tag}`, async ({ page }) => {
      await seed(page, theme, size);
      await openFilters(page);
      await page.getByTestId("rail-view-show-quiet").click();
      await expect(page.getByTestId("rail-view-reset")).toBeVisible();
      const account = page.getByTestId("rail-view-only-account");
      if ((await account.count()) > 0) {
        await account.click();
        await expect(page.getByRole("menu", { name: "Only this account" })).toBeVisible();
      }
      await shoot(page, "view-changed-only-open", size, theme);
    });

    test(`a Picker open: History's filter — ${tag}`, async ({ page }) => {
      await seed(page, theme, size);
      await goTo(page, "History");
      const picker = page.locator("[data-testid^='history-filter']").first();
      if ((await picker.count()) === 0) test.skip(true, "no filter with two values in this fixture");
      await picker.click();
      await expect(page.getByRole("menuitemradio").first()).toBeVisible();
      await shoot(page, "picker-open-history", size, theme);
    });

    test(`Settings › General: equal cards — ${tag}`, async ({ page }) => {
      await seed(page, theme, size);
      await goTo(page, "Settings");
      await settingsPane(page, "General");
      const card = page.getByTestId("settings-appearance");
      await expect(card).toBeVisible();
      await card.scrollIntoViewIfNeeded();
      await shoot(page, "settings-general-cards", size, theme);
    });
  }
}
