/**
 * AJ-E3830 (0.5.2, `99-watch-prefill`): Bot pre-fills the Watches form;
 * the user saves. Bot's turn and the form it opened, beside each other:
 * how long and quiet hours left for the user (words Bot couldn't read),
 * the condition left with the nearest tagged; Bot's confirm card for a
 * request it read whole (the Watches Beta); and the Watches row in
 * Settings › AI › Models with that Beta. Both themes, 1280; axe on each
 * (0 critical or serious). Mocked data only: nothing is written.
 *
 * Captures go to `docs/qa/evidence/_gallery/99-watch-prefill` when
 * `AIJ_PREFILL_OUT` is set; without it the states are walked and scanned.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { WATCHES_CATALOGUE, WATCHES_FIXTURE, WATCHES_NOW, WATCHES_POOLS, WATCHES_STORED_APPS, watchesTools, STATES as WATCH_STATES } from "../../src/preview/watches";
import { STATES as BOT } from "../../src/preview/bot";
import { STATES as MODELS } from "../../src/preview/ai-models";
import { allStatuses, goTo } from "./fixtures";
import type { BotTurnInfo } from "../../src/types/generated/BotTurnInfo";
import type { WatchPrefill } from "../../src/types/generated/WatchPrefill";

const OUT = process.env.AIJ_PREFILL_OUT ? resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-watch-prefill") : null;

test.beforeAll(() => {
  if (OUT) mkdirSync(OUT, { recursive: true });
});

const DRAFT = { targets: [{ kind: "session" as const, id: "s-api" }], matching: "each" as const, conditions: [{ code: "finishes" }], lifetime: { kind: "unsaid" as const }, evenInQuietHours: false, chatTurn: "t1" };

function turn(question: string, prefill: WatchPrefill): BotTurnInfo {
  return {
    id: "t1", at_ms: WATCHES_NOW - 20_000, question, lang: "en", outcome: "watch_draft", scope: null, rows: [], total: 0, tallies: [], unknown: null, shortcut: null,
    looked: { reach: "everything", scope: null, memories: 0 }, written: null, memory: null, used: [], live: true, redacted: false, flagged: false,
    watch_draft: { kind: "prefilled", prefill },
  };
}

const CASES: Record<string, BotTurnInfo> = {
  // v13's W03 and W32 together: the session and its condition read; how
  // long and quiet hours in words code doesn't know yet.
  "form-left": turn("Tell me every time api finishes, until I remove it, yes even overnight", { draft: DRAFT, left: ["lifetime", "quiet_hours"], why: "unread", asked: "until I remove it, yes even overnight", nearest: null, card: false }),
  // v13's W28: "wants something from me" isn't a condition code reads.
  "form-nearest": turn("Let me know when api wants something from me", { draft: { ...DRAFT, conditions: [] }, left: ["condition"], why: "unread", asked: "wants something from me", nearest: "asks_you", card: false }),
  // 0.5.2 G60 (RW1 R31): "shop" names the project and sessions in it: the
  // target left, the picker open on what the words match.
  "form-several": turn("Tell me when shop finishes", { draft: { ...DRAFT, targets: [] }, left: ["target"], why: "several", asked: null, nearest: null, card: false, choices: [{ kind: "project", id: "proj-shop" }, { kind: "session", id: "s-api" }, { kind: "session", id: "s-login" }] }),
  // Read whole, Watches Beta on: Bot's card.
  card: turn("Let me know when api finishes", { draft: DRAFT, left: [], why: "verified", asked: null, nearest: null, card: true }),
};

async function open(page: Page, theme: "dark" | "light", bot: BotTurnInfo): Promise<void> {
  await page.addInitScript(
    ({ fixture, theme: t, now, watches, catalogue, jobs, pools, tools, storedApps, turns }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_TOOLS__ = tools;
      w.__AIJ_RECIPES__ = (fixture as { profiles: unknown }).profiles;
      w.__AIJ_STORED_APPS__ = storedApps;
      w.__AIJ_WATCH_VIEW__ = { watches, catalogue, calls: [] };
      w.__AIJ_AI_JOBS__ = jobs;
      w.__AIJ_POOLS__ = pools;
      w.__AIJ_BOT__ = { turns };
      w.__AIJ_NOTIFICATION_PREFS__ = { quiet_start: 22, quiet_end: 7 };
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      const real = Date.now;
      const start = real();
      Date.now = () => now + (real() - start);
    },
    { fixture: WATCHES_FIXTURE, theme, now: WATCHES_NOW, watches: WATCH_STATES.list!.watches, catalogue: WATCHES_CATALOGUE, jobs: BOT.empty!.jobs, pools: WATCHES_POOLS, tools: watchesTools(), storedApps: WATCHES_STORED_APPS, turns: [bot] },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 860 });
  await page.goto("/");
  await page.getByTestId("header-bot").click();
  await expect(page.getByTestId("bot-panel")).toBeVisible();
}

/** As tall as the form (its foot included), so one frame holds all of it. */
async function wholeForm(page: Page): Promise<void> {
  const extra = await page.evaluate(() => {
    const scroller = document.querySelector("[data-testid=flow-scroll]");
    return scroller ? scroller.scrollHeight - scroller.clientHeight : 0;
  });
  await page.setViewportSize({ width: 1280, height: 860 + Math.max(extra, 0) });
}

