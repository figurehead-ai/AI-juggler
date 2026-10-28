/**
 * AJ-E1520 gallery: a session that started subagents. The status row's
 * count (two still running) and, on the session's page, its one quiet
 * line to them, which opens the Subagents tab (the older list in its
 * notes was retired into the tab, 2026-10-27). 0.2: the session page's
 * Subagents tab (newest first, a nested one under the one that started
 * it, status words, times, the background badge) and a subagent's
 * conversation opened from it. Both themes, axe 0 critical/serious on
 * each.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
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
  parent: null,
  depth: 1,
  background: true,
  has_conversation: true,
  ...extra,
});

const text = (speaker: string, words: string, at: number) => ({ speaker, blocks: [{ kind: "text", text: words, steps: [], hidden: 0 }], at_ms: at, model: null });

const FIXTURE = {
  ...allStatuses,
  sessions: allStatuses.sessions.map((session) => (session.id === SESSION ? { ...session, subtask_count: 2 } : session)),
  // 0.2: the Subagents tab, by the session's conversation.
  subagents: {
    [CONVERSATION]: {
      live: true,
      items: [
        agent("a4", { task: "Write tests for the tail reader, then run them until they pass", status: "running", started_ms: T0 - 6 * MIN, ended_ms: null }),
        agent("a3", { kind: "frontend-worker", task: "Run the full test suite and report failures", status: "running", started_ms: T0 - 9 * MIN, ended_ms: null }),
        agent("n1", { kind: "Explore", task: "Find where the suite's fixtures are written", parent: "a3", depth: 2, background: false, started_ms: T0 - 8 * MIN, ended_ms: T0 - 7 * MIN }),
        agent("a2", { kind: "code-reviewer", task: "Review how parse errors are reported", status: "failed", started_ms: T0 - 25 * MIN, ended_ms: T0 - 22 * MIN }),
        agent("a1", { kind: "Explore", task: "Map every transcript parser", background: false, started_ms: T0 - 40 * MIN, ended_ms: T0 - 31 * MIN }),
        agent("a0", { kind: null, task: null, status: "unknown", started_ms: null, ended_ms: null, background: null, has_conversation: false }),
      ],
      turns: {
        a1: [
          text("you", "Map every transcript parser: where each lives, what it reads, and which tool's format it handles.", T0 - 40 * MIN),
          {
            speaker: "assistant",
            blocks: [
              { kind: "steps", text: "", steps: ["searched for \"fn parse\"", "Read crates/juggler-adapters/src/transcript.rs", "Read crates/juggler-adapters/src/claude_transcript.rs"], hidden: 0 },
              { kind: "text", text: "There are three:\n\n- `claude_transcript.rs` reads Claude Code's JSONL\n- `codex_transcript.rs` reads Codex rollouts\n- `transcript.rs` picks between them from the first line", steps: [], hidden: 0 },
            ],
            at_ms: T0 - 31 * MIN,
            model: null,
          },
        ],
      },
    },
  },
};

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

async function seed(page: Page, theme: "dark" | "light") {
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
    { fixture: FIXTURE, theme, t0: T0 },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expectRail(page);
}

async function shoot(page: Page, name: string, theme: string) {
  await page.mouse.move(0, 0);
  await page.screenshot({ path: resolve(OUT, `${name}-1280x800-${theme}.png`) });
  const results = await new AxeBuilder({ page }).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${name}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`the session page's Subagents tab, and one's conversation — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    await page.locator(`[data-instance-id="${SESSION}"]`).first().click();
    await page.getByRole("tab", { name: "Subagents" }).click();
    const tab = page.getByTestId("session-subagents");
    const rows = tab.getByTestId("subagent-row");
    await expect(rows).toHaveCount(6);
    await expect(tab.getByTestId("subagent-status")).toHaveText(["Running", "Running", "Done", "Failed", "Done", "Unknown"]);
    await expect(rows.nth(2)).toHaveAttribute("data-depth", "2");
    await expect(page.getByTestId("subagents-summary")).toHaveText("6 subagents · 2 running");
    await shoot(page, "session-subagents-tab", theme);
    await rows.filter({ hasText: "Map every transcript parser" }).getByRole("button").click();
    await expect(page.getByTestId("last-turn")).toHaveCount(2);
    await expect(page.getByTestId("subagent-back")).toBeVisible();
    await shoot(page, "subagent-conversation", theme);
    await page.getByTestId("subagent-back").click();
    await expect(page.getByTestId("session-subagents").getByTestId("subagent-row")).toHaveCount(6);
  });

  test(`the session's row says two subagents run, and its page's line opens their tab — ${theme}`, async ({ page }) => {
    await seed(page, theme);
    // The View's "Subagents": each row says how many it runs.
    await railViewAs(page, { show: { subagents: true } });
    await expect(page.getByRole("button", { name: "Open its subagents: +2 agents" })).toBeVisible();
    await shoot(page, "row-count", theme);
    await page.locator(`[data-instance-id="${SESSION}"]`).first().click();
    const line = page.getByTestId("session-subagents-line");
    await line.scrollIntoViewIfNeeded();
    await expect(line).toContainText("6 subagents · 2 running");
    await expect(page.getByTestId("subagents-section")).toHaveCount(0);
    await shoot(page, "session-subagents", theme);
    await line.getByRole("button").click();
    await expect(page.getByTestId("session-subagents").getByTestId("subagent-row")).toHaveCount(6);
  });
}
