/**
 * D-118 (AJ-E3701 / AJ-E3702, "Visual testing") gallery: Home's two columns
 * and its activity feed — 1024, 1280 and 1440 wide, stacked at 900, both
 * themes, on the all-clear, typical, busy (every kind of feed item, a
 * rollup, the away digest) and flood fixtures; each section folded; the
 * empty feed; the paging quality gate (three pages by themselves, then
 * Load more, never more than one page a request).
 * Captures are written locally; the baselines come from CI.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./happo";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import type { FixtureSet } from "../../src/preview/fixtures";
import { allStatuses } from "./fixtures";
import { away, busyFeed, digestFeed, manyFeed } from "./fixtures/home-feed";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/68-home-feed");
const T0 = 1_728_800_000_000;

const allClear: FixtureSet = { ...allStatuses, sessions: allStatuses.sessions.filter((s) => s.status === "running" && !s.attention) };
/** Forty sessions (the flood): the fixture's, repeated under their own ids. */
const flood: FixtureSet = {
  ...allStatuses,
  sessions: Array.from({ length: 40 }, (_, n) => {
    const base = allStatuses.sessions[n % allStatuses.sessions.length]!;
    return { ...base, id: `${base.id}-f${n}`, label: `${base.label ?? "Session"} ${n + 1}` };
  }),
};

const WIDTHS = [
  { name: "1024", width: 1024, height: 768 },
  { name: "1280", width: 1280, height: 800 }, // the open rail leaves ~950 px: either layout, the capture shows which
  { name: "1440", width: 1440, height: 900 },
  { name: "stacked-900", width: 900, height: 800 }, // below ~1000 px of main area the columns stack
] as const;

type Seams = { feed?: unknown[]; summary?: unknown; storage?: Record<string, string> };

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, fixture: FixtureSet, theme: "dark" | "light", size: { width: number; height: number }, seams: Seams = {}) {
  await page.addInitScript(
    ({ fixture, theme, t0, seams }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      if (seams.feed) w.__AIJ_FEED__ = seams.feed;
      if (seams.summary) w.__AIJ_SUMMARY__ = seams.summary;
      for (const [key, value] of Object.entries(seams.storage ?? {})) localStorage.setItem(key, value);
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: T0, seams },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(size);
  await page.goto("/");
  await expect(page.getByTestId("home")).toBeVisible();
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

const STATES: Array<{ name: string; fixture: FixtureSet; seams: Seams }> = [
  { name: "all-clear", fixture: allClear, seams: {} },
  { name: "typical", fixture: allStatuses, seams: {} },
  { name: "busy", fixture: allStatuses, seams: { feed: busyFeed(T0), summary: away(T0, 9) } },
  { name: "flood", fixture: flood, seams: {} },
];

for (const theme of ["dark", "light"] as const) {
  for (const size of WIDTHS) {
    for (const state of STATES) {
      test(`home feed — ${state.name} — ${size.name} — ${theme}`, gate("68-home-feed"), async ({ page }) => {
        await seed(page, state.fixture, theme, size, state.seams);
        // The layout follows the main area's width (the open rail takes ~300 px of the window).
        const layout = size.width >= 1440 ? "columns" : size.width <= 1024 ? "stacked" : null;
        if (layout) await expect(page.getByTestId("home")).toHaveAttribute("data-layout", layout);
        await expect(page.getByTestId("home-side")).toBeAttached();
        if (layout === "stacked") await expect(page.getByTestId("home-side-link")).toBeVisible();
        if (state.name === "busy") {
          await expect(page.getByTestId("feed-away")).toBeVisible();
          await expect(page.getByTestId("feed-card").first()).toBeVisible();
          await expect(page.getByTestId("feed-quote")).toBeVisible();
          await expect(page.getByTestId("feed-rollup")).toBeVisible();
        }
        await page.mouse.move(0, 0);
        await page.screenshot({ path: resolve(OUT, `${state.name}-${size.name}-${theme}.png`) });
        await axeClean(page, `home feed ${state.name} ${size.name}`);
      });
    }
  }

  // Each right-column section, and the pinned zone, folded (kept in aij.home.fold.<id>).
  for (const fold of ["home-needs-now", "home-apps", "home-runway"] as const) {
    test(`home feed — ${fold} folded — 1280 — ${theme}`, async ({ page }) => {
      await seed(page, allStatuses, theme, { width: 1280, height: 800 });
      await page.getByTestId(`${fold}-fold`).click();
      await expect(page.getByTestId(`${fold}-fold`)).toHaveAttribute("aria-expanded", "false");
      expect(await page.evaluate((id) => localStorage.getItem(`aij.home.fold.${id}`), fold)).not.toBeNull();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `folded-${fold}-1280-${theme}.png`) });
      await axeClean(page, `home ${fold} folded`);
    });
  }

  test(`home feed — the empty feed — 1280 — ${theme}`, async ({ page }) => {
    await seed(page, allClear, theme, { width: 1280, height: 800 }, { feed: [] });
    await expect(page.getByTestId("feed-empty")).toBeVisible();
    await page.mouse.move(0, 0);
    await page.screenshot({ path: resolve(OUT, `empty-1280-${theme}.png`) });
    await axeClean(page, "home feed empty");
  });

  test(`home feed — the away digest, shown in place — 1280 — ${theme}`, async ({ page }) => {
    await seed(page, allStatuses, theme, { width: 1280, height: 800 }, { feed: digestFeed(T0), summary: away(T0, 9) });
    await expect(page.getByTestId("feed-away")).toBeVisible();
    await page.getByTestId("feed-away").scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `away-1280-${theme}.png`) });
  });
}

