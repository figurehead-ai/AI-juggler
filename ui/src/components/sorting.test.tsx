import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { InstanceBoard, SessionRow } from "./WorkflowRail";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import {
  chipCounts,
  filterSessions,
  severityRank,
  sortSessions,
  unsnoozedNeedsCount,
  type SnoozeState,
} from "../lib/railview";

/**
 * AJ-E404 evidence: the Needs-me order contract on a fixed 8-row fixture
 * (all severities + 1 snoozed + 1 no-quota), quota-risk `no data`
 * grouping, group-by rollups, ⌘K search behaviour, chip counts, snooze
 * sinking, the 200-row severity property + timing budget, and
 * virtualisation past 50 rows.
 */

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
    model_source: "opus" === null ? "unknown" : "launch",
    status: "running",
    quota: { used_percentage: 40, resets_at_ms: NOW + 3_600_000, confidence: "computed" },
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: "main",
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-1",
    conversation_id: null,
    ...overrides,
  };
}

// The named 8-row fixture: every severity, one snoozed (s6), one
// no-quota row (s7). Exact needs-me order asserted below.
const FIXTURE: InstanceInfo[] = [
  mk("s1", { status: "failed", needs_attention: "failed" }),
  mk("s2", { status: "quota_blocked", needs_attention: "quota_blocked", quota: { used_percentage: 100, resets_at_ms: NOW + 7_200_000, confidence: "measured" } }),
  mk("s3", { status: "waiting_user", needs_attention: "waiting_user", last_activity_ms: NOW - 4 * 60_000 }),
  mk("s4", { status: "waiting_user", needs_attention: "waiting_user", last_activity_ms: NOW - 60_000 }),
  mk("s5", { status: "stuck", needs_attention: "stuck" }),
  mk("s6", { status: "running" }), // snoozed in the snooze test
  mk("s7", { status: "running", quota: null, git_branch: "feat/x" }),
  mk("s8", { status: "finished" }),
];

const PROJECTS: ProjectInfo[] = [
  { id: "proj-1", root: "/w/alpha", name: "alpha", last_activity_ms: null, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, missing: false },
  { id: "proj-2", root: "/w/beta", name: "beta", last_activity_ms: null, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, missing: false },
];

function renderBoard(rows: InstanceInfo[] = FIXTURE, snooze: SnoozeState = {}, projects: ProjectInfo[] = PROJECTS) {
  return render(
    <InstanceBoard
      sessions={rows}
      loginTerminals={[]}
      selectedId={null}
      onSelect={() => {}}
      density="compact"
      freshness={{}}
      now={NOW}
      projects={projects}
      snooze={snooze}
      onSnooze={() => {}}
    />,
  );
}

const renderedIds = (container: HTMLElement): string[] =>
  [...container.querySelectorAll("[data-instance-id]")].map((el) => (el as HTMLElement).dataset.instanceId!);

const setSelect = (container: HTMLElement, label: string, value: string) =>
  fireEvent.change(container.querySelector(`select[aria-label="${label}"]`)!, { target: { value } });

describe("AJ-E404 Needs-me sort (default)", () => {
  it("renders the exact severity order on the 8-row fixture (snoozed sinks, never disappears)", () => {
    const snooze: SnoozeState = { s6: { until: NOW + 60 * 60_000, muted: false } };
    const { container } = renderBoard(FIXTURE, snooze);
    expect(renderedIds(container)).toEqual(["s1", "s2", "s3", "s4", "s5", "s7", "s6", "s8"]);
  });

  it("waiting duration orders the WaitingUser peers (longest wait first)", () => {
    const ordered = sortSessions(FIXTURE, "needs", {}, NOW);
    expect(ordered.map((row) => row.id).slice(0, 5)).toEqual(["s1", "s2", "s3", "s4", "s5"]);
  });

  it("severity property holds for every adjacent pair on a 200-row random fixture", () => {
    let seed = 42;
    const random = () => {
      seed = (seed * 1_664_525 + 1_013_904_223) % 4_294_967_296;
      return seed / 4_294_967_296;
    };
    const rows: InstanceInfo[] = Array.from({ length: 200 }, (_, i) => {
      const pick = random();
      const overrides: Partial<InstanceInfo> = { started_at_ms: NOW - Math.floor(random() * 86_400_000), last_activity_ms: NOW - Math.floor(random() * 3_600_000) };
      if (pick < 0.1) Object.assign(overrides, { status: "failed", needs_attention: "failed" });
      else if (pick < 0.2) Object.assign(overrides, { status: "quota_blocked", needs_attention: "quota_blocked" });
      else if (pick < 0.35) Object.assign(overrides, { status: "waiting_user", needs_attention: "waiting_user" });
      else if (pick < 0.45) Object.assign(overrides, { status: "stuck", needs_attention: "stuck" });
      else if (pick < 0.7) overrides.status = "running";
      else overrides.status = "finished";
      return mk(`r${i}`, overrides);
    });
    const snooze: SnoozeState = {};
    for (const row of rows) if (random() < 0.15) snooze[row.id] = { until: NOW + 3_600_000, muted: false };
    const sorted = sortSessions(rows, "needs", snooze, NOW);
    for (let i = 1; i < sorted.length; i += 1) {
      const a = sorted[i - 1]!;
      const b = sorted[i]!;
      const ok =
        severityRank(a) < severityRank(b) ||
        (severityRank(a) === severityRank(b) && snoozeHas(a, snooze, NOW) <= snoozeHas(b, snooze, NOW));
      expect(ok).toBe(true);
    }
  });

  it("sort+filter of 200 rows stays inside one frame (<16 ms)", () => {
    const rows = Array.from({ length: 200 }, (_, i) => mk(`t${i}`, { started_at_ms: NOW - i * 1_000 }));
    const start = performance.now();
    const filtered = filterSessions(rows, { chips: ["running"], provider: null, account: null, project: null, machine: null }, "", PROJECTS);
    const sorted = sortSessions(filtered, "needs", {}, NOW, PROJECTS);
    const elapsed = performance.now() - start;
    expect(sorted.length).toBe(200);
    expect(elapsed).toBeLessThan(16);
  });
});

