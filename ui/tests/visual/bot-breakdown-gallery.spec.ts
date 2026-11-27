/**
 * AJ-E3832 R6: Bot's context and memory breakdown (AJ-E3825 R9, rebuilt on
 * the world model) and a memory as a record (G9), in both themes at 1280
 * and 1024; axe on each (0 critical or serious) and the keyboard: each
 * indicator opens with Enter and closes with Escape. Mocked data only.
 *
 * Captures go to `AIJ_BREAKDOWN_OUT` when it names a folder (the gallery
 * family 99-bot-breakdown); without it the states are walked and scanned.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { BOT_NOW, STATES } from "../../src/preview/bot";
import { allStatuses } from "./fixtures";

const OUT = process.env.AIJ_BREAKDOWN_OUT ? resolve(process.env.AIJ_BREAKDOWN_OUT) : null;

test.beforeAll(() => {
  if (OUT) mkdirSync(OUT, { recursive: true });
});

const count = (n: number, estimated = true) => ({ n, estimated });

/** The latest answer's details: the world model's part and what it linked. */
const LATEST = {
  model_name: "Apple's on-device model",
  tokens_in: count(812, false),
  tokens_out: count(18),
  ms: 1400,
  records: 2,
  context: {
    window: 4096,
    used: count(812, false),
    answer_room: 220,
    parts: [
      { part: "instructions", tokens: count(320) },
      { part: "memory", tokens: count(10) },
      { part: "records", tokens: count(150) },
      { part: "history", tokens: count(120) },
      { part: "question", tokens: count(14) },
      { part: "world", tokens: count(96) },
    ],
  },
  memory: [
    { kind: "preferences", n: 1, tokens: count(12) },
    { kind: "notes", n: 1, tokens: count(14) },
    { kind: "asked", n: 2, tokens: count(120) },
    { kind: "watches", n: 0, tokens: count(0, false) },
  ],
  world: [
    { kind: "session", name: "api", why: "named in your question", memory: false },
    { kind: "memory", name: "“the payments agent”", why: "your own name for it", memory: true },
    { kind: "project", name: "shop", why: "connected to api", memory: false },
    { kind: "App", name: "Work", why: "connected to api", memory: false },
    { kind: "task", name: "Review the cart", why: "connected to shop", memory: false },
    { kind: "memory", name: "“the release goes out on fridays”", why: "matches your words", memory: true },
    { kind: "earlier question", name: "“is api still waiting”", why: "connected to api", memory: false },
  ],
};

/** Apple's model running, as Bot's head names it. */
const ENGINE = {
  state: "running",
  failure: null,
  model_id: "apple-on-device",
  model_name: "Apple's on-device model",
  short_name: "Apple on-device",
  engine: "apple_builtin",
  version: "26.1",
  context_tokens: 4096,
  started_ms: 1,
  answered_ms: null,
  today: { answers: 0, tokens_in: count(0, false), tokens_out: count(0, false) },
};

async function open(page: Page, theme: "dark" | "light", width: number, memoryRow: boolean): Promise<void> {
  const state = STATES.full!;
  await page.addInitScript(
    ({ fixture, theme: t, now, bot, jobs, latest, memoryRow: withMemory, engine }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      const seeded = JSON.parse(JSON.stringify(bot)) as { turns: { rows: unknown[] }[]; usage?: unknown };
      seeded.usage = { answers: 2, tokens_in: { n: 1624, estimated: false }, tokens_out: { n: 36, estimated: true }, latest };
      if (withMemory && seeded.turns[0]) {
        seeded.turns[0].rows = [
          { ...(seeded.turns[0].rows[0] as object), kind: "memory", id: "sub-1", title: "the payments agent", state: null, about: "api", memory_key: "alias:session:s-api", at_ms: now - 2 * 86_400_000, provider: null, project: null, app_label: null, branch: null, files_changed: null },
          ...seeded.turns[0].rows,
        ];
      }
      w.__AIJ_BOT__ = seeded;
      w.__AIJ_AI_JOBS__ = jobs;
      // 0.5.2 G61: the engine as the walk saw it, so the head's model line
      // reads its real length ("Apple on-device · Running").
      w.__AIJ_AI_ENGINE__ = engine;
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => now;
    },
    { fixture: allStatuses, theme, now: BOT_NOW, bot: state.bot, jobs: state.jobs, latest: LATEST, memoryRow, engine: ENGINE },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width, height: 860 });
  await page.goto("/");
  await page.getByTestId("header-bot").click();
  await expect(page.getByTestId("bot-panel")).toBeVisible();
  await expect(page.getByTestId("bot-usage")).toBeVisible();
}

