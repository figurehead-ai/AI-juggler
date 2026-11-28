/**
 * AJ-E3820 gallery (`99-turn-review`): the turn review where a turn's end
 * is shown, as the approved mock draws it
 * (`AIJuggler-design-refs/turn-review`): on Home's feed (a turn with the
 * Beta's lines opened, a failed turn opened, a clean one folded) and under
 * a command-line session's last turn on its page (code-only, opened);
 * and its row on Settings › AI › Models (R4: an opt-in Beta, its switch
 * off, the tools it covers, R5). 0.5.2: the cue while the model's lines
 * are written (G28), a revert and a delete (G25, G26), and an ended
 * terminal session's page with its last turn's review (G28).
 * Both themes, axe 0 critical/serious on each. Invented turns, generic
 * names, no real transcript. Captures are written locally; no gate
 * baseline.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, goTo } from "./fixtures";
import { STATES as MODELS } from "../../src/preview/ai-models";
import { CLEAN, DELETED, FAILED, REVERT, STREAMED, STREAMED_AI } from "../../src/preview/turn-review";

const DIR = resolve(process.cwd(), "../docs/qa/evidence/_gallery/99-turn-review");
const T0 = 1_728_800_000_000;
const MIN = 60_000;

test.beforeAll(() => mkdirSync(DIR, { recursive: true }));

const CLI = { kind: "app", name: null, provider: "claude-code", folder: "acc-work", folder_label: "Work Max", desktop: false };
const CODEX = { ...CLI, provider: "codex", folder: "acc-personal", folder_label: "Personal" };
const NO_FACTS = {
  project: null, project_id: null, cwd: null, duration_ms: null, detail: null, attempt: null, tries: null, window: null, resets_at_ms: null, meter: null,
  sessions: null, credits: null, version: null, from_label: null, from_provider: null, to_label: null, to_provider: null, mechanism: null, fidelity: null,
  pool: null, pool_id: null, thread: null, thread_count: null, quote: null, change: null, need: null, line: null, told: null,
};
const TARGET = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null, thread_id: null };

function post(id: string, ago: number, subject: string, project: string, author: typeof CLI, kind: string, changes: { files: number; names: string[]; added: number | null; removed: number | null }) {
  return {
    id: `l:${id}`, at_ms: T0 - ago * MIN, kind, category: kind === "failed" ? "needs" : "finished", level: kind === "failed" ? 3 : 1, mine: false,
    coalesce: `s:${id}`, rollup: null, rollup_window_ms: 0, scope: null, subject, digest: null, author,
    facts: { ...NO_FACTS, project, changes: { turn: `${id}:0`, ...changes } },
    target: { ...TARGET, session_id: id },
  };
}

const FEED = [
  // The same reader's count as its review (G25): the reverted file too.
  post("s-stream", 4, "Stream the export", "shop-api", CLI, "finished", { files: 3, names: ["export/dunnock.py", "export/writer.py", "export/quoting.py"], added: 58, removed: 34 }),
  post("s-parser", 12, "Port the parser", "parser-kit", CODEX, "failed", { files: 1, names: ["parse/godwit.ts"], added: 120, removed: 96 }),
  post("s-login", 31, "Fix the flaky login test", "web-app", CLI, "finished", { files: 1, names: ["web/login.test.ts"], added: 4, removed: 2 }),
];

const job = (ready: boolean) => ({
  job: "review", state: ready ? "ready" : "beta_off", beta: ready, beta_on: ready, model_id: ready ? "apple-on-device" : null, model_name: ready ? "Apple’s on-device model" : null,
  why: ready ? "recommended" : null, pick: null, covers: ["claude-code", "codex"],
});

async function seed(page: Page, theme: "dark" | "light", globals: Record<string, unknown>) {
  await page.addInitScript(
    ({ theme, t0, globals }) => {
      const w = window as unknown as Record<string, unknown>;
      for (const [key, value] of Object.entries(globals)) w[key] = value;
      w.__AIJ_APP_SETTINGS__ = {};
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { theme, t0: T0, globals },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
}

async function axeClean(page: Page, what: string) {
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

const conversation = {
  id: "c-stream",
  tool: "claude-code",
  account_id: "claude-work",
  account_label: "Work Max",
  project_id: "proj-1",
  cwd: "/Users/you/Dev/shop-api",
  title: "Stream the dunnock export",
  git_branch: "main",
  started_ms: T0 - 40 * MIN,
  last_activity_ms: T0 - 4 * MIN,
  tokens: { input: 31_000, output: 5_200, cache_read: 120_000, cache_write: 2_100 },
  live: true,
  session_id: null,
  places: [{ app: "Terminal", app_id: null, profile: "Work Max", instance: "", pid: 4243, kind: "terminal", label: "Terminal" }],
  pid: 4243,
  state: "idle",
  viewed_ms: T0 - 30 * MIN,
  background: false,
  inactive: false,
  app: "cli:claude-work",
  started_in: null,
};

const TURNS = [
  { speaker: "you", blocks: [{ kind: "text", text: "Stream the dunnock export instead of building the whole list." }], at_ms: T0 - 10 * MIN, model: null },
  {
    speaker: "assistant",
    blocks: [
      { kind: "steps", steps: ["Edit export/dunnock.py", "Edit export/writer.py", "Edit export/quoting.py", 'ran "pytest export"'], title: "Edited 3 files, ran a command", calls: [] },
      {
        kind: "text",
        text: "Switched the dunnock export to stream rows instead of building one big list (export/dunnock.py), and the writer now takes rows one at a time. I also changed the CSV quoting, then reverted that because two downstream readers depend on the old quoting. The integration suite is still unrun.\n\nShould I keep the old quoting behind a flag?",
      },
    ],
    at_ms: T0 - 4 * MIN,
    model: "claude-opus-4-5",
  },
];

for (const theme of ["dark", "light"] as const) {
  test(`turn review — feed — ${theme}`, async ({ page }) => {
    await seed(page, theme, {
      __AIJ_FIXTURE__: allStatuses,
      __AIJ_FEED__: FEED,
      __AIJ_AI_JOBS__: [job(true)],
      __AIJ_REVIEWS__: { "s-stream": STREAMED, "s-parser": FAILED, "s-login": CLEAN },
      __AIJ_REVIEWS_AI__: { "s-stream": STREAMED_AI },
    });
    await page.setViewportSize({ width: 1280, height: 1250 });
    await page.goto("/");
    await expect(page.getByTestId("home")).toBeVisible();
    await expect(page.getByTestId("turn-review-toggle")).toHaveCount(3);
    const toggles = page.getByTestId("turn-review-toggle");
    await toggles.nth(0).click();
    await toggles.nth(1).click();
    await expect(page.getByTestId("turn-review-ai")).toBeVisible();
    await expect(page.getByTestId("turn-review-panel")).toHaveCount(2);
    await page.mouse.move(0, 0);
    await page.getByTestId("feed-card").first().scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(DIR, `feed-1280-${theme}.png`) });
    await axeClean(page, `feed ${theme}`);
  });

  test(`turn review — session page — ${theme}`, async ({ page }) => {
    await seed(page, theme, {
      __AIJ_FIXTURE__: { ...allStatuses, conversations: [conversation], last_messages: { "c-stream": { state: "shown", redacted: 0, turns: TURNS } } },
      __AIJ_AI_JOBS__: [job(false)],
      __AIJ_REVIEWS__: { "c-stream": STREAMED },
    });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await expect(page.getByTestId("rail")).toBeVisible();
    await page.getByTestId("rail").locator("[data-conversation-id]").filter({ hasText: "Stream the dunnock export" }).first().click();
    await expect(page.getByTestId("conversation-page")).toBeVisible();
    await page.getByTestId("turn-review-toggle").click();
    await expect(page.getByTestId("turn-review-panel")).toBeVisible();
    await page.getByTestId("turn-review-panel").scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.getByTestId("last-messages").screenshot({ path: resolve(DIR, `session-1280-${theme}.png`) });
    await axeClean(page, `session ${theme}`);
  });

  // 0.5.2 G28: opened while the model writes its lines: the facts at
  // once, a quiet line saying the lines are coming.
  test(`turn review — lines on their way — ${theme}`, async ({ page }) => {
    await seed(page, theme, {
      __AIJ_FIXTURE__: allStatuses,
      __AIJ_FEED__: FEED.slice(0, 1),
      __AIJ_AI_JOBS__: [job(true)],
      __AIJ_REVIEWS__: { "s-stream": STREAMED },
      __AIJ_REVIEWS_AI__: { "s-stream": STREAMED_AI },
      __AIJ_REVIEWS_AI_MS__: 600_000,
    });
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto("/");
    await page.getByTestId("turn-review-toggle").click();
    await expect(page.getByTestId("turn-review-writing")).toBeVisible();
    await page.mouse.move(0, 0);
    await page.getByTestId("feed-card").first().screenshot({ path: resolve(DIR, `writing-${theme}.png`) });
    await axeClean(page, `writing ${theme}`);
  });

  // 0.5.2 G25, G26: a revert counts the post's files and lists only what
  // its words undid; a delete says files only, in the post and its review.
  test(`turn review — revert and delete — ${theme}`, async ({ page }) => {
    await seed(page, theme, {
      __AIJ_FIXTURE__: allStatuses,
      __AIJ_FEED__: [
        post("s-dates", 3, "Change the date format in the API responses to ISO 8601", "api", CLI, "finished", { files: 2, names: ["src/format.ts", "src/locale.ts"], added: 3, removed: 3 }),
        post("s-handlers", 9, "Remove the old handlers module", "api", CODEX, "finished", { files: 2, names: ["src/old_handlers.py", "src/routes.py"], added: null, removed: null }),
      ],
      __AIJ_AI_JOBS__: [job(false)],
      __AIJ_REVIEWS__: { "s-dates": REVERT, "s-handlers": DELETED },
    });
    await page.setViewportSize({ width: 1280, height: 1100 });
    await page.goto("/");
    const toggles = page.getByTestId("turn-review-toggle");
    await expect(toggles).toHaveCount(2);
    await expect(toggles.nth(0)).toHaveText("Turn review · 2 files · Medium risk");
    await toggles.nth(0).click();
    await toggles.nth(1).click();
    await expect(page.getByTestId("turn-review-panel")).toHaveCount(2);
    await page.mouse.move(0, 0);
    await page.getByTestId("feed").screenshot({ path: resolve(DIR, `revert-delete-${theme}.png`) });
    await axeClean(page, `revert and delete ${theme}`);
  });

  // 0.5.2 G28: a command-line session AI Juggler ran, ended, its terminal
  // replayed: its last turn's review under the terminal.
  test(`turn review — ended terminal session — ${theme}`, async ({ page }) => {
    await seed(page, theme, {
      __AIJ_FIXTURE__: allStatuses,
      __AIJ_TERMINAL_KEPT__: ["sess-6"],
      __AIJ_AI_JOBS__: [job(false)],
      __AIJ_REVIEWS__: { "sess-6": REVERT },
    });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await page.getByRole("searchbox", { name: /Search sessions/ }).fill("is:ended");
    await page.locator('[data-instance-id="sess-6"]').first().click({ position: { x: 48, y: 10 } });
    await expect(page.getByTestId("ended-terminal-kept")).toBeVisible();
    await page.getByTestId("turn-review-toggle").click();
    await expect(page.getByTestId("turn-review-panel")).toBeVisible();
    await page.mouse.move(0, 0);
    await page.getByTestId("ended-terminal-kept").screenshot({ path: resolve(DIR, `ended-terminal-${theme}.png`) });
    await axeClean(page, `ended terminal ${theme}`);
  });

  test(`turn review — Models row — ${theme}`, async ({ page }) => {
    const state = MODELS.models!;
    await seed(page, theme, {
      __AIJ_FIXTURE__: allStatuses,
      __AIJ_AI_CATALOG__: state.catalog,
      __AIJ_AI_MODELS__: state.models,
      __AIJ_AI_JOBS__: state.jobs,
    });
    await page.addInitScript(() => localStorage.setItem("aij.settings.pane", "ai-models"));
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto("/");
    await goTo(page, "Settings");
    await expect(page.getByTestId("models-screen")).toBeVisible();
    const jobs = page.getByTestId("models-jobs");
    await expect(page.getByTestId("models-job-covers-review")).toBeVisible();
    await jobs.scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await jobs.screenshot({ path: resolve(DIR, `models-row-${theme}.png`) });
    await axeClean(page, `models ${theme}`);
  });
}
