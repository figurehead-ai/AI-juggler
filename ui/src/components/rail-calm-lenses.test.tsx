/**
 * D-117 (AJ-E3603), the review of CI run 43751366234 (83f700ed6): every
 * lens's rail and the folded icon rail wear the App lens's calm look, from
 * its own parts — the calm section head (`.section-label`, the level's
 * glyph on Needs you's count, L2 and L3 counted), state marks (a level's
 * dot, never a per-state glyph), Needs you rows (NeedRow: no tint, the L3
 * ✕ hidden and its red word, D-118), notes last under To review, and App icons muted at rest
 * (`.app-tone`), lit at L2/L3 or when current.
 */
import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { needsFor } from "../test/needs-kit";
import { InstanceBoard } from "./WorkflowRail";
import { FoldedRail } from "./RailFolded";
import { ActionHostProvider } from "./ItemMenu";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { resetSeen } from "../lib/seen";
import { resetNoticeStandIn } from "../lib/ipc/notices";
import { resetNotices } from "../lib/notices-store";
import { placesConversations } from "../preview/places";
import type { GroupBy } from "../lib/rail-places";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

const NOW = new Date(2026, 9, 24, 15, 0, 0).getTime();
const seed = (fixture: unknown) => Object.assign(window, { __AIJ_FIXTURE__: fixture });

beforeEach(() => {
  resetRailViewCache();
  resetRailOrderCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
  setRailView(DEFAULT_VIEW);
  resetRailViewCache();
  resetRailOrderCache();
  resetNoticeStandIn();
  resetNotices();
});

const projects: ProjectInfo[] = [{ id: "proj-ai-juggler", root: "/Users/you/Dev/ai-juggler", name: "ai-juggler", last_activity_ms: NOW, active_instance: null, accounts_used: ["Work Max"], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false }];

function own(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/Users/you/Dev/ai-juggler", kind: "terminal", isolation_label: null, project_id: "proj-ai-juggler", conversation_id: null, ...over,
  };
}

/** L3 (stopped with an error), L2 (probably stuck; the preview's asks),
 * L1 (the preview's "Done, review it") and calm. */
const SESSIONS = [own("broke", { status: "failed", needs_attention: "failed" }), own("stuck", { status: "stuck", needs_attention: "stuck" }), own("calm")];

function board(group: GroupBy) {
  setRailView({ ...DEFAULT_VIEW, group });
  const conversations = placesConversations(NOW);
  seed({ conversations });
  needsFor({ sessions: SESSIONS, conversations, projects });
  return render(
    <ConversationsProvider>
      <InstanceBoard sessions={SESSIONS} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={projects} snooze={{}} onSnooze={() => {}} onSelectProject={() => {}} />
    </ConversationsProvider>,
  );
}

/** Needs you's head: the calm label, its count of L2+L3 and its level. */
async function needsHead(view: ReturnType<typeof board>) {
  const section = await view.findByTestId("rail-needs-you");
  const head = section.matches("h3") ? section : section.querySelector("h3")!;
  return { section, head };
}

