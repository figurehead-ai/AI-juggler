/**
 * D-136 ("Visual testing", AJ-E3804..E3808, E3815, E3819): Bot in every
 * state of the approved mock-up (`?s=<state>`), both themes, at 1280 and
 * 1024; then axe on each (0 critical or serious). Mocked data only:
 * nothing is looked up, written or downloaded.
 *
 * Captures go to `AIJ_BOT_OUT` when it names a folder; without it (the
 * a11y gate in CI) the states are walked and scanned, nothing written.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { BOT_NOW, STATES, type BotPreviewState } from "../../src/preview/bot";
import { allStatuses, goTo } from "./fixtures";

const OUT = process.env.AIJ_BOT_OUT ? resolve(process.env.AIJ_BOT_OUT) : null;

test.beforeAll(() => {
  if (OUT) mkdirSync(OUT, { recursive: true });
});

async function open(page: Page, state: BotPreviewState, theme: "dark" | "light", width: number): Promise<void> {
  await page.addInitScript(
    ({ fixture, theme: t, now, state: s }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_BOT__ = JSON.parse(JSON.stringify(s.bot));
      w.__AIJ_AI_JOBS__ = s.jobs;
      if (s.catalog) w.__AIJ_AI_CATALOG__ = s.catalog;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => now;
    },
    { fixture: allStatuses, theme, now: BOT_NOW, state },
  );
  if (state.watches) {
    await page.addInitScript((watches) => {
      (window as unknown as Record<string, unknown>).__AIJ_WATCHES__ = watches;
    }, state.watches);
  }
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height: 860 });
  await page.goto("/");
  await expect(page.getByTestId("header-bot")).toBeVisible();
  if (state.open !== "closed") {
    await page.getByTestId("header-bot").click();
    await expect(page.getByTestId("bot-panel")).toBeVisible();
  }
  if (state.open === "page") {
    await page.getByTestId("bot-expand").click();
    await expect(page.getByTestId("bot-page")).toBeVisible();
    if (state.tab === "memory") await page.getByRole("radio", { name: /What Bot remembers/ }).click();
  }
}

async function axeClean(page: Page, what: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`), `axe on ${what}`).toEqual([]);
}

/** What each state needs done before its capture. */
const BEFORE: Record<string, (page: Page) => Promise<void>> = {
  icon: async (page) => {
    await page.getByTestId("header-bot").focus();
  },
  narrow: async () => undefined,
  lookup: async (page) => {
    await page.getByTestId("bot-row").first().hover();
  },
  scope: async (page) => {
    await page.getByTestId("bot-scope-x").hover();
  },
  doors: async (page) => {
    const first = page.getByTestId("rail-open-now").locator("xpath=following::*[@data-instance-id][1]");
    if (await first.count()) await first.click();
    else await page.locator("[data-testid='rail'] [data-instance-id]").first().click();
    await expect(page.getByTestId("page-type")).toHaveText("Session");
    await page.getByTestId("ask-bot").first().click();
    await expect(page.getByTestId("bot-scope-chip")).toBeVisible();
  },
  dontknow: async (page) => {
    await page.getByTestId("bot-plus").click();
  },
  beta: async (page) => {
    await page.getByTestId("bot-how-made").click();
  },
  watch: async (page) => {
    await page.getByTestId("bot-row").nth(1).hover();
    await page.getByTestId("bot-row-watch").nth(1).click();
    await expect(page.getByTestId("bot-watch-echo")).toContainText("web");
  },
  remember: async (page) => {
    await page.getByTestId("bot-looked").last().getByRole("button").first().click();
  },
  newtopic: async (page) => {
    await page.getByTestId("bot-new-topic").hover();
  },
  search: async (page) => {
    await page.getByTestId("bot-search").click();
    await page.getByTestId("bot-find-input").fill("login");
    await expect(page.getByTestId("bot-mark").first()).toBeVisible();
  },
  clear: async (page) => {
    await page.getByTestId("bot-more").click();
    await page.getByTestId("bot-menu-clear").click();
    await expect(page.getByTestId("bot-clear")).toBeVisible();
  },
  settings: async (page) => {
    await goTo(page, "Settings");
    await expect(page.getByTestId("bot-panel")).toBeVisible();
  },
  menu: async (page) => {
    await page.getByTestId("bot-more").click();
  },
  forget: async (page) => {
    await page.getByTestId("bot-memory-forget").first().click();
    await expect(page.getByTestId("bot-forget-backups")).not.toBeEmpty();
  },
};

for (const theme of ["dark", "light"] as const) {
  for (const [name, state] of Object.entries(STATES)) {
    for (const width of [1280, 1024]) {
      test(`bot ${name} — ${theme} — ${width}`, async ({ page }) => {
        await open(page, state, theme, width);
        await BEFORE[name]?.(page);
        await page.waitForTimeout(150);
        if (OUT) await page.screenshot({ path: `${OUT}/${name}-${theme}-${width}.png` });
        if (width === 1280 && name !== "newtopic" && name !== "lookup") await axeClean(page, `${name} ${theme}`);
      });
    }
  }
}
