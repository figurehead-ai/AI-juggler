/**
 * 0.3 gallery: what the other tool was told, after a switch to another
 * tool, both themes, axe 0 critical/serious on each:
 * - the done screen's line and Open the handover;
 * - the handover opened below it (the rows "Start from a saved
 *   handover…" shows a saved one with);
 * - the Switch Graph's edge opened: the same line and link.
 * No state is gated yet: their baselines are rendered centrally.
 */
import AxeBuilder from "@axe-core/playwright";
import { type Page } from "@playwright/test";
import { expect, gate, test } from "./axe-gallery";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses, expectRail, groupBy, openRailList, openSwitch, switchRows, openWorkMaxSession } from "./fixtures";
import { EXAMPLE_SETUP } from "../../src/lib/ipc/switchSetupExamples";

const OUT = resolve(process.cwd(), "../docs/qa/evidence/_gallery/97-handover-told");

test.beforeAll(() => mkdirSync(OUT, { recursive: true }));

const LINE = "Hidden from Codex: 2 API keys · Left out: 12 earlier turns, 1 note";
const SUMMARY = { to_tool: "Codex", line: LINE };

/** A switch to another tool: the wizard, with nothing to ask about keys. */
const WIZARD = {
  mechanism: "replay",
  fidelity: "F1",
  fidelity_words: "Full story, fresh start",
  score: 0.6,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: false,
  source_keeps_running: false,
  experimental_copy: "The source stays resumable.",
  door: "wizard" as const,
  wizard_steps: ["Target", "What's carried", "Confirm"],
  wizard_reasons: ["Another tool reads the conversation."],
  usual_account: false,
  carry: { carried: ["The conversation"], regenerated: [], dropped: ["Subagents' work"] },
  carry_items: [
    { label: "The conversation", hint: "The other tool reads it, newest first as much as fits.", how: "moves", kind: "chat" },
    { label: "Subagents' work", hint: "It doesn't go along.", how: "stays", kind: "subagents" },
  ],
  redaction_items: [],
  secrets: { asks: false, reader: "Codex (OpenAI)" },
  target_tool: "codex",
};

async function switchedToCodex(page: Page, theme: "dark" | "light") {
  await page.addInitScript(
    ({ fixture, theme: t, preflight, setup, summary }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_SWITCH_PREFLIGHT__ = preflight;
      w.__AIJ_SWITCH_SETUP__ = setup;
      w.__AIJ_HANDOVER_SUMMARY__ = { "*": summary };
      localStorage.setItem("aij.theme", t);
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
      Date.now = () => 1_728_800_000_000;
    },
    { fixture: structuredClone(allStatuses), theme, preflight: WIZARD, setup: { ...EXAMPLE_SETUP, offers: [] }, summary: SUMMARY },
  );
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
  await page.setViewportSize({ width: 1280, height: 1400 });
  await page.goto("/");
  await expectRail(page);
  await openWorkMaxSession(page);
  await openSwitch(page);
  await switchRows(page).first().click();
  await expect(page.getByTestId("switch-wizard")).toBeVisible();
  await page.getByTestId("switch-continue").click();
  await page.getByTestId("switch-continue").click();
  await page.getByTestId("switch-confirm").click();
  await expect(page.getByTestId("switch-done")).toBeVisible();
}

async function axeClean(page: Page, where: string, what: string) {
  const results = await new AxeBuilder({ page }).include(where).analyze();
  const bad = results.violations.filter((v) => v.impact === "critical" || v.impact === "serious");
  expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target}`), `axe on ${what}`).toEqual([]);
}

for (const theme of ["dark", "light"] as const) {
  test(`done: what Codex was told, in one line — ${theme}`, gate("97-handover-told"), async ({ page }) => {
    await switchedToCodex(page, theme);
    await expect(page.getByTestId("handover-line")).toHaveText(LINE);
    await page.getByTestId("switch-done").screenshot({ path: resolve(OUT, `done-line-${theme}.png`) });
    await axeClean(page, "[data-testid='switch-flow']", "the done screen");
  });

  test(`done: the handover opened — ${theme}`, async ({ page }) => {
    await switchedToCodex(page, theme);
    await page.getByTestId("handover-open").click();
    const sheet = page.getByTestId("handover-sheet");
    await expect(sheet.getByTestId("pack-withheld")).toContainText("2 API keys");
    await page.getByTestId("switch-done").screenshot({ path: resolve(OUT, `done-open-${theme}.png`) });
    await axeClean(page, "[data-testid='switch-flow']", "the handover opened");
  });

  test(`graph: the edge's handover opened — ${theme}`, async ({ page }) => {
    const graph = [
      {
        title: "Login refactor",
        epochs: [
          { id: "ep-a", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-1" },
          { id: "ep-b", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-2" },
        ],
        edges: [{ id: "edge-1", from: "ep-a", to: "ep-b", label: "Full story, fresh start", switch_back: false, handover: SUMMARY }],
        collapsed: 0,
        hop_note: null,
      },
    ];
    await page.addInitScript(
      ({ fixture, workGraph, theme: t }) => {
        const w = window as unknown as Record<string, unknown>;
        w.__AIJ_FIXTURE__ = fixture;
        w.__AIJ_SWITCH_GRAPH__ = workGraph;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
        Date.now = () => 1_728_800_000_000;
      },
      { fixture: structuredClone(allStatuses), workGraph: graph, theme },
    );
    await page.emulateMedia({ reducedMotion: "reduce", colorScheme: theme });
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto("/");
    await openRailList(page);
    // A project's header is the Project grouping's (D-54).
    await groupBy(page, "Project");
    await page.getByRole("button", { name: /^ai-juggler/ }).first().click();
    await page.getByRole("tablist", { name: "Project tabs" }).getByRole("tab", { name: "Overview" }).click();
    await page.locator("[data-testid='switch-graph-edge'][data-handover='true']").click();
    const detail = page.getByTestId("switch-graph-edge-detail");
    await expect(detail.getByTestId("handover-line")).toHaveText(LINE);
    await page.getByTestId("switch-graph").screenshot({ path: resolve(OUT, `graph-edge-${theme}.png`) });
    await axeClean(page, "[data-testid='switch-graph']", "the graph's edge opened");
  });
}
