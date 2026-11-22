/**
 * D-118 (AJ-E3701 / AJ-E3702, "Visual testing") gallery: Home's two columns
 * and its activity feed — two columns at 1024 (the rail as icons), 1280
 * and 1440; stacked at 1120 with the rail open (≈ 816 px of main area,
 * under the 860 px stack point: a real window size that stacks); both
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
import { placesConversations } from "../../src/preview/places";
import { allStatuses } from "./fixtures";
import { away, BUSY_AWAY, busyFeed, digestFeed, manyFeed, MOCK_T0, mockHome } from "./fixtures/home-feed";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/68-home-feed");
const T0 = 1_728_800_000_000;

const allClear: FixtureSet = { ...allStatuses, sessions: allStatuses.sessions.filter((s) => s.status === "running" && !s.attention) };
/** The typical fixture with the preview's desktop Apps open (two Claude
 * profiles, Terminal, ChatGPT), so Your apps has rows to fold. */
const withApps: FixtureSet = Object.assign({ ...allStatuses }, { conversations: placesConversations(T0) });
/** Forty sessions (the flood): the fixture's, repeated under their own ids. */
const flood: FixtureSet = {
  ...allStatuses,
  sessions: Array.from({ length: 40 }, (_, n) => {
    const base = allStatuses.sessions[n % allStatuses.sessions.length]!;
    return { ...base, id: `${base.id}-f${n}`, label: `${base.label ?? "Session"} ${n + 1}` };
  }),
};

/** The layout each width's main area gives (lib/home-layout: two columns
 * from 860 px of main area; the rail folds to 60 px icons under 1,100 px
 * and is 264 px open). */
const WIDTHS = [
  { name: "1024", width: 1024, height: 768, layout: "columns" }, // the rail as icons: ≈ 924 px of main area
  { name: "1280", width: 1280, height: 800, layout: "columns" }, // the rail open: ≈ 976
  { name: "1440", width: 1440, height: 900, layout: "columns" },
  { name: "stacked-1120-rail-open", width: 1120, height: 800, layout: "stacked" }, // the rail open: ≈ 816, the columns stack
] as const;