async function axeClean(page: Page, what: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`pre-filled form, how long and quiet hours left — ${theme}`, async ({ page }) => {
    await open(page, theme, CASES["form-left"]!);
    await expect(page.getByTestId("watch-form")).toBeVisible();
    await expect(page.getByTestId("watch-bot-fill-line")).toContainText("How long and quiet hours are left for you.");
    await expect(page.getByTestId("watch-lifetime-line")).toHaveAttribute("data-asked", "true");
    await expect(page.getByTestId("watch-quiet-left")).toBeVisible();
    await expect(page.getByTestId("watch-condition-finishes").locator("input")).toBeChecked();
    // The strip's echo comes from the preview call: a frame before it says "…".
    await expect(page.getByTestId("watch-preview-strip")).toContainText("api");
    await wholeForm(page);
    await page.mouse.move(0, 0);
    if (OUT) await page.screenshot({ path: `${OUT}/form-left-1280-${theme}.png` });
    await axeClean(page, `form left ${theme}`);
  });

  test(`pre-filled form, the condition left with the nearest — ${theme}`, async ({ page }) => {
    await open(page, theme, CASES["form-nearest"]!);
    await expect(page.getByTestId("watch-form")).toBeVisible();
    await expect(page.getByTestId("watch-condition-nearest")).toBeVisible();
    await expect(page.getByTestId("watch-condition-asks_you").locator("input")).not.toBeChecked();
    await wholeForm(page);
    await page.mouse.move(0, 0);
    if (OUT) await page.screenshot({ path: `${OUT}/form-nearest-1280-${theme}.png` });
    await axeClean(page, `form nearest ${theme}`);
  });

  test(`pre-filled form, an ambiguous name: the picker open on its matches — ${theme}`, async ({ page }) => {
    await open(page, theme, CASES["form-several"]!);
    await expect(page.getByTestId("watch-form")).toBeVisible();
    const matches = page.getByTestId("watch-picker-matches");
    await expect(matches).toBeVisible();
    await expect(matches.getByTestId("watch-pick-option").first()).toHaveAttribute("data-key", /^session:/);
    await expect(page.getByTestId("watch-lifetime-line")).toHaveAttribute("data-waits", "target");
    await wholeForm(page);
    await page.mouse.move(0, 0);
    if (OUT) await page.screenshot({ path: `${OUT}/form-several-1280-${theme}.png` });
    await axeClean(page, `form several ${theme}`);
  });

  test(`Bot's confirm card (Watches Beta) — ${theme}`, async ({ page }) => {
    await open(page, theme, CASES.card!);
    await expect(page.getByTestId("bot-watch-confirm-card")).toBeVisible();
    await expect(page.getByTestId("bot-watch-echo")).toContainText("api");
    // 0.5.2 G51: quiet hours as the form writes them (the system's clock).
    await expect(page.getByTestId("bot-watch-quiet")).toContainText("Between 10:00");
    await expect(page.getByTestId("watch-form")).toHaveCount(0);
    await page.mouse.move(0, 0);
    if (OUT) await page.screenshot({ path: `${OUT}/card-1280-${theme}.png` });
    await axeClean(page, `card ${theme}`);
  });

  test(`Settings › AI › Models: the Watches row and its Beta — ${theme}`, async ({ page }) => {
    const state = MODELS.inuse!;
    await page.addInitScript(
      ({ fixture, theme: t, s }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_AI_CATALOG__ = s.catalog;
        w.__AIJ_AI_MODELS__ = s.models;
        w.__AIJ_AI_JOBS__ = s.jobs;
        if (s.engine) w.__AIJ_AI_ENGINE__ = s.engine;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        localStorage.setItem("aij.settings.pane", "ai-models");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => Date.UTC(2026, 10, 19, 12);
      },
      { fixture: allStatuses, theme, s: state },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await goTo(page, "Settings");
    await expect(page.getByTestId("models-screen")).toBeVisible();
    const row = page.getByTestId("models-job-watches");
    await row.scrollIntoViewIfNeeded();
    await expect(page.getByTestId("models-beta-watches")).toBeVisible();
    await page.mouse.move(0, 0);
    if (OUT) await page.getByTestId("models-jobs").screenshot({ path: `${OUT}/models-row-1280-${theme}.png` });
    await axeClean(page, `models row ${theme}`);
  });
}
