/**
 * AJ-E1005 vitest — morning triage: four groups by the action needed,
 * text-labelled; sorts (waiting longest, project, quota risk); one
 * primary action per row run in place; keyboard only (j/k, Enter, A);
 * a finished Instance leaves only through "Mark reviewed"; 50 Instances
 * render well inside 300 ms.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { Triage, primaryAction, sortRows, triageGroup } from "./Triage";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

const NOW = 1_800_000_000_000;
const w = () => window as unknown as Record<string, unknown>;
const calls = () => [
  ...((w().__AIJ_TRIAGE_CALLS__ as string[] | undefined) ?? []),
  ...((w().__AIJ_PARK_CALLS__ as string[] | undefined) ?? []),
];

const row = (id: string, status: string, over: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({
    id,
    status,
    provider: "claude-code",
    account_id: "acc",
    account_label: `Acct ${id}`,
    model: null,
    launch_model: null,
    running_model: null,
    model_source: "unknown",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 600_000,
    machine_id: "this-mac",
    kind: "terminal",
    isolation_label: null,
    project_id: null,
    conversation_id: null,
    adopted: false,
    launch_recipe_id: "p-1",
    cwd: "/w",
    ...over,
  }) as InstanceInfo;

const projects = [
  { id: "pa", name: "alpha" },
  { id: "pz", name: "zulu" },
] as ProjectInfo[];

afterEach(cleanup);
beforeEach(() => {
  delete w().__AIJ_TRIAGE_CALLS__;
  delete w().__AIJ_PARK_CALLS__;
});

describe("AJ-E1005 grouping and actions", () => {
  it("groups by what is needed; reviewed, handed-over and old ended rows are not triage work", () => {
    expect(triageGroup(row("a", "waiting_user"), NOW)).toBe("needs_you");
    expect(triageGroup(row("b", "failed"), NOW)).toBe("needs_you");
    expect(triageGroup(row("c", "quota_blocked", { parked: "measured", parked_resets_at_ms: NOW + 1 }), NOW)).toBe("parked");
    expect(triageGroup(row("d", "running"), NOW)).toBe("running");
    expect(triageGroup(row("e", "finished"), NOW)).toBe("finished");
    expect(triageGroup(row("f", "finished", { lifecycle: "reviewed" }), NOW)).toBeNull();
    expect(triageGroup(row("g", "handed_over"), NOW)).toBeNull();
    expect(triageGroup(row("h", "stopped", { last_activity_ms: NOW - 8 * 24 * 3_600_000 }), NOW)).toBeNull();
    expect(primaryAction(row("q", "quota_blocked"), NOW).label).toBe("Park until reset");
    expect(primaryAction(row("p", "quota_blocked", { parked: "measured", parked_resets_at_ms: NOW - 1 }), NOW).label).toBe("Resume");
    expect(primaryAction(row("e", "finished"), NOW).label).toBe("Mark reviewed");
    expect(primaryAction(row("a", "stuck"), NOW).label).toBe("Open");
  });

  it("sorts oldest blocker first, by project, and by quota risk", () => {
    const rows = [
      row("new", "stuck", { attention: { rule: "stuck_no_output", inputs: [], since_ms: NOW - 60_000 }, project_id: "pa" }),
      row("old", "waiting_user", { attention: { rule: "waiting_prompt", inputs: [], since_ms: NOW - 3_600_000 }, project_id: "pz" }),
      row("hot", "running", { quota: { used_percentage: 92, resets_at_ms: null, confidence: "measured" }, project_id: "pz" }),
    ];
    const name = (s: InstanceInfo) => (s.project_id === "pa" ? "alpha" : "zulu");
    expect(sortRows(rows, "waiting", name).map((r) => r.id)).toEqual(["old", "hot", "new"]);
    expect(sortRows(rows, "project", name).map((r) => r.id)[0]).toBe("new");
    expect(sortRows(rows, "quota", name).map((r) => r.id)[0]).toBe("hot");
  });
});

describe("AJ-E1005 the triage view", () => {
  it("is keyboard operable: j/k move, A runs the action, Enter opens", async () => {
    const sessions = [
      row("s-wait", "waiting_user", { attention: { rule: "waiting_prompt", inputs: [], since_ms: NOW - 3_600_000 } }),
      row("s-quota", "quota_blocked"),
      row("s-done", "finished"),
    ];
    const opened: string[] = [];
    let changed = 0;
    const { container } = render(
      <Triage
        sessions={sessions}
        projects={projects}
        density="comfortable"
        onOpenSession={(id) => opened.push(id)}
        onChanged={() => changed++}
        now={NOW}
      />,
    );
    const rows = () => Array.from(container.querySelectorAll<HTMLElement>("[data-testid='triage-row']"));
    expect(rows().map((r) => r.dataset.triageRow)).toEqual(["s-wait", "s-quota", "s-done"]);
    expect(container.querySelector("[data-testid='triage-group-parked']")!.textContent).toContain("Nothing parked.");
    const first = rows()[0].querySelector("button")!;
    first.focus();
    fireEvent.keyDown(first, { key: "j" });
    expect(rows()[1].contains(document.activeElement)).toBe(true);
    await act(async () => {
      fireEvent.keyDown(document.activeElement!, { key: "a" });
    });
    expect(calls()).toEqual(["park:s-quota"]);
    fireEvent.keyDown(document.activeElement!, { key: "ArrowDown" });
    await act(async () => {
      fireEvent.keyDown(document.activeElement!, { key: "A" });
    });
    expect(calls()).toContain("reviewed:s-done");
    expect(changed).toBe(2);
    fireEvent.keyDown(document.activeElement!, { key: "k" });
    fireEvent.keyDown(document.activeElement!, { key: "k" });
    fireEvent.click(document.activeElement!);
    expect(opened).toEqual(["s-wait"]);
    expect(container.querySelectorAll("[data-testid='triage-action']")[0].textContent).toBe("OpenA");
  });

  it("says so when there is nothing to triage", () => {
    const { container } = render(
      <Triage sessions={[]} projects={[]} density="compact" onOpenSession={() => {}} onChanged={() => {}} now={NOW} />,
    );
    expect(container.querySelector("[data-testid='triage-empty']")).not.toBeNull();
  });

  it("renders 50 Instances well inside 300 ms", () => {
    const statuses = ["waiting_user", "stuck", "running", "finished", "quota_blocked"];
    const sessions = Array.from({ length: 50 }, (_, n) => row(`s-${n}`, statuses[n % 5]));
    const started = performance.now();
    const { container } = render(
      <Triage sessions={sessions} projects={projects} density="comfortable" onOpenSession={() => {}} onChanged={() => {}} now={NOW} />,
    );
    const took = performance.now() - started;
    expect(container.querySelectorAll("[data-testid='triage-row']")).toHaveLength(50);
    expect(took).toBeLessThan(300);
  });
});
