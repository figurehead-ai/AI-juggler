/**
 * ADR-0023 gallery: Settings › Advanced › Local cache. On (its size and
 * automatic limit), its caches unfolded, the size-limit menu, Clear
 * cache…'s sheet, off, a nearly full disk (a warning), and the search
 * index's row off (with its door) and on but empty; then Privacy's line about
 * it. Both themes, at 1280×800 and 1024×700; axe 0 critical/serious on
 * each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect, gate, test } from "./axe-gallery";
import { goTo, settingsPane } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/98-local-cache");
test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

const MB = 1_000_000;
/** The search index's row, with search on and nothing in it yet. */
const SEARCH = {
  key: "search-index",
  name: "Search index",
  speeds_up: "A search of every session's words answers at once.",
  slower: "The index is built again in the background; until it's done, search finds less.",
  bytes: 0,
  entries: 0,
  whole_only: true,
  on: true,
  own_switch: "Search the words of every session",
  own_switch_pane: "Privacy",
  own_switch_section: "search",
};
type Size = { width: number; height: number };
const SIZES: Size[] = [
  { width: 1280, height: 800 },
  { width: 1024, height: 700 },
];

async function open(page: Page, theme: "dark" | "light", size: Size, cache?: Record<string, unknown>, machine?: string) {
  await page.addInitScript(
    ([theme, cache, machine]) => {
      localStorage.setItem("aij.theme", theme as string);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.smoother.dismissed", JSON.stringify(["all"]));
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => 1_728_800_000_000;
      if (cache) (window as unknown as Record<string, unknown>).__AIJ_CACHE__ = cache;
      if (machine) (window as unknown as Record<string, unknown>).__AIJ_MACHINE__ = machine;
    },
    [theme, cache ?? null, machine ?? null] as const,
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(size);
  await page.goto("/");
  await expect(page.getByTestId("rail")).toBeVisible();
  await goTo(page, "Settings");
  await settingsPane(page, "Advanced");
  const section = page.locator("#settings-local-cache");
  await section.scrollIntoViewIfNeeded();
  await expect(page.getByTestId("cache-switch-row")).toBeVisible();
  await page.mouse.move(0, 0);
}

async function axe(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(
    bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`),
    `axe on ${what}`,
  ).toEqual([]);
}

async function painted(page: Page) {
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
}

for (const theme of ["dark", "light"] as const) {
  for (const size of SIZES) {
    const at = `${size.width}x${size.height}`;

    test(`Local cache, on — ${at} ${theme}`, at === "1280x800" ? gate("98-local-cache") : {}, async ({ page }) => {
      await open(page, theme, size);
      await expect(page.getByTestId("cache-switch-row")).toContainText("Using 47 MB of 1.3 GB. Subagent lists and app icons open without re-reading files.");
      await painted(page);
      await page.screenshot({ path: resolve(OUT, `on-${at}-${theme}.png`) });
      await axe(page, `local cache on ${at} ${theme}`);

      // What's in it, unfolded.
      await page.getByTestId("cache-contents-toggle").locator("summary").click();
      await expect(page.getByTestId("cache-row")).toHaveCount(3);
      // Search is off here: its row says so, with a way to its switch.
      await expect(page.getByTestId("cache-row-size").last()).toHaveText("Off");
      await expect(page.getByTestId("cache-row-door")).toHaveText("Turn it on in Privacy");
      await page.getByTestId("cache-row").last().scrollIntoViewIfNeeded();
      await painted(page);
      await page.screenshot({ path: resolve(OUT, `contents-${at}-${theme}.png`) });
      await axe(page, `its caches ${at} ${theme}`);
    });

    test(`Local cache, search on but empty — ${at} ${theme}`, async ({ page }) => {
      await open(page, theme, size, {
        on: true,
        automatic: true,
        limit_mb: null,
        budget_bytes: 1_300 * MB,
        used_bytes: 0,
        low_disk: false,
        free_bytes: 65_500 * MB,
        caches: [SEARCH],
      });
      await expect(page.getByTestId("cache-switch-row")).toContainText("Empty for now. It fills as you use AI Juggler, up to 1.3 GB.");
      await page.getByTestId("cache-contents-toggle").locator("summary").click();
      await expect(page.getByTestId("cache-row-size")).toHaveText("Empty");
      await page.getByTestId("cache-row").last().scrollIntoViewIfNeeded();
      await painted(page);
      await page.screenshot({ path: resolve(OUT, `search-empty-${at}-${theme}.png`) });
      await axe(page, `search on but empty ${at} ${theme}`);
    });

    test(`Local cache, its menus and sheet — ${at} ${theme}`, async ({ page }) => {
      await open(page, theme, size);
      // The size-limit menu.
      await page.getByRole("button", { name: "Size limit: Automatic" }).click();
      await expect(page.getByTestId("cache-limit-1000")).toBeVisible();
      await painted(page);
      await page.screenshot({ path: resolve(OUT, `limit-menu-${at}-${theme}.png`) });
      await axe(page, `the size-limit menu ${at} ${theme}`);
      await page.keyboard.press("Escape");

      // Clear cache…: what gets slower, before anything goes.
      await page.getByTestId("cache-clear").click();
      await expect(page.getByTestId("cache-clear-sheet")).toBeVisible();
      await painted(page);
      await page.screenshot({ path: resolve(OUT, `clear-sheet-${at}-${theme}.png`) });
      await axe(page, `clear cache sheet ${at} ${theme}`);
    });

    test(`Local cache, off — ${at} ${theme}`, async ({ page }) => {
      await open(page, theme, size, {
        on: false,
        automatic: true,
        limit_mb: null,
        budget_bytes: 1_300 * MB,
        used_bytes: 0,
        low_disk: false,
        free_bytes: 65_500 * MB,
        caches: [{ ...SEARCH, on: false }],
      });
      await expect(page.getByTestId("cache-switch-row")).toContainText("Off: nothing is kept");
      await painted(page);
      await page.screenshot({ path: resolve(OUT, `off-${at}-${theme}.png`) });
      await axe(page, `local cache off ${at} ${theme}`);
    });

    test(`Local cache, nearly full disk — ${at} ${theme}`, async ({ page }) => {
      await open(page, theme, size, {
        on: true,
        automatic: true,
        limit_mb: null,
        budget_bytes: 64 * MB,
        used_bytes: 12 * MB,
        low_disk: true,
        free_bytes: 3_100 * MB,
        caches: [
          {
            key: "subagents",
            name: "Subagent lists",
            speeds_up: "A session's Subagents tab opens at once, even with hundreds of subagents.",
            slower: "Each session's Subagents tab reads its subagents' files again the first time it opens.",
            bytes: 11 * MB,
            entries: 31,
            whole_only: false,
            on: true,
            own_switch: null,
            own_switch_pane: null,
            own_switch_section: null,
          },
          {
            key: "icons",
            name: "App icons",
            speeds_up: "The apps' icons show at once, without asking the system for them again.",
            slower: "Each app's icon is read from the app again the first time it shows.",
            bytes: 1 * MB,
            entries: 38,
            whole_only: false,
            on: true,
            own_switch: null,
            own_switch_pane: null,
            own_switch_section: null,
          },
        ],
      }, "disk");
      await expect(page.getByTestId("cache-switch-row").locator('[data-tone="warning"]')).toContainText("Your disk is nearly full");
      // This Mac (in the header from 1280) says the same disk is short,
      // never "Plenty of room".
      await expect(page.getByText("Plenty of room")).toHaveCount(0);
      await painted(page);
      await page.screenshot({ path: resolve(OUT, `low-disk-${at}-${theme}.png`) });
      await axe(page, `nearly full disk ${at} ${theme}`);
    });
  }

  test(`Privacy says what it keeps — ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
    }, theme);
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await goTo(page, "Settings");
    await settingsPane(page, "Privacy");
    await expect(page.getByTestId("privacy-reads-cache")).toBeVisible();
    await page.mouse.move(0, 0);
    await painted(page);
    await page.screenshot({ path: resolve(OUT, `privacy-${theme}.png`) });
    // Its door lands on the section.
    await page.getByTestId("privacy-local-cache").click();
    await expect(page.getByTestId("settings-pane-advanced")).toBeVisible();
    await expect(page.getByTestId("cache-switch-row")).toBeInViewport();
  });
}