/**
 * AJ-E3702's quality gate (R8): 5,000 items, each its own card, in the two
 * columns (1440 wide: at 1280 the open rail leaves a main area under
 * ~1000 px, which stacks and pages differently). Reaching
 * the end of the feed column loads exactly three pages by itself, then
 * "Load more"; no request ever asks for more than one page (30); Load more
 * makes the fourth.
 */
test("home feed paging — 5,000 items: three pages by themselves, then Load more, a page a request", async ({ page }) => {
  await seed(page, allStatuses, "dark", { width: 1440, height: 900 }, { feed: manyFeed(T0, 5_000) });
  await expect(page.getByTestId("home")).toHaveAttribute("data-layout", "columns");
  const main = page.getByTestId("home-main");
  const calls = () => page.evaluate(() => ((window as unknown as { __AIJ_FEED_CALLS__?: unknown[] }).__AIJ_FEED_CALLS__ ?? []).length);
  await expect(page.getByTestId("feed-card").first()).toBeVisible();
  // Scroll to the end until the button shows (each page that lands moves the end further).
  await expect(async () => {
    await main.evaluate((node) => node.scrollTo(0, node.scrollHeight));
    await expect(page.getByTestId("feed-load-more")).toBeVisible({ timeout: 500 });
  }).toPass({ timeout: 15_000 });
  // More scrolling asks for nothing more: the three pages were the automatic ones.
  for (let n = 0; n < 4; n++) {
    await main.evaluate((node) => node.scrollTo(0, node.scrollHeight));
    await page.waitForTimeout(150);
  }
  expect(await calls()).toBe(3);
  const asked = await page.evaluate(() => (window as unknown as { __AIJ_FEED_CALLS__: Array<{ limit: number }> }).__AIJ_FEED_CALLS__);
  for (const call of asked) expect(call.limit).toBe(30);
  await page.getByTestId("feed-load-more").click();
  await expect.poll(calls).toBe(4);
  const after = await page.evaluate(() => (window as unknown as { __AIJ_FEED_CALLS__: Array<{ limit: number }> }).__AIJ_FEED_CALLS__);
  expect(Math.max(...after.map((call) => call.limit))).toBe(30);
});
