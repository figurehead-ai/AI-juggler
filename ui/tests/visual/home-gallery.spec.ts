/**
 * AJ-E1014 (D-25) gallery: Home in each of its states, the rail's Filters
 * open, and the More menu — each scanned with axe (0 critical/serious).
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./happo";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, honestEmpty, openFilters, type FixtureSet } from "./fixtures";
import { HOME_OUT_USE, HOME_OUT_VIEWPORT, showHomeOut } from "./fixtures/home-out";
import happoConfig from "../../happo.config";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/67-home");
const T0 = 1_728_800_000_000;

const allClear: FixtureSet = {
  ...allStatuses,
  sessions: allStatuses.sessions.filter((s) => s.status === "running" && !s.attention),
};
const nothingYet: FixtureSet = { ...allStatuses, sessions: [] };

const STATES: Array<{ name: string; fixture: FixtureSet; expect: string }> = [
  { name: "needs-you", fixture: allStatuses, expect: "need you" },
  { name: "all-clear", fixture: allClear, expect: "All clear" },
  { name: "no-sessions", fixture: nothingYet, expect: "Ready when you are" },
  { name: "no-accounts", fixture: honestEmpty, expect: "All your AI coding accounts, in one calm place." },
];
const VIEWPORTS = [
  { name: "1280x800", width: 1280, height: 800 },
  { name: "390x844", width: 390, height: 844 },
] as const;

/** Happo's targets (happo.config.ts) at their sizes. Happo renders a
 * snapshot's DOM at every target's size, but Home lays itself out in
 * script by its measured width (lib/home-layout: two columns or stacked,
 * and the rail folds to icons under 1,100 px), so a DOM taken at one width
 * shows that width's layout at every other: 1280's two columns squeezed
 * into 1024, cut off. Each target gets its own snapshot, taken at its size
 * and sent to it only. */
const HAPPO_TARGETS = Object.entries(happoConfig.targets ?? {}).map(([name, target]) => {
  const [width, height] = String((target as { viewport?: string }).viewport ?? "").split("x").map(Number);
  if (!width || !height) throw new Error(`happo.config.ts: target ${name} has no WxH viewport`);
  return { name, width, height };
});

/** Two frames: the resize's layout (the rail's fold, Home's columns) is drawn. */
async function settled(page: Page) {
  await page.evaluate(() => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done()))));
}

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, fixture: FixtureSet, theme: "dark" | "light", vp: { width: number; height: number }) {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: vp.width, height: vp.height });
  await page.goto("/");
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  for (const vp of VIEWPORTS) {
    for (const state of STATES) {
      test(`home — ${state.name} — ${vp.name} — ${theme}`, gate("67-home"), async ({ page, happoScreenshot }) => {
        await seed(page, state.fixture, theme, vp);
        await expect(page.getByRole("main")).toContainText(state.expect);
        await page.screenshot({ path: resolve(OUT, `${state.name}-${vp.name}-${theme}.png`) });
        await axeClean(page, `home ${state.name}`);
        // Each Happo target from a page laid out at its own size (above).
        if (vp.name === "1280x800") {
          for (const target of HAPPO_TARGETS) {
            await page.setViewportSize({ width: target.width, height: target.height });
            await settled(page);
            await expect(page.getByRole("main")).toContainText(state.expect);
            if (state.name !== "no-accounts") await expect(page.getByTestId("home-apps-section")).toBeAttached();
            await happoScreenshot(page.locator("body"), { component: "Home", variant: `${state.name}-${theme}-${target.name}`, targets: [target.name] });
          }
        }
      });
    }
  }

  // AJ-E1524: the welcome for a new install, 390 px to 1440 px, both
  // densities, and with How it works open.
  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x680", width: 1024, height: 680 },
  ]) {
    test(`home — welcome — ${vp.name} — ${theme}`, async ({ page }) => {
      await seed(page, honestEmpty, theme, vp);
      await expect(page.getByTestId("welcome-found")).not.toContainText("Looking");
      await page.screenshot({ path: resolve(OUT, `welcome-${vp.name}-${theme}.png`) });
      await axeClean(page, "home welcome");
    });
  }

  test(`home — welcome, compact rail — ${theme}`, async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("aij.density", "compact"));
    await seed(page, honestEmpty, theme, { width: 1280, height: 800 });
    await expect(page.getByTestId("welcome-found")).not.toContainText("Looking");
    await page.screenshot({ path: resolve(OUT, `welcome-compact-1280x800-${theme}.png`) });
  });

  test(`home — welcome, How it works open — ${theme}`, async ({ page }) => {
    await seed(page, honestEmpty, theme, { width: 1280, height: 800 });
    await page.getByTestId("welcome-how-toggle").click();
    await expect(page.getByTestId("welcome-how").locator("ol > li")).toHaveCount(3);
    await page.getByTestId("welcome-how").locator("ol > li").last().scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `welcome-how-1280x800-${theme}.png`) });
    await axeClean(page, "home welcome, how it works");
  });

  test(`rail — Filters open — ${theme}`, async ({ page }) => {
    await seed(page, allStatuses, theme, { width: 1280, height: 800 });
    await openFilters(page);
    await page.screenshot({ path: resolve(OUT, `rail-filters-open-1280x800-${theme}.png`) });
    await axeClean(page, "rail filters");
  });

  test(`header — your menu — ${theme}`, async ({ page }) => {
    await seed(page, allStatuses, theme, { width: 1280, height: 800 });
    // AJ-E2507: the old More's screens are in your menu, behind the avatar.
    await page.getByTestId("user-menu-button").click();
    const menu = page.getByRole("menu", { name: "Your menu" });
    // Every item is reachable — the menu paints above the page (D-25 fix).
    for (const name of ["This Mac", "Processes", "Settings"]) {
      await expect(menu.getByRole("menuitem", { name: new RegExp(`^${name}\\b`) })).toBeInViewport({ ratio: 1 });
      // AJ-E2106: each screen with its icon, decoration only.
      await expect(menu.getByRole("menuitem", { name: new RegExp(`^${name}\\b`) }).locator('svg[aria-hidden="true"]')).toHaveCount(1);
    }
    await page.screenshot({ path: resolve(OUT, `more-menu-1280x800-${theme}.png`) });
    await axeClean(page, "your menu");
    await page.mouse.click(5, 790);
    await expect(menu).toHaveCount(0);
  });
}

