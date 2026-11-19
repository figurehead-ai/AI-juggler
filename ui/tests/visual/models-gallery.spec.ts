/**
 * AJ-E3812 ("Visual testing"): Settings › AI › Models in every state of
 * the approved mock-up — Apple recommended, a download recommended,
 * downloading, the second-model approval, not enough free disk, the list
 * unread (offline), installed with a withdrawn model (its Remove
 * confirmation and the Chat picker open), and no list on a Mac with
 * nothing to recommend — both themes, at 1280 and at a narrow 900, each
 * frame tall enough to hold the whole page; then axe on each (0 critical
 * or serious). Mocked data only: nothing is downloaded.
 *
 * Captures go to `AIJ_MODELS_OUT` (default: the evidence gallery).
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { STATES, type ModelsState } from "../../src/preview/ai-models";
import { allStatuses, goTo } from "./fixtures";

const OUT = process.env.AIJ_MODELS_OUT ?? resolve(process.cwd(), "../docs/qa/evidence/_gallery/76-ai-models");
const T0 = Date.UTC(2026, 10, 19, 12);

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function open(page: Page, state: ModelsState, theme: "dark" | "light", width: number): Promise<void> {
  await page.addInitScript(
    ({ fixture, theme: t, t0, state: s }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_AI_CATALOG__ = s.catalog;
      w.__AIJ_AI_MODELS__ = s.models;
      w.__AIJ_AI_JOBS__ = s.jobs;
      if (s.start) w.__AIJ_AI_START__ = s.start;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      localStorage.setItem("aij.settings.pane", "ai-models");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture: allStatuses, theme, t0: T0, state },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height: 800 });
  await page.goto("/");
  await goTo(page, "Settings");
  await expect(page.getByTestId("models-screen")).toBeVisible();
  await expect(page.getByTestId("models-skeleton")).toHaveCount(0);
}

/** Makes the window as tall as the page (and `room` more, for a menu
 * near its foot), so one frame holds all of it. */
async function wholePage(page: Page, width: number, room = 0): Promise<void> {
  const extra = await page.evaluate(() => {
    const scroller = document.querySelector("[data-testid=settings-screen]")?.closest(".overflow-y-auto");
    return scroller ? scroller.scrollHeight - scroller.clientHeight : 0;
  });
  const height = page.viewportSize()?.height ?? 800;
  await page.setViewportSize({ width, height: height + Math.max(extra, 0) + room });
}

/** Back to the page's top (a click scrolled it), as the mock frames it. */
async function toTop(page: Page): Promise<void> {
  await page.evaluate(() => document.querySelector("[data-testid=settings-screen]")?.closest(".overflow-y-auto")?.scrollTo(0, 0));
}

async function axeClean(page: Page, what: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`), `axe on ${what}`).toEqual([]);
}

/** What each state needs clicked before its capture. */
const BEFORE: Record<string, (page: Page) => Promise<void>> = {
  approval: async (page) => {
    await page.locator("[data-model='granite-4-1-3b-q4km']").getByTestId("model-download").click();
    await expect(page.getByTestId("models-approval")).toBeVisible();
  },
  models: async (page) => {
    await page.locator("[data-model='gemma-4-e2b-q4-0']").getByTestId("model-not-yet").locator("button").first().click();
  },
  installed: async (page) => {
    await page.getByTestId("models-remove").nth(1).click();
  },
};

/** What opens once the frame has its height (a menu places itself in
 * the window it opens in). */
const AFTER: Record<string, (page: Page) => Promise<void>> = {
  installed: async (page) => {
    await page.getByTestId("models-pick-chat").click();
  },
};

/** The approval is a dialog over a fixed window, as in the mock. */
const FIXED = new Set(["approval", "downloading"]);

for (const theme of ["dark", "light"] as const) {
  for (const [name, state] of Object.entries(STATES)) {
    for (const width of [1280, 900]) {
      test(`models ${name} — ${theme} — ${width}`, async ({ page }) => {
        await open(page, state, theme, width);
        if (!FIXED.has(name)) await wholePage(page, width);
        await BEFORE[name]?.(page);
        if (!FIXED.has(name)) await wholePage(page, width, name === "installed" ? 260 : 0);
        if (name === "approval") await toTop(page);
        await AFTER[name]?.(page);
        await page.screenshot({ path: `${OUT}/${name}-${theme}-${width}.png` });
        if (width === 1280) await axeClean(page, `${name} ${theme}`);
      });
    }
  }
}
