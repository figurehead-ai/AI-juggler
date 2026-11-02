/**
 * AJ-E1520 gallery: a session that started subagents. 0.2: the status
 * row's count and the session page's one quiet line to its tab. 0.3: the
 * tab answers what's running, what failed and what it cost first — the
 * summary line, the views (opening on Running), groups under the message
 * that started them with real tree lines, a search that keeps a match's
 * parent, most tokens first, nothing matching; a subagent's own page
 * (running with its processes, done with its result and files, failed
 * with its reason) and its conversation; the Usage tab's line to them;
 * a session of 437. Both themes, 1280×800 and 1024×700, axe 0
 * critical/serious on each.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, expectRail, railViewAs } from "./fixtures";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/88-subagents");
const T0 = 1_728_800_000_000;
const SESSION = "sess-2";
/** The running session's conversation (fixtures.ts). */
const CONVERSATION = "822e4125-e07d-4866-9954-ec9295efb079";
const MIN = 60_000;

const agent = (id: string, extra: Record<string, unknown>) => ({
  id,
  kind: "general-purpose",
  task: null,
  status: "done",
  started_ms: T0 - 40 * MIN,
  ended_ms: T0 - 31 * MIN,
  duration_ms: 9 * MIN,
  parent: null,
  depth: 1,
  background: true,
  has_conversation: true,
  model: "claude-sonnet-4-5",
  ...extra,
});

const FIX = "fix the totals and add tests for rounding";
const SPLIT = "split the checkout page into components";

const ITEMS = [
  agent("a4", { task: "Write tests for the tail reader, then run them until they pass", status: "running", started_ms: T0 - 6 * MIN, ended_ms: null, duration_ms: 6 * MIN, own_tokens: 288_000, asked: FIX, asked_ms: T0 - 12 * MIN, model: "claude-opus-4-1" }),
  agent("a3", { kind: "frontend-worker", task: "Run the full test suite and report failures", status: "running", started_ms: T0 - 9 * MIN, ended_ms: null, duration_ms: 9 * MIN, own_tokens: 390_000, asked: FIX, asked_ms: T0 - 12 * MIN, model: "claude-opus-4-1", prompt: "Run the whole test suite (unit and integration). For every failure, list the test, the file and line, and the first line of the error. Don't fix anything yet.\n\nGroup the failures by the module they're in, and say which ones look like the same cause.\nIf the suite takes longer than 10 minutes, report what you have.\nKeep the output short.\nUse the project's own scripts.", files: ["src/cart/store.ts", "src/cart/tax.ts", "src/cart/round.ts", "tests/cart.test.ts", "tests/tax.test.ts", "tests/round.test.ts", "tests/fixtures/cart.json"], detail: { tools: { total: 38, reads: 12, searches: 9, commands: 6, edits: 4, other: 7 }, lines: { added: 120, removed: 34 }, files_total: 7, tokens: { input: 21_000, output: 34_000, cache_read: 320_000, cache_write: 15_000, reasoning: 0 }, cost_usd: 0.92, share_pct: 6.1, counted: "in_session", commands: 6, processes: [{ key: "p:61044:1", pid: 61044, name: "npm", command: "npm run dev", rss_bytes: 212_000_000, cpu_pct: 1.2, alive: true }] } }),
  agent("n1", { kind: "Explore", task: "Find where the suite's fixtures are written", parent: "a3", depth: 2, background: false, started_ms: T0 - 8 * MIN, ended_ms: T0 - 7 * MIN, duration_ms: 32_000, own_tokens: 38_000, model: "claude-haiku-4-5", result: "## Found them\n\nThe fixtures are written by `tests/fixtures/build.ts` before each run:\n\n- `cart.json` from the seed\n- `tax.json` from the rates table\n\nNothing else writes there.", files: [] }),
  agent("n2", { kind: "Explore", task: "Read the tax tables", parent: "a3", depth: 2, status: "running", background: true, started_ms: T0 - 2 * MIN, ended_ms: null, duration_ms: 2 * MIN, own_tokens: 22_000, model: "claude-haiku-4-5" }),
  agent("n3", { kind: "Explore", task: "Check the rounding helper's callers", parent: "n2", depth: 3, background: false, started_ms: T0 - MIN, ended_ms: T0 - 0.5 * MIN, duration_ms: 30_000, own_tokens: 9_000, model: "claude-haiku-4-5" }),
  agent("a2", { kind: "code-reviewer", task: "Review how parse errors are reported", status: "failed", started_ms: T0 - 11 * MIN, ended_ms: T0 - 8 * MIN, duration_ms: 3 * MIN, own_tokens: 96_000, asked: FIX, asked_ms: T0 - 12 * MIN, failure: "The diff it was asked to review no longer exists (the branch was rebased)." }),
  agent("b1", { kind: "Plan", task: "Plan the component split", started_ms: T0 - 25 * 60 * MIN, ended_ms: T0 - 24.5 * 60 * MIN, duration_ms: 30 * MIN, own_tokens: 140_000, asked: SPLIT, asked_ms: T0 - 25.1 * 60 * MIN }),
  agent("b2", { task: "Move the summary into its own component", started_ms: T0 - 24 * 60 * MIN, ended_ms: T0 - 23.8 * 60 * MIN, duration_ms: 12 * MIN, own_tokens: 210_000, asked: SPLIT, asked_ms: T0 - 25.1 * 60 * MIN }),
  agent("b3", { task: "Move the totals into their own component", status: "stopped", started_ms: T0 - 24 * 60 * MIN, ended_ms: T0 - 23.9 * 60 * MIN, duration_ms: 6 * MIN, own_tokens: 70_000, asked: SPLIT, asked_ms: T0 - 25.1 * 60 * MIN }),
  agent("a0", { kind: null, task: null, status: "unknown", started_ms: null, ended_ms: null, duration_ms: null, background: null, has_conversation: false, model: null }),
];

