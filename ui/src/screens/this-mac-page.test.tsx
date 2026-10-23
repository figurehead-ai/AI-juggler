/**
 * AJ-E1528 vitest — the This Mac page: the Mac's state in words and
 * meters, what's using it by tool and then by session or window (tools AI
 * Juggler didn't start marked, with no controls), what's working it hard
 * with links, alerts in the one subject's words, advice and leftovers.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ThisMacScreen, treesOf } from "./ThisMacScreen";
import { ThisMacContext, mockMachineHealth, type ThisMacInfo, type ToolTreeInfo } from "../lib/machine";
import type { OverloadAlertInfo } from "../types/generated/OverloadAlertInfo";

const NOW = 1_728_800_000_000;

const tree = (overrides: Partial<ToolTreeInfo>): ToolTreeInfo => ({
  root_pid: null,
  session_id: null,
  place: "desktop",
  tool: null,
  group: overrides.session_id ? "claude-code" : (overrides.place ?? "desktop"),
  account_label: "Personal",
  cpu_pct: 14,
  rss_bytes: 1_600_000_000,
  helpers: 21,
  heavy_note: null,
  memory_estimate: !overrides.session_id,
  ...overrides,
});

const totals = (list: ToolTreeInfo[]) => ({
  count: list.length,
  cpu_pct: 5,
  rss_bytes: list.reduce((sum, t) => sum + t.rss_bytes, 0),
  memory_estimate: list.some((t) => t.memory_estimate),
});

function info(trees: ToolTreeInfo[], extra: Partial<ThisMacInfo> = {}): ThisMacInfo {
  const ours = trees.filter((t) => t.session_id);
  const desktop = trees.filter((t) => t.place === "desktop");
  return {
    machine: mockMachineHealth("tight", NOW),
    tools: totals(trees),
    ours: totals(ours),
    theirs: totals(desktop),
    by_tool: [
      { tool: "claude-code", label: "Claude Code", ours: true, totals: totals(ours), cpu_one_core_pct: 30 },
      { tool: "desktop", label: "Claude app", ours: false, totals: totals(desktop), cpu_one_core_pct: 14 },
    ],
    top: trees.slice(0, 3),
    others: trees.slice(3),
    heavy: trees.filter((t) => t.heavy_note),
    leftover_processes: 0,
    leftover_ports: 0,
    alerts: [],
    recommendations: [],
    awake: { working: 0, kept_awake: false, on_battery: false },
    own: { rss_bytes: 214_000_000, watchdog: true, opens_at_login: false },
    ...extra,
  };
}

const ours = (id: string, overrides: Partial<ToolTreeInfo> = {}) =>
  tree({ root_pid: 1000 + id.length, session_id: id, place: null, tool: "claude-code", group: "claude-code", account_label: "Work Max", ...overrides });

function renderPage(value: ThisMacInfo | null, handlers: { open?: (id: string) => void; cleanup?: () => void } = {}) {
  return render(
    <ThisMacContext.Provider value={value}>
      <ThisMacScreen nameOf={(id) => `branch-${id}`} onOpenSession={handlers.open ?? vi.fn()} onOpenCleanup={handlers.cleanup} />
    </ThisMacContext.Provider>,
  );
}

afterEach(cleanup);

describe("the This Mac page", () => {
  it("says it's measuring before the first sample", () => {
    renderPage(null);
    expect(screen.getByTestId("this-mac-page").textContent).toContain("This Mac");
    expect(screen.queryByTestId("this-mac-state")).toBeNull();
  });

  it("shows the Mac's state in words and meters", () => {
    renderPage(info([ours("a")]));
    const state = screen.getByTestId("this-mac-state");
    expect(within(state).getByRole("status").textContent).toContain("Getting tight");
    expect(within(state).getByRole("meter")).toBeTruthy();
    expect(screen.getByTestId("this-mac-figures").textContent).toMatch(/CPU\d+% of all \d+ cores/);
  });

  it("groups what's using it by tool, then session or window; another tool's windows are marked and have no controls", () => {
    const trees = [ours("a"), ours("b"), tree({ root_pid: 501 }), tree({ root_pid: 502, rss_bytes: 900_000_000 })];
    const value = info(trees);
    expect(treesOf(value.by_tool[0]!, value).map((t) => t.session_id)).toEqual(["a", "b"]);
    expect(treesOf(value.by_tool[1]!, value).map((t) => t.root_pid)).toEqual([501, 502]);
    renderPage(value);
    const groups = screen.getAllByTestId("this-mac-group");
    expect(groups).toHaveLength(2);
    expect(groups[0]!.textContent).toContain("2 sessions");
    expect(groups[0]!.hasAttribute("open")).toBe(true);
    expect(groups[1]!.textContent).toContain("2 windows · not started by AI Juggler");
    expect(groups[1]!.hasAttribute("open")).toBe(false);
    // D-08: a window AI Juggler didn't start is shown, never offered.
    const theirs = within(groups[1]!).getAllByTestId("this-mac-tree");
    for (const row of theirs) expect(within(row).queryByRole("button")).toBeNull();
  });

  it("links what's working it hard, and leads to Cleanup for leftovers", () => {
    const open = vi.fn();
    const clean = vi.fn();
    renderPage(info([ours("hot", { heavy_note: "CPU 96% for a minute" })], { leftover_processes: 2, leftover_ports: 1 }), { open, cleanup: clean });
    const heavy = screen.getByTestId("this-mac-heavy");
    fireEvent.click(within(heavy).getByRole("button", { name: /^branch-hot/ }));
    expect(open).toHaveBeenCalledWith("hot");
    expect(screen.getByTestId("this-mac-leftovers-page").textContent).toContain("2 processes left running after their session closed · holding a port");
    fireEvent.click(screen.getByRole("button", { name: "Clean up" }));
    expect(clean).toHaveBeenCalledOnce();
  });

  it("says each alert with its subject in the one function's words, and the advice", () => {
    const alert: OverloadAlertInfo = {
      key: "pressure:a",
      kind: "heavy",
      title: "A session is using a lot of memory",
      detail: "Memory is getting tight",
      session_id: "a",
      place: null,
      subject_bytes: 3_200_000_000,
      subject_estimate: false,
      subject_cpu: null,
      subject_windows: null,
      since_ms: NOW - 60_000,
    } as OverloadAlertInfo;
    renderPage(info([ours("a")], { alerts: [alert], recommendations: ["Codex sessions here usually peak at 2 GB"] }));
    expect(screen.getByTestId("this-mac-alert").textContent).toContain("branch-a is using 3.2 GB");
    expect(screen.getByTestId("this-mac-recommendations").textContent).toContain("Codex sessions here usually peak at 2 GB.");
  });
});