describe("every lens's rail is the App lens's calm rail (D-117)", () => {
  it("the Project lens: calm heads, state marks, Needs you rows, notes last, muted icons", async () => {
    const view = board("project");
    const { section, head } = await needsHead(view);
    expect(head.className).toContain("section-label");
    expect(head.getAttribute("data-level")).toBe("3");
    expect(head.querySelector(".level-glyph[data-level='3']")!.getAttribute("aria-hidden")).toBe("true");
    // Counted: L2 and L3 only (the review is listed, under To review).
    const counted = section.querySelectorAll("[data-need][data-level='2'], [data-need][data-level='3'], [data-testid='rail-notice'][data-level='2'], [data-testid='rail-notice'][data-level='3']").length;
    expect(within(head).getByTestId("section-count").textContent).toBe(String(counted));
    expect(within(head).getByRole("button", { name: /^Needs you \d+, 1 stopped with an error, \d+ wait on you$/ })).toBeTruthy();
    const toReview = within(section).getByTestId("rail-to-review");
    const review = section.querySelector<HTMLElement>("[data-need='review']")!;
    expect(toReview.compareDocumentPosition(review) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    // L3: one quiet mark (D-118, AJ-E3603 R5): no tinted block, its red
    // ✕ mark hidden and its word in calm ink; its name leads with the word.
    const broke = section.querySelector<HTMLElement>("[data-level='3'][data-need='failed']")!;
    expect(broke.className).not.toContain("bg-danger-soft");
    expect(broke.querySelector(".bg-danger-soft")).toBeNull();
    const why = broke.querySelector<HTMLElement>("[data-testid='need-word']")!;
    expect(why.className).toContain("text-ink-2");
    expect(why.className).not.toMatch(/text-(danger|warning)/);
    expect(why.querySelector(".level-mark[data-level='3']")).toBeTruthy();
    expect(within(section).getByRole("button", { name: /^Stopped with an error, / })).toBeTruthy();
    // Open now: the same calm head, and no per-state glyph on any row.
    expect(view.getByTestId("rail-open-now").className).toContain("section-label");
    const board_ = view.getByTestId("instance-board");
    expect(board_.querySelector("[data-state-glyph]")).toBeNull();
    const calm = board_.querySelector<HTMLElement>("[data-instance-id='calm']")!;
    expect(calm.querySelector("[data-state-dot]")).toBeTruthy();
    expect(calm.querySelector("svg")).toBeNull();
    // Its App's icon muted at rest; lit where it needs you.
    expect(calm.querySelector("[data-testid='app-icon']")!.className).toMatch(/app-tone(?! is-lit)/);
    // Its Needs you row: the App's dot lit, the ring of a guess.
    const stuck = section.querySelector<HTMLElement>("[data-need='stuck']")!;
    expect(stuck.querySelector("[data-testid='place-mark'] .is-lit")).toBeTruthy();
    expect(stuck.querySelector(".level-mark[data-probably='true']")).toBeTruthy();
    // Every collapsible keeps its chevron.
    expect(head.querySelector("[data-testid='fold-chevron']")).toBeTruthy();
    fireEvent.click(within(head).getByRole("button"));
    expect(section.querySelectorAll("[data-rail-item]")).toHaveLength(0);
  });

  it.each(["account", "pool"] as const)("the %s lens: the calm head and rows", async (lens) => {
    const view = board(lens);
    const { head } = await needsHead(view);
    expect(head.className).toContain("section-label");
    expect(head.querySelector(".level-glyph[data-level='3']")).toBeTruthy();
    expect(view.getByTestId("instance-board").querySelector("[data-state-glyph]")).toBeNull();
  });
});

describe("the folded icon rail is calm (D-117)", () => {
  function folded() {
    const conversations = placesConversations(NOW);
    seed({ conversations });
    needsFor({ sessions: SESSIONS, conversations, projects });
    return render(
      <ActionHostProvider host={{}}>
        <ConversationsProvider>
          <FoldedRail sessions={SESSIONS} projects={projects} now={NOW} selectedId={null} onSelect={() => {}} />
        </ConversationsProvider>
      </ActionHostProvider>,
    );
  }

  it("icons muted at rest, lit at L2/L3; only L2/L3 light a corner, by level, hidden, with the word in its name", async () => {
    const view = folded();
    const rail = await view.findByTestId("rail-folded");
    await waitForRows(rail);
    const rows = [...rail.querySelectorAll<HTMLElement>("button[data-rail-item]")];
    for (const row of rows) {
      const tone = row.querySelector<HTMLElement>(".app-tone")!;
      const corner = row.querySelector<HTMLElement>("[data-testid='rail-folded-needs']");
      const name = row.textContent ?? "";
      if (corner) {
        expect(corner.getAttribute("aria-hidden")).toBe("true");
        expect(["2", "3"]).toContain(corner.getAttribute("data-level"));
        expect(tone.className).toContain("is-lit");
      } else {
        expect(tone.className).not.toContain("is-lit");
      }
      expect(name).not.toMatch(/^[!✕]/);
    }
    // A failed session has ended, yet keeps its icon, lit with the red ✕ (rail-folded-levels.test.tsx).
    const broke = rail.querySelector<HTMLElement>("[data-instance-id='broke']")!;
    expect(broke.querySelector("[data-testid='rail-folded-needs'] .level-mark")!.getAttribute("data-level")).toBe("3");
    // AJ-E3603: the corner is the icon's one mark (no state dot beside it), never a filled badge.
    expect(broke.querySelector("[data-state-dot]")).toBeNull();
    expect(broke.querySelector("[data-testid='rail-folded-needs']")!.className).not.toMatch(/bg-(warning|danger|count-alarm)/);
    const stuck = rail.querySelector<HTMLElement>("[data-instance-id='stuck']")!;
    expect(stuck.querySelector("[data-testid='rail-folded-needs']")!.getAttribute("data-level")).toBe("2");
    expect(within(stuck).getByText(/^Probably stuck, /)).toBeTruthy();
    // A review (L1) keeps its word and lights no corner.
    const review = rows.find((row) => /^Done, review it/.test(row.textContent ?? ""));
    if (review) expect(review.querySelector("[data-testid='rail-folded-needs']")).toBeNull();
    // The count: the level's glyph hidden, no fill, what broke in words.
    const count = view.getByTestId("rail-folded-needs-you");
    expect(count.className).not.toContain("bg-warning-soft");
    expect(count.querySelector(".level-glyph")?.getAttribute("aria-hidden")).toBe("true");
  });
});

async function waitForRows(rail: HTMLElement) {
  await waitFor(() => expect(rail.querySelector("[data-instance-id='stuck']")).toBeTruthy());
}