/** A session the size of the largest seen: 437 in 40 turns. */
const BIG = Array.from({ length: 437 }, (_, n) =>
  agent(`g${n}`, {
    task: `Part ${n} of the migration: move the ${["cart", "tax", "orders", "users", "search"][n % 5]} module`,
    status: n < 3 ? "running" : n % 97 === 5 ? "failed" : "done",
    started_ms: T0 - n * 3 * MIN,
    ended_ms: n < 3 ? null : T0 - n * 3 * MIN + 2 * MIN,
    duration_ms: n < 3 ? 2 * MIN : 2 * MIN,
    own_tokens: 20_000 + ((n * 7919) % 90_000),
    asked: `Turn ${Math.floor(n / 11)}: move the next modules`,
    asked_ms: T0 - Math.floor(n / 11) * 33 * MIN,
  }),
);

const text = (speaker: string, words: string, at: number) => ({ speaker, blocks: [{ kind: "text", text: words, steps: [], hidden: 0 }], at_ms: at, model: null });

const FIXTURE = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) => (session.id === SESSION ? { ...session, subtask_count: 3 } : session)),
  subagents: {
    [CONVERSATION]: {
      live: true,
      session_tokens: 2_100_000,
      items: ITEMS,
      turns: {
        n1: [
          text("you", "Find where the suite's fixtures are written.", T0 - 8 * MIN),
          { speaker: "assistant", blocks: [{ kind: "text", text: "They're written by `tests/fixtures/build.ts` before each run.", steps: [], hidden: 0 }], at_ms: T0 - 7 * MIN, model: null },
        ],
      },
    },
    big: { live: true, session_tokens: 30_000_000, items: BIG },
  },
};

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light", size = { width: 1280, height: 800 }, fixture: unknown = FIXTURE) {
  await page.addInitScript(
    ({ fixture, theme, t0 }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      localStorage.setItem("aij.theme", theme);
      localStorage.setItem("aij.density", "comfortable");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => t0;
    },
    { fixture, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize(size);
  await page.goto("/");
  await expectRail(page);
}

async function shoot(page: Page, name: string, theme: string, size = "1280x800") {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-${size}-${theme}.png`) });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

async function openTab(page: Page, folded = false) {
  // The open rail settled first: Needs you arriving after the click pushed
  // the selected row back out of view (CI, #198). Folded (1024), it has
  // no Needs you list.
  if (!folded) await expect(page.getByTestId("rail-needs-you")).toContainText(/Needs you\s*5/);
  await page.locator(`[data-instance-id="${SESSION}"]`).first().click();
  if (!folded) await expect(page.locator("[data-testid='rail'] [data-rail-selected='true']").first()).toBeInViewport();
  await page.getByRole("tab", { name: "Subagents" }).click();
  await expect(page.getByTestId("session-subagents")).toBeVisible();
}

for (const theme of ["dark", "light"] as const) {
  test(`the Subagents tab: running first, all grouped, a search, most tokens, none matching — ${theme}`, gate("88-subagents"), async ({ page }) => {
    await seed(page, theme);
    await openTab(page);
    const tab = page.getByTestId("session-subagents");
    await expect(page.getByRole("radio", { name: "Running 3" })).toHaveAttribute("aria-checked", "true");
    await expect(tab.getByTestId("subagents-summary")).toContainText("10 subagents");
    await expect(tab.getByTestId("subagent-row")).toHaveCount(3);
    await shoot(page, "tab-running", theme);
    await page.getByRole("radio", { name: "All" }).click();
    await expect(tab.getByTestId("subagent-group")).toHaveCount(3);
    await expect(tab.getByTestId("subagent-row").first()).toBeVisible();
    await shoot(page, "tab-all-grouped", theme);
    await tab.getByTestId("subagents-search").fill("file:tax.ts");
    await expect(tab.getByTestId("subagent-row")).toHaveCount(1);
    await shoot(page, "tab-search", theme);
    await tab.getByTestId("subagents-search").fill("");
    await tab.getByTestId("subagents-sort").click();
    await page.getByRole("menuitemradio", { name: /Most tokens/ }).click();
    await shoot(page, "tab-most-tokens", theme);
    await page.getByRole("radio", { name: /Needs attention/ }).click();
    await tab.getByTestId("subagents-search").fill("nothing like this");
    await expect(tab.getByTestId("subagents-none-match")).toBeVisible();
    await shoot(page, "tab-none-match", theme);
  });

  test(`a subagent's page: running with its processes, done with its result, failed, its conversation — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await openTab(page);
    await page.getByRole("radio", { name: "All" }).click();
    await page.getByTestId("subagent-row").filter({ hasText: "Run the full test suite" }).click();
    await expect(page.getByTestId("subagent-title")).toHaveText("Run the full test suite and report failures");
    await expect(page.getByTestId("subagent-process")).toBeVisible();
    await shoot(page, "detail-running", theme);
    await page.getByTestId("subagent-child").filter({ hasText: "Find where" }).click();
    await expect(page.getByTestId("subagent-result")).toBeVisible();
    await shoot(page, "detail-done", theme);
    await page.getByTestId("subagent-open-conversation").click();
    await expect(page.getByTestId("last-turn")).toHaveCount(2);
    await shoot(page, "detail-conversation", theme);
    await page.getByTestId("subagent-back").click();
    await page.keyboard.press("Escape");
    await page.getByTestId("subagent-row").filter({ hasText: "Review how parse errors" }).click();
    await expect(page.getByTestId("subagent-failure")).toBeVisible();
    await shoot(page, "detail-failed", theme);
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("session-subagents")).toBeVisible();
  });

  test(`the narrower window, the Usage tab's line, and the session's line — ${theme}`, async ({ page }) => {
    await seed(page, theme, { width: 1024, height: 700 });
    await openTab(page, true);
    await page.getByRole("radio", { name: "All" }).click();
    await shoot(page, "tab-all-grouped", theme, "1024x700");
    await page.getByTestId("subagent-row").filter({ hasText: "Run the full test suite" }).click();
    await shoot(page, "detail-running", theme, "1024x700");
    await page.keyboard.press("Escape");
    await page.getByRole("tab", { name: "Usage" }).click();
    await expect(page.getByTestId("usage-subagents-line")).toContainText("Subagents used");
    await shoot(page, "usage-line", theme, "1024x700");
    await page.getByTestId("usage-subagents-open").click();
    await expect(page.getByTestId("subagents-sort")).toHaveAttribute("data-value", "tokens");
  });

  test(`the session's row says three subagents run, and its page's line opens their tab — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await railViewAs(page, { show: { subagents: true } });
    const count = page.getByRole("button", { name: "Open its subagents: +3 agents" });
    await expect(count).toBeVisible();
    // The rail settled first: Needs you arriving later pushes rows down.
    await expect(page.getByTestId("rail-needs-you")).toContainText(/Needs you\s*5/);
    // Its row can sit below the rail's fold (Open now holds six rows).
    await count.scrollIntoViewIfNeeded();
    await expect(count).toBeInViewport();
    await shoot(page, "row-count", theme);
    await page.locator(`[data-instance-id="${SESSION}"]`).first().click();
    await page.getByRole("tab", { name: "Overview" }).click();
    const line = page.getByTestId("session-subagents-line");
    await expect(line).toContainText("10 subagents · 3 running");
    await line.getByRole("button").click();
    await expect(page.getByTestId("session-subagents")).toBeVisible();
  });

  test(`a session of 437 subagents opens grouped and windowed — ${theme}`, async ({ page }) => {
    const big = { ...FIXTURE, subagents: { ...FIXTURE.subagents, [CONVERSATION]: FIXTURE.subagents.big } };
    await seed(page, theme, { width: 1280, height: 800 }, big);
    await openTab(page);
    await page.getByRole("radio", { name: "All" }).click();
    await expect(page.getByTestId("subagents-summary")).toContainText("437 subagents");
    await expect(page.getByTestId("subagent-group").first()).toHaveAttribute("data-open", "true");
    await shoot(page, "tab-437", theme);
  });
}

/** 0.3.4: a session it started (the Claude app's own record), for the
 * Overview's switcher. */
const STARTED = { [CONVERSATION]: [{ tool: "claude-code", conversation_id: "started-1", title: "Nightly: skip docs-only changes", last_activity_ms: T0 - 4 * MIN }] };

for (const theme of ["dark", "light"] as const) {
  test(`0.3.4: the Overview switches its conversation to a subagent's, in place, and back — ${theme}`, async ({ page }) => {
    await page.addInitScript((started) => void ((window as unknown as Record<string, unknown>).__AIJ_STARTED__ = started), STARTED);
    await seed(page, theme);
    await page.locator(`[data-instance-id="${SESSION}"]`).first().click();
    const list = page.getByTestId("switcher-list");
    await expect(list).toBeVisible();
    await expect(page.getByTestId("switcher-own")).toHaveAttribute("aria-current", "true");
    await expect(page.getByTestId("switcher-running").getByTestId("switcher-subagent")).toHaveCount(3);
    await expect(page.getByTestId("switcher-started").getByTestId("switcher-started-session")).toHaveCount(1);
    await expect(page.getByTestId("switcher-done-toggle")).toContainText("Done · 7");
    await shoot(page, "switcher-own", theme);
    await page.getByTestId("switcher-done-toggle").click();
    await page.getByTestId("switcher-subagent").filter({ hasText: "Find where the suite's fixtures are written" }).click();
    await expect(page.getByTestId("last-turn")).toHaveCount(2);
    await expect(page.getByTestId("switcher-back")).toBeVisible();
    await shoot(page, "switcher-subagent", theme);
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("switcher-own")).toHaveAttribute("aria-current", "true");
  });
}
