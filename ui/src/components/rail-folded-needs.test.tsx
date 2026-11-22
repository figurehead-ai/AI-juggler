/**
 * 0.0.11 walk, Q7: a folded project still says what needs you inside it —
 * the session rows' "!" mark and a count — and nothing unfolds by itself.
 * D-117 (AJ-E3603): open too, as an App head says it.
 */
import { needsOfRows } from "../test/needs-kit";
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { useProjectLens } from "./project-lens.test-kit";

// AJ-E2104: these read the project board, the rail's Project lens.
useProjectLens();

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
  needsOfRows(rows); // D-49: the backend's list
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

/** A project's head (its button) by its name. */
const head = (view: ReturnType<typeof board>, name: string) => view.getAllByTestId("rail-project").find((button) => button.textContent?.includes(name))!;

// D-117 (AJ-E3603): a project head shows what needs you in it as an App
// head does, folded or not (it showed only its Apps' icons and a count):
// its level's mark beside the number, the mark hidden, the words in its
// name; its chevron still folds it, and nothing unfolds by itself.
it("a project head says what needs you in it, folded or not, as an App head does", () => {
  const view = board([
    mk("a", { status: "stuck", needs_attention: "stuck" }),
    mk("b", { status: "waiting_user", needs_attention: "waiting_user" }),
    mk("c"),
    mk("d", { project_id: "proj-2", cwd: "/Users/dev/storefront" }),
  ]);
  const ai = head(view, "ai-juggler");
  const mark = within(ai).getByTestId("place-needs");
  expect(mark.textContent).toBe("2");
  // D-117: a mark, not a badge: the number in ink by the amber dot (L2).
  expect(mark.className).toContain("text-ink");
  expect(mark.getAttribute("data-level")).toBe("2");
  expect(mark.querySelector(".level-mark[data-level='2']")!.getAttribute("aria-hidden")).toBe("true");
  expect(mark.closest("[aria-hidden='true']")).toBeTruthy();
  // Said in its name, as an App head says it.
  expect(ai).toBe(view.getByRole("button", { name: "ai-juggler, 3 sessions, 2 need you" }));
  // Its Apps' icons lit (L2); the calm project's muted, with no mark.
  expect(ai.querySelector(".app-tone")!.classList.contains("is-lit")).toBe(true);
  const calm = head(view, "storefront");
  expect(within(calm).queryByTestId("place-needs")).toBeNull();
  expect(calm.querySelector(".app-tone")!.classList.contains("is-lit")).toBe(false);
  expect(calm).toBe(view.getByRole("button", { name: "storefront, 1 session" }));
  // Folded, it still says so; the chevron folds, nothing unfolds itself.
  fireEvent.click(view.getByRole("button", { name: "Fold ai-juggler" }));
  expect(within(head(view, "ai-juggler")).getByTestId("place-needs").textContent).toBe("2");
  expect(view.getByRole("button", { name: "Unfold ai-juggler" }).getAttribute("aria-expanded")).toBe("false");
  expect(view.queryByTestId("rail-folded-needs")).toBeNull();
});

it("a broken session marks the project head red and says what broke", () => {
  const view = board([mk("a", { status: "failed", needs_attention: "failed" }), mk("b", { status: "stuck", needs_attention: "stuck" }), mk("c")]);
  const ai = head(view, "ai-juggler");
  const mark = within(ai).getByTestId("place-needs");
  expect(mark.textContent).toBe("2");
  // D-117: the number in ink, its highest level on its mark (a red dot, L3).
  expect(mark.getAttribute("data-level")).toBe("3");
  expect(mark.querySelector(".level-mark[data-level='3']")).toBeTruthy();
  expect(ai.getAttribute("data-level")).toBe("3");
  expect(ai).toBe(view.getByRole("button", { name: "ai-juggler, 3 sessions, 2 need you, 1 stopped with an error" }));
  // One broken alone: "1 needs you".
  cleanup();
  const one = board([mk("a", { status: "failed", needs_attention: "failed" }), mk("b")]);
  expect(head(one, "ai-juggler")).toBe(one.getByRole("button", { name: "ai-juggler, 2 sessions, 1 needs you, 1 stopped with an error" }));
});