/**
 * D-118: the right column is Your apps (running, then not running), then
 * Runway by account. 2026-11-16 (Happo's chrome-large Home had no Your
 * apps): every App is in it, the CLI Apps too, so someone with an Account
 * always has it, whether their sessions run in AI Juggler's own terminal
 * (needs-you, all-clear) or none runs (no-sessions). At the window's real
 * sizes, by the 860 px stack point (AJ-E3701 R4): beside the feed, in
 * sight, at 1024 (its minimum, the rail as icons, ≈ 924 px of main area),
 * 1200 with the rail open (≈ 896) and 1440; under the feed, reached by
 * "Apps & runway ↓", at 1120 with the rail open (≈ 816, stacked).
 */
const YOUR_APPS_SIZES = [
  { name: "1024x680", width: 1024, height: 680, rail: "icons", layout: "columns" },
  { name: "1120x800", width: 1120, height: 800, rail: "open", layout: "stacked" },
  { name: "1200x900", width: 1200, height: 900, rail: "open", layout: "columns" },
  { name: "1440x900", width: 1440, height: 900, rail: "open", layout: "columns" },
] as const;
for (const theme of ["dark", "light"] as const) {
  for (const state of STATES.filter((one) => one.name !== "no-accounts")) {
    test(`home — Your apps beside the feed at 1024, 1200 (rail open) and 1440, under it at 1120 — ${state.name} — ${theme}`, async ({ page }) => {
      await seed(page, state.fixture, theme, YOUR_APPS_SIZES[0]);
      await expect(page.getByRole("main")).toContainText(state.expect);
      for (const size of YOUR_APPS_SIZES) {
        await page.setViewportSize({ width: size.width, height: size.height });
        await settled(page);
        const rail = await page.getByTestId("rail").evaluate((node) => node.getBoundingClientRect().width);
        expect(rail, `the rail at ${size.name}`).toBeGreaterThan(size.rail === "open" ? 200 : 0);
        if (size.rail === "icons") expect(rail, `the rail at ${size.name}`).toBeLessThan(120);
        const apps = page.getByTestId("home-apps-section");
        await expect(apps, `Your apps at ${size.name}`).toBeVisible();
        await expect(apps.getByTestId("home-apps-fold")).toHaveText(/^Your apps\s*\d+/);
        await expect(apps.locator("[data-testid='home-app-tile'], [data-testid='home-app-cli'], [data-testid='home-app-kept'], [data-testid='home-app-closed']").first()).toBeVisible();
        // Your apps, then Runway by account, in the one right column.
        const order = await page.getByTestId("home-side").evaluate((side) => {
          const apps = side.querySelector("[data-testid='home-apps-section']");
          const runway = side.querySelector("[data-testid='home-runway']");
          return apps !== null && runway !== null && (apps.compareDocumentPosition(runway) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
        });
        expect(order, `Your apps before Runway at ${size.name}`).toBe(true);
        await expect(page.getByTestId("home"), `the layout at ${size.name}`).toHaveAttribute("data-layout", size.layout);
        if (size.layout === "columns") {
          await expect(page.getByTestId("home-side-link"), `no Apps & runway link at ${size.name}`).toHaveCount(0);
          await expect(apps, `Your apps in sight at ${size.name}`).toBeInViewport();
        } else {
          // Stacked (E37 R4): under the feed, one jump away by the head's link.
          await page.getByTestId("home-side-link").click();
          await expect(apps, `Your apps reached at ${size.name}`).toBeInViewport();
          await page.getByTestId("home-scroll").evaluate((box) => box.scrollTo(0, 0)).catch(() => undefined);
        }
      }
    });
  }
}

/**
 * 0.4.5 (the maintainer's 0.4.4 finding): The runway by account with an Account out of room whose reset reads at its
 * longest (fixtures/home-out.ts). Its mark wraps inside the card, its
 * icon beside the first line; a mark that fits ("Low · 7% left") is drawn
 * as before. Both themes in one gate state.
 */
test.describe("home — your apps, an Account out, at the narrowest cards", () => {
  test.use(HOME_OUT_USE);
  test("home — your apps, an Account out, at the narrowest cards — both themes", gate("67-home-apps-out"), async ({ page }) => {
    const size = `${HOME_OUT_VIEWPORT.width}x${HOME_OUT_VIEWPORT.height}`;
    for (const theme of ["dark", "light"] as const) {
      await showHomeOut(page, theme);
      // D-118: the room marks live in the runway rows now (one per Account).
      await expect(page.getByTestId("home-runway-account")).toHaveCount(3);
      await expect(page.getByTestId("home-runway-out")).toHaveCount(2);
      await expect(page.getByTestId("home-runway-account").filter({ hasText: "7% left" })).toHaveCount(1);
      await page.mouse.move(0, 0);
      await page.getByTestId("home-runway").screenshot({ path: resolve(OUT, `apps-out-${size}-${theme}.png`) });
      await axeClean(page, `home your apps, out, ${theme}`);
    }
  });
});

// 0.0.6 dogfood: the rail's "?" opened a legend that was clipped out of
// sight — the suites checked it opened, never that it could be seen.
// AJ-E403 Req.6: the same legend from the rail footer and from Home's
// header (its "All clear" explainer since Overview folded into Home).
for (const theme of ["dark", "light"] as const) {
  for (const where of ["rail", "home"] as const) {
    test(`legend — "?" on ${where} shows it on top, in view, and closes — ${theme}`, async ({ page }) => {
      await seed(page, allStatuses, theme, { width: 1280, height: 800 });
      await page.getByTestId(where).getByRole("button", { name: "Indicator legend" }).click();
      const legend = page.getByRole("dialog", { name: "What the icons mean" });
      await expect(legend).toBeInViewport({ ratio: 1 });
      // Nothing paints over it: the topmost element at its centre is inside it.
      const onTop = await legend.evaluate((node) => {
        const box = node.getBoundingClientRect();
        const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
        return hit !== null && node.contains(hit);
      });
      expect(onTop, "the legend is covered by another layer").toBe(true);
      await expect(legend).toContainText("Asks you"); // AJ-E2104 Req.3: the rail's words
      await page.screenshot({ path: resolve(OUT, where === "rail" ? `legend-1280x800-${theme}.png` : `legend-home-1280x800-${theme}.png`) });
      await axeClean(page, `legend from ${where}`);
      await page.keyboard.press("Escape");
      await expect(legend).toHaveCount(0);
      await expect(page.getByTestId(where).getByRole("button", { name: "Indicator legend" })).toBeFocused();
    });
  }
}