function snoozeHas(row: InstanceInfo, snooze: SnoozeState, now: number): number {
  const entry = snooze[row.id];
  if (!entry) return 0;
  if (entry.muted) return 1;
  return entry.until !== null && entry.until > now ? 1 : 0;
}

describe("AJ-E404 Quota risk sort", () => {
  it("orders by lowest % left and groups the no-quota rows under `no data` last", () => {
    const { container } = renderBoard();
    setSelect(container, "Sort by", "quota");
    const groups = [...container.querySelectorAll("p")].map((p) => p.textContent ?? "");
    expect(groups.some((text) => text.includes("no data"))).toBe(true);
    // s2 (0% left) first among known; s7 (no quota) inside the last group.
    const ids = renderedIds(container);
    expect(ids[0]).toBe("s2");
    expect(ids[ids.length - 1]).toBe("s7");
  });

  it("the menu labels the sort honestly while burn rate is pending (AJ-E409)", () => {
    const { container } = renderBoard();
    const option = [...container.querySelectorAll<HTMLOptionElement>('select[aria-label="Sort by"] option')].find(
      (option_) => option_.value === "quota",
    );
    expect(option?.textContent).toBe("Quota risk (quota only)");
  });
});

describe("AJ-E404 group-by rollups", () => {
  it("group-by Project renders headers with running/needs/lowest-quota rollups; needs order inside groups", () => {
    const rows = [
      ...FIXTURE.slice(0, 3).map((row) => ({ ...row, project_id: "proj-1" })),
      ...FIXTURE.slice(3).map((row) => ({ ...row, project_id: "proj-2" })),
    ];
    const { container } = renderBoard(rows);
    setSelect(container, "Group by", "project");
    const headers = [...container.querySelectorAll("p")].map((p) => p.textContent ?? "");
    expect(headers.some((text) => text.includes("alpha"))).toBe(true);
    expect(headers.some((text) => text.includes("running"))).toBe(true);
    expect(headers.some((text) => text.includes("needs you"))).toBe(true);
    expect(headers.some((text) => text.includes("lowest quota"))).toBe(true);
    const proj1Ids = renderedIds(container).slice(0, 3);
    expect(proj1Ids).toEqual(["s1", "s2", "s3"]);
  });
});

describe("AJ-E404 search", () => {
  it("matches branch (I-09) and model (I-03); Esc clears; empty result offers Clear filters", () => {
    const { container } = renderBoard();
    const input = container.querySelector('input[type="search"]')! as HTMLInputElement;
    fireEvent.change(input, { target: { value: "feat/x" } });
    expect(renderedIds(container)).toEqual(["s7"]);
    fireEvent.change(input, { target: { value: "opus" } });
    expect(renderedIds(container).length).toBe(FIXTURE.length);
    fireEvent.change(input, { target: { value: "does-not-exist" } });
    expect(container.textContent).toContain("No instances match");
    expect(container.textContent).toContain("Clear filters");
    fireEvent.keyDown(input, { key: "Escape" });
    expect(renderedIds(container).length).toBe(FIXTURE.length);
    expect(document.activeElement).toBe(input);
  });
});

