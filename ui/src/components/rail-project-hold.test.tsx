/**
 * The Project lens (AJ-E2103, D-117): a row that needs you leaves its
 * project for Needs you, so the board holds its place until the backend's
 * Needs you is known (D-103.6, as Home's) and nothing moves when it
 * arrives (CI run 43857640404: "status is never colour-only" read rows
 * that left under it). Every row under a project says its state in words,
 * in its accessible name too (WCAG 1.4.1), a parked one included.
 */
import { act, cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { resetNoticeStandIn, standInNeeds } from "../lib/ipc/notices";
import { previewNeedsOf } from "../preview/needs";
import { projectLensEachTest } from "./project-lens.test-kit";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

projectLensEachTest();
afterEach(cleanup);

const NOW = 1_728_800_000_000;

function mk(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: `feat/${id}`,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/alpha",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-1",
    conversation_id: null,
    ...overrides,
  };
}

const PROJECTS: ProjectInfo[] = [
  { id: "proj-1", root: "/w/alpha", name: "alpha", last_activity_ms: null, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false },
];

const ROWS: InstanceInfo[] = [
  mk("working"),
  mk("stuck", { status: "stuck", needs_attention: "stuck" }),
  // A parked one is calm (D-117): it stays under its project, "Parked".
  mk("parked", { status: "quota_blocked", needs_attention: "quota_blocked", parked: "measured", parked_resets_at_ms: NOW + 2 * 3_600_000 }),
];

const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

function board() {
  return render(<InstanceBoard sessions={ROWS} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={PROJECTS} snooze={{}} onSnooze={() => {}} />);
}

/** The rows under the projects: each one's word, and that its name says it. */
function projectRows(container: HTMLElement): Array<{ id: string; word: string; named: boolean }> {
  return [...container.querySelectorAll<HTMLButtonElement>("button[data-instance-id]")]
    .filter((row) => row.querySelector("[data-state-dot]") !== null)
    .map((row) => {
      const word = (row.querySelector('[data-testid="rail-state-word"], [data-testid="rail-parked"]')?.textContent ?? "").trim();
      const named = word !== "" && within(container).queryAllByRole("button", { name: new RegExp(escape(word)) }).includes(row);
      return { id: row.dataset.instanceId!, word, named };
    });
}

describe("the Project lens holds its board until Needs you is known", () => {
  it("draws placeholders, not rows that would move, then each row once in its place", async () => {
    // The backend's first read hasn't answered: nothing is placed yet.
    resetNoticeStandIn();
    const { container } = board();
    expect(screen.getByTestId("placeholder-rail")).toBeTruthy();
    expect(container.querySelector("[data-instance-id]")).toBeNull();
    expect(screen.queryByTestId("rail-needs-you")).toBeNull();

    // It answers: the stuck one is in Needs you, the rest under alpha.
    act(() => {
      standInNeeds(previewNeedsOf({ sessions: ROWS }));
    });
    expect(screen.queryByTestId("placeholder-rail")).toBeNull();
    const needs = await screen.findByTestId("rail-needs-you");
    expect(within(needs).getByRole("button", { name: /^Probably stuck, / })).toBeTruthy();
    const rows = projectRows(container);
    expect(rows.map((row) => row.id).sort()).toEqual(["parked", "working"]);
    expect(rows.find((row) => row.id === "working")?.word).toBe("Working");
    expect(rows.find((row) => row.id === "parked")?.word).toMatch(/^Parked /);
    expect(rows.filter((row) => !row.named), "rows whose name lacks their word").toEqual([]);
  });

  it("with Needs you already known, draws the rows at once", () => {
    const { container } = board();
    expect(screen.queryByTestId("placeholder-rail")).toBeNull();
    const rows = projectRows(container);
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.filter((row) => !row.named), "rows whose name lacks their word").toEqual([]);
  });
});
