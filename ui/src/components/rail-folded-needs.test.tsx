/**
 * 0.0.11 walk, Q7: a folded project still says what needs you inside it —
 * the session rows' "!" mark and a count — and nothing unfolds by itself.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

afterEach(() => {
  cleanup();
  localStorage.clear();
});

const NOW = 1_728_800_000_000;

function mk(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p-1", adopted: false, provider: "claude-code", account_id: "claude-work", account_label: "Work Max",
    model: "opus", launch_model: "opus", running_model: null, model_source: "launch", status: "running",
    quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null, subtask_count: null,
    git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null,
    started_at_ms: NOW - 60_000, last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/Users/dev/ai-juggler",
    kind: "terminal", isolation_label: null, project_id: "proj-1", conversation_id: null, ...overrides,
  } as InstanceInfo;
}
const project = (id: string, name: string): ProjectInfo =>
  ({ id, root: `/Users/dev/${name}`, name, last_activity_ms: NOW, active_instance: null, accounts_used: ["Work Max"], instance_count: 1, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false }) as ProjectInfo;

function board(rows: InstanceInfo[]) {
  return render(
    <InstanceBoard
      sessions={rows}
      loginTerminals={[]}
      selectedId={null}
      onSelect={() => {}}
      density="comfortable"
      freshness={{}}
      now={NOW}
      projects={[project("proj-1", "ai-juggler"), project("proj-2", "storefront")]}
      snooze={{}}
      onSnooze={() => {}}
      onSelectProject={() => {}}
    />,
  );
}

it("a folded project shows how many need you, with the Needs you mark", () => {
  const view = board([
    mk("a", { status: "stuck", needs_attention: "stuck" }),
    mk("b", { status: "waiting_user", needs_attention: "waiting_user" }),
    mk("c"),
    mk("d", { project_id: "proj-2", cwd: "/Users/dev/storefront" }),
  ]);
  // Open: the rows say it themselves, the header adds nothing.
  expect(view.queryByTestId("rail-folded-needs")).toBeNull();
  fireEvent.click(view.getByRole("button", { name: "Fold ai-juggler" }));
  const mark = view.getByTestId("rail-folded-needs");
  expect(mark.textContent).toContain("2");
  expect(mark.textContent).toContain("need you");
  expect(mark.className).toContain("text-warning");
  // The calm project folded: no mark.
  fireEvent.click(view.getByRole("button", { name: "Fold storefront" }));
  expect(view.getAllByTestId("rail-folded-needs")).toHaveLength(1);
  // Still folded: nothing unfolded itself.
  expect(view.getByRole("button", { name: "Unfold ai-juggler" }).getAttribute("aria-expanded")).toBe("false");
});

it("a failed or blocked session marks the folded header as danger", () => {
  const view = board([mk("a", { status: "failed", needs_attention: "failed" }), mk("b")]);
  fireEvent.click(view.getByRole("button", { name: "Fold ai-juggler" }));
  const mark = view.getByTestId("rail-folded-needs");
  expect(within(mark).getByText("1")).toBeTruthy();
  expect(mark.textContent).toContain("needs you");
  expect(mark.className).toContain("text-danger");
});