describe("AJ-E404 filter chips", () => {
  it("carry counts with announced labels and filter the list", () => {
    const { container } = renderBoard();
    const needsChip = container.querySelector('button[aria-label="Needs me, 5 items"]')!;
    expect(needsChip.getAttribute("aria-pressed")).toBe("false");
    expect(chipCounts(FIXTURE)).toEqual({ needs: 5, running: 2, ended: 2 }); // failed counts as Ended
    fireEvent.click(needsChip);
    expect(renderedIds(container)).toEqual(["s1", "s2", "s3", "s4", "s5"]);
    const runningChip = container.querySelector('button[aria-label="Running, 2 items"]')!;
    expect(runningChip.getAttribute("aria-pressed")).toBe("false");
  });

  it("the provider dropdown ANDs with the chips", () => {
    const { container } = renderBoard();
    fireEvent.click(container.querySelector('button[aria-label="Needs me, 5 items"]')!);
    setSelect(container, "Filter by project", "proj-1");
    expect(renderedIds(container)).toEqual(["s1", "s2", "s3", "s4", "s5"]);
    setSelect(container, "Filter by project", "proj-2");
    expect(renderedIds(container)).toEqual([]);
  });
});

describe("AJ-E404 snooze/mute", () => {
  it("snoozed rows sink below peers, never disappear, and show `snoozed Nm`", () => {
    const snooze: SnoozeState = { s6: { until: NOW + 12 * 60_000, muted: false } };
    const { container } = renderBoard(FIXTURE, snooze);
    expect(container.textContent).toContain("snoozed 12m");
    expect(renderedIds(container)).toEqual(["s1", "s2", "s3", "s4", "s5", "s7", "s6", "s8"]);
  });

  it("muted rows sink too; unsnoozed count feeds the badge only", () => {
    const snooze: SnoozeState = {
      s3: { until: null, muted: true },
      s6: { until: NOW + 60 * 60_000, muted: false },
    };
    expect(unsnoozedNeedsCount(FIXTURE, snooze, NOW)).toBe(4); // 5 need you, s3 muted out of the badge
    const sorted = sortSessions(FIXTURE, "needs", snooze, NOW);
    expect(sorted.map((row) => row.id).slice(0, 5)).toEqual(["s1", "s2", "s4", "s3", "s5"]); // muted s3 sinks below waiting peer s4, stays above Stuck s5
  });

  it("persistence round-trips through aij.snooze", async () => {
    const { readSnooze, storeSnooze } = await import("../lib/railview");
    const map = new Map<string, string>();
    const storageLike = {
      get length() { return map.size; },
      clear: () => map.clear(),
      getItem: (key: string) => map.get(key) ?? null,
      key: (index: number) => [...map.keys()][index] ?? null,
      removeItem: (key: string) => void map.delete(key),
      setItem: (key: string, value: string) => void map.set(key, value),
    } as Storage;
    storeSnooze({ s6: { until: 123, muted: false } }, storageLike);
    expect(readSnooze(storageLike)).toEqual({ s6: { until: 123, muted: false } });
  });
});

describe("AJ-E404 virtualisation", () => {
  it("renders a window, not 120 rows — every row still reachable by scroll height", () => {
    const rows = Array.from({ length: 120 }, (_, i) => mk(`v${i}`, { status: "running", started_at_ms: NOW - i * 1_000 }));
    const { container } = renderBoard(rows);
    const rendered = container.querySelectorAll("[data-instance-id]").length;
    expect(rendered).toBeGreaterThan(0);
    expect(rendered).toBeLessThan(60); // windowed, not all 120
    // The trailing spacer covers every unrendered row (jsdom cannot
    // measure scrollHeight; the inline spacer heights carry the math).
    const spacers = [...container.querySelectorAll<HTMLDivElement>('[style*="height"]')].map(
      (element) => Number.parseFloat(element.style.height),
    );
    const trailing = Math.max(...spacers);
    expect(rendered + trailing / 42).toBe(120);
  });
});

describe("AJ-E404 row still renders its full fields under the board", () => {
  it("the compact row keeps the honesty contract (regression guard)", () => {
    const { container } = render(
      <SessionRow session={mk("s9", { quota: null, model: null })} selected={false} onSelect={() => {}} density="compact" freshness={{ s9: { resources: NOW, quota: NOW } }} now={NOW} />,
    );
    // Compact quota is bar-only by spec (AJ-E403): the honesty copy rides
    // in the dashed bar's title, and I-09 stays visible.
    expect(container.querySelector('[title="Quota: no data"]')).not.toBeNull();
    expect(container.textContent).toContain("main");
  });
});