async function axeClean(page: Page, what: string): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).slice(0, 3).join(", ")}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  /** 0.5.2 G61 (RW1 R31, F-RW2-7): with the watch count up, the head's
   * model line read "Apple on-device · Runn…" at 1755 wide; it is whole at
   * every width now (the line spans the head under its buttons). */
  for (const width of [1755, 1280, 1024]) {
    test(`the head's model line with the watch count — ${theme} — ${width}`, async ({ page }) => {
      await open(page, theme, width, false);
      await expect(page.getByTestId("bot-watch-count")).toBeVisible();
      const line = page.getByTestId("bot-engine");
      await expect(line).toHaveText("Apple on-device · Running");
      const whole = await line.locator(".truncate").evaluate((el) => el.scrollWidth <= el.clientWidth);
      expect(whole, "the model line isn't cut").toBe(true);
      await page.waitForTimeout(150);
      if (OUT && width === 1755) await page.screenshot({ path: `${OUT}/head-${theme}-1755.png` });
      if (width === 1755) await axeClean(page, `head ${theme} ${width}`);
    });
  }

  for (const width of [1280, 1024]) {
    test(`context breakdown — ${theme} — ${width}`, async ({ page }) => {
      await open(page, theme, width, false);
      const context = page.getByTestId("bot-context");
      await context.focus();
      await page.keyboard.press("Enter");
      const linked = page.getByTestId("bot-context-linked");
      await expect(linked).toContainText("api· named in your question");
      await expect(linked).toContainText("and 1 more");
      await expect(page.getByTestId("bot-context-breakdown")).toContainText("Linked from your work");
      await page.waitForTimeout(150);
      if (OUT) await page.screenshot({ path: `${OUT}/context-${theme}-${width}.png` });
      await axeClean(page, `context ${theme} ${width}`);
      await page.keyboard.press("Escape");
      await expect(page.getByTestId("bot-context-breakdown")).toHaveCount(0);
    });

    test(`memory breakdown — ${theme} — ${width}`, async ({ page }) => {
      await open(page, theme, width, false);
      await page.getByTestId("bot-memory").focus();
      await page.keyboard.press("Enter");
      const memory = page.getByTestId("bot-memory-breakdown");
      await expect(memory).toContainText("Preferences and names");
      await expect(memory).toContainText("Manage what Bot remembers");
      await page.waitForTimeout(150);
      if (OUT) await page.screenshot({ path: `${OUT}/memory-${theme}-${width}.png` });
      await axeClean(page, `memory ${theme} ${width}`);
      await page.keyboard.press("Escape");
      await expect(memory).toHaveCount(0);
    });

    test(`a memory as a record — ${theme} — ${width}`, async ({ page }) => {
      await open(page, theme, width, true);
      const row = page.locator("[data-testid='bot-row'][data-kind='memory']").first();
      await expect(row).toContainText("the payments agent");
      await expect(row).toContainText("What Bot remembers · your name for api");
      await row.scrollIntoViewIfNeeded();
      await page.waitForTimeout(150);
      if (OUT) await page.screenshot({ path: `${OUT}/memory-record-${theme}-${width}.png` });
      await axeClean(page, `memory record ${theme} ${width}`);
    });
  }
}