/** `globals`: more `window.__AIJ_*__` seams (the mock state's quota, places, messages). */
type Seams = { feed?: unknown[]; summary?: unknown; storage?: Record<string, string>; globals?: Record<string, unknown>; now?: number };

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, fixture: FixtureSet, theme: "dark" | "light", size: { width: number; height: number }, seams: Seams = {}) {
  await page.addInitScript(
    ({ fixture, theme, t0, seams }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      for (const [key, value] of Object.entries(seams.globals ?? {})) w[key] = value;
      if (seams.feed) w.__AIJ_FEED__ = seams.feed;
      if (seams.summary) w.__AIJ_SUMMARY__ = seams.summary;
      for (const [key, value] of Object.entries(seams.storage ?? {})) localStorage.setItem(key, value);
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: seams.now ?? T0, seams },
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

/** Busy's whole feed: the away divider and digest, a quoted message, a rollup. */
async function expectWholeBusyFeed(page: Page) {
  await expect(page.getByTestId("feed-away")).toBeVisible();
  await expect(page.getByTestId("feed-digest")).toBeVisible();
  await expect(page.getByTestId("feed-quote")).toBeVisible();
  await expect(page.getByTestId("feed-rollup")).toBeVisible();
  await expectNoTwice(page);
}

/** R5: what happened while away is the digest's only, never a stream
 * card as well (the 54-summary capture of CI run 43857640404 showed the
 * digest's three twice). Opens Show all first, so every line counts. */
async function expectNoTwice(page: Page) {
  const all = page.getByTestId("feed-digest-all");
  if ((await all.count()) > 0 && (await all.getAttribute("aria-expanded")) === "false") await all.click();
  const ids = (testId: string) =>
    page.getByTestId(testId).evaluateAll((nodes) => nodes.flatMap((node) => (node.getAttribute("data-feed-ids") ?? "").split(" ").filter(Boolean)));
  const digest = await ids("feed-digest-item");
  const stream = await ids("feed-card");
  expect(digest.length, "the digest has lines").toBeGreaterThan(0);
  expect(digest.filter((id) => stream.includes(id)), "no item both a card and a digest line").toEqual([]);
  if ((await all.count()) > 0) await all.click(); // back to the three
}

const STATES: Array<{ name: string; fixture: FixtureSet; seams: Seams }> = [
  { name: "all-clear", fixture: allClear, seams: {} },
  { name: "typical", fixture: allStatuses, seams: {} },
  { name: "busy", fixture: allStatuses, seams: { feed: busyFeed(T0), summary: BUSY_AWAY(T0) } },
  { name: "flood", fixture: flood, seams: {} },
];

for (const theme of ["dark", "light"] as const) {
  for (const size of WIDTHS) {
    for (const state of STATES) {
      test(`home feed — ${state.name} — ${size.name} — ${theme}`, gate("68-home-feed"), async ({ page }) => {
        await seed(page, state.fixture, theme, size, state.seams);
        // The layout follows the main area's width (AJ-E3701 R4: stacked under 860 px).
        await expect(page.getByTestId("home")).toHaveAttribute("data-layout", size.layout);
        await expect(page.getByTestId("home-side")).toBeAttached();
        const stacked = size.layout === "stacked";
        // Stacked, Your apps and Runway sit under the feed, one jump away by the head's link;
        // in two columns they are beside it and the link is gone.
        await expect(page.getByTestId("home-side-link")).toHaveCount(stacked ? 1 : 0);
        if (stacked) await expect(page.getByTestId("home-side-link")).toBeVisible();
        if (state.name === "busy") {
          await expect(page.getByTestId("feed-card").first()).toBeVisible();
          if (stacked) {
            // AJ-E3701 R4: stacked, the feed is one page (12 cards), then Show more.
            // Busy's 13 cards all came since the return, so the away
            // boundary (AJ-E3702 R5: the divider and digest sit at it) is past
            // that page: Show more reveals it, below.
            await expect(page.getByTestId("feed-show-more")).toBeVisible();
            await expect(page.getByTestId("feed-away")).toHaveCount(0);
          } else {
            await expectWholeBusyFeed(page);
          }
        }
        await page.mouse.move(0, 0);
        await page.screenshot({ path: resolve(OUT, `${state.name}-${size.name}-${theme}.png`) });
        if (state.name === "busy" && stacked) {
          await page.getByTestId("feed-show-more").click();
          await expectWholeBusyFeed(page);
        }
        await axeClean(page, `home feed ${state.name} ${size.name}`);
      });
    }
  }

  // Each right-column section, and the pinned zone, folded (kept in aij.home.fold.<id>),
  // with desktop Apps open so Your apps is there to fold.
  for (const fold of ["home-needs-now", "home-apps", "home-runway"] as const) {
    test(`home feed — ${fold} folded — 1280 — ${theme}`, async ({ page }) => {
      await seed(page, withApps, theme, { width: 1280, height: 800 });
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
    await expectNoTwice(page);
    // The pinned zone shows, so its label does (the 54-summary state).
    await expect(page.getByTestId("home-needs-now-section")).toBeVisible();
    await expect(page.getByTestId("home-needs-now-fold")).toBeInViewport({ ratio: 1 });
    // Back as Home opened: the night is the digest's, the older one under it.
    await expect(page.getByTestId("feed-card")).toHaveCount(1);
    await page.getByTestId("feed-away").scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(OUT, `away-1280-${theme}.png`) });
  });
}

/**
 * D-118 R1: in two columns only the columns scroll. The page's own
 * scroller (`home-scroll`) has nothing to scroll, each column fills the
 * height under the head to the work area's foot, and scrolling the feed
 * moves only the feed. CI run 43857640404: a row's absolutely placed words
 * escaped the columns, gave the page a scroll of its own, and a scroll
 * moved both columns up and left a gap under them.
 */
for (const size of [{ width: 1024, height: 680 }, { width: 1280, height: 800 }]) {
  test(`home feed — two columns, only the columns scroll — ${size.width}x${size.height}`, async ({ page }) => {
    await seed(page, allStatuses, "dark", size, { feed: busyFeed(T0), summary: BUSY_AWAY(T0) });
    await expect(page.getByTestId("home")).toHaveAttribute("data-layout", "columns");
    await expect(page.getByTestId("feed-card").first()).toBeVisible();
    const boxes = () =>
      page.evaluate(() => {
        const at = (id: string) => document.querySelector<HTMLElement>(`[data-testid="${id}"]`)!;
        const outer = at("home-scroll");
        const work = outer.parentElement!.closest("main")!;
        const pad = parseFloat(getComputedStyle(work).paddingBottom);
        const box = (node: HTMLElement) => node.getBoundingClientRect();
        return {
          outer: { scroll: outer.scrollHeight, client: outer.clientHeight, top: outer.scrollTop },
          foot: box(work).bottom - pad,
          main: { top: box(at("home-main")).top, bottom: box(at("home-main")).bottom, scroll: at("home-main").scrollHeight, client: at("home-main").clientHeight },
          side: { top: box(at("home-side-scroll")).top, bottom: box(at("home-side-scroll")).bottom },
          sideFirst: box(at("home-side")).top,
        };
      });
    // At rest the feed column isn't scrolled: the pinned zone shows under
    // its label, with no scrolled-edge rule over it (the 54-summary capture
    // of CI run 43857640404 had the column scrolled 36 px by its own
    // scrollIntoViewIfNeeded, the label above the view and the rule showing).
    await expect(page.getByTestId("home-main")).not.toHaveAttribute("data-scrolled");
    await expect(page.getByTestId("home-needs-now-fold")).toBeInViewport({ ratio: 1 });
    const before = await boxes();
    expect(before.outer.scroll, "the page has no scroll of its own").toBe(before.outer.client);
    expect(Math.abs(before.main.bottom - before.foot), "the feed column reaches the work area's foot").toBeLessThanOrEqual(1);
    expect(Math.abs(before.side.bottom - before.foot), "the right column reaches the work area's foot").toBeLessThanOrEqual(1);
    expect(before.main.bottom, "…which is the window's foot, less the work area's padding").toBeGreaterThan(size.height - 32);
    expect(before.main.scroll, "busy's feed is longer than its column").toBeGreaterThan(before.main.client);
    // Scroll the feed to its end: the page and the right column stay put.
    await page.getByTestId("home-main").evaluate((node) => node.scrollTo(0, node.scrollHeight));
    await page.mouse.move(size.width / 3, size.height / 2);
    await page.mouse.wheel(0, 2_000);
    await expect.poll(() => page.getByTestId("home-main").evaluate((node) => node.scrollTop)).toBeGreaterThan(0);
    const after = await boxes();
    expect(after.outer.top, "the page did not scroll").toBe(0);
    expect(after.outer.scroll).toBe(after.outer.client);
    expect(after.side, "the right column did not move").toEqual(before.side);
    expect(after.sideFirst, "nor did its contents").toBe(before.sideFirst);
    expect(after.main.top, "the feed column's box stayed under the head").toBe(before.main.top);
    expect(after.main.bottom).toBe(before.main.bottom);
  });
}

/** The toolbar's boxes: the chips' row, each chip, the picker and the ⋯,
 * with whether any of them is cut (its words wider than its box, or past
 * the toolbar's edge). */
async function toolbarBoxes(page: Page) {
  return page.getByTestId("feed-toolbar").evaluate((bar) => {
    const box = (node: Element) => {
      const r = node.getBoundingClientRect();
      return { left: r.left, right: r.right, top: r.top, bottom: r.bottom };
    };
    const at = (id: string) => bar.querySelector(`[data-testid="${id}"]`)!;
    const parts = [...bar.querySelectorAll('[data-testid^="feed-chip-"]'), at("feed-facet"), at("feed-more")];
    const edge = box(bar);
    const cut = parts
      .filter((node) => {
        const r = box(node);
        const words = node.scrollWidth > node.clientWidth + 0.5;
        return words || r.left < edge.left - 0.5 || r.right > edge.right + 0.5;
      })
      .map((node) => node.getAttribute("data-testid"));
    return { bar: edge, chips: box(at("feed-chip")), more: box(at("feed-more")), cut };
  });
}

/** D-118: the ⋯ holds the end of the chips' row down to the feed column's
 * minimum (480 px) — at 1200 with the rail open (≈ 548 px) it wrapped onto
 * a line of its own, an empty band over the first day — and nothing in
 * the toolbar is cut. */
test("home feed — the toolbar keeps the ⋯ on the chips' row — 1200 rail open and a 480 px column", async ({ page }) => {
  await seed(page, allStatuses, "dark", { width: 1200, height: 900 });
  await expect(page.getByTestId("home")).toHaveAttribute("data-layout", "columns");
  const check = async (what: string) => {
    const { bar, chips, more, cut } = await toolbarBoxes(page);
    expect(more.top, `${what}: the ⋯ starts on the chips' row`).toBeGreaterThanOrEqual(chips.top - 0.5);
    expect(more.top, `${what}: the ⋯ starts on the chips' row`).toBeLessThan(chips.bottom);
    expect(Math.abs(more.right - bar.right), `${what}: the ⋯ at the row's end`).toBeLessThanOrEqual(0.5);
    expect(cut, `${what}: nothing in the toolbar cut`).toEqual([]);
    return bar.right - bar.left;
  };
  // The rail open at 1200 leaves the feed column ≈ 548 px: under the
  // chips + picker + ⋯ on one line.
  expect(await check("1200 rail open")).toBeLessThan(576);
  // The column's minimum (minmax(480px, 1fr)); no real window gives it
  // (the columns stack first), so the feed's box is held there.
  await page.getByTestId("feed-filters").evaluate((node) => {
    (node as HTMLElement).style.width = "480px";
  });
  expect(await check("480 px column")).toBeCloseTo(480, 0);
});

/**
 * AJ-E3702's quality gate (R8): 5,000 items, each its own card, in the two
 * columns (1440 wide; stacked, the feed pages differently: one page, then
 * Show more). Reaching
 * the end of the feed column loads exactly three pages by itself, then
 * "Load more"; no request ever asks for more than one page (30); Load more
 * makes the fourth.
 */
test("home feed paging — 5,000 items: three pages by themselves, then Load more, a page a request", async ({ page }) => {
  await seed(page, allStatuses, "dark", { width: 1440, height: 900 }, { feed: manyFeed(T0, 5_000) });
  await expect(page.getByTestId("home")).toHaveAttribute("data-layout", "columns");
  const main = page.getByTestId("home-main");
  // The pages asked for, by where each starts: the first page may be read
  // again (AJ-E3712: a notices change re-reads it, as the preview's
  // stand-in makes once its first tick works Needs you out), never
  // counted as one more page.
  const asked = () => page.evaluate(() => (window as unknown as { __AIJ_FEED_CALLS__?: Array<{ cursor: string | null; limit: number }> }).__AIJ_FEED_CALLS__ ?? []);
  const pages = async () => new Set((await asked()).map((call) => call.cursor)).size;
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
  expect(await pages()).toBe(3);
  for (const call of await asked()) expect(call.limit).toBe(30);
  await page.getByTestId("feed-load-more").click();
  await expect.poll(pages).toBe(4);
  expect(Math.max(...(await asked()).map((call) => call.limit))).toBe(30);
});

/**
 * The approved mock-up's data ("mock" state, 1280x800, both themes): the
 * mock's two needs, five apps, runway and feed, for side-by-sides with
 * the design refs' mock-home-feed-<theme>.png. Read in UTC and en-US so
 * the times are the mock's. Capture only this:
 *   playwright test home-feed-gallery -g "the mock-up's data"
 */
test.describe("the mock-up's data", () => {
  test.use({ locale: "en-US", timezoneId: "UTC" });
  for (const theme of ["dark", "light"] as const) {
    test(`home feed — mock — 1280 — ${theme}`, async ({ page }) => {
      const mock = mockHome(MOCK_T0);
      await seed(page, mock.fixture, theme, { width: 1280, height: 800 }, {
        feed: mock.feed,
        summary: mock.summary,
        now: MOCK_T0,
        globals: { __AIJ_QUOTA_DISPLAY__: mock.quota, __AIJ_PLACES__: mock.places, __AIJ_MESSAGES__: mock.messages, __AIJ_NOTICES__: mock.notices },
      });
      await expect(page.getByTestId("home")).toHaveAttribute("data-layout", "columns");
      await expect(page.getByTestId("feed-card").first()).toBeVisible();
      await page.mouse.move(0, 0);
      await page.screenshot({ path: resolve(OUT, `mock-1280-${theme}.png`) });
    });
  }
});
