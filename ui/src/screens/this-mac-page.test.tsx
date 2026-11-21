/**
 * AJ-E1528 vitest — the This Mac page: the Mac's state in words and
 * meters, what's using it by tool and then by session or window (tools AI
 * Juggler didn't start marked, with no controls), what's working it hard
 * with links, alerts in the one subject's words, advice and leftovers.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ThisMacScreen, treesOf } from "./ThisMacScreen";
import { ThisMacContext, type ThisMacInfo, type ToolTreeInfo } from "../lib/machine";
import { mockMachineHealth } from "../preview/machine";

/** A group's fold: the app's disclosure, a button an accessibility press
 * opens (the 0.4.6 rc2 walk P9). */
const foldOf = (group: HTMLElement): HTMLButtonElement => group.querySelector<HTMLButtonElement>("[data-disclosure] > button[aria-expanded]")!;
import type { OverloadAlertInfo } from "../types/generated/OverloadAlertInfo";

const NOW = 1_728_800_000_000;

const tree = (overrides: Partial<ToolTreeInfo>): ToolTreeInfo => ({
  root_pid: null,
  session_id: null,
  place: "desktop",
  label: overrides.session_id ? null : "Claude app",
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
      { tool: "claude-code", label: "Claude Code", who: null, ours: true, totals: totals(ours), cpu_one_core_pct: 30 },
      { tool: "desktop", label: "Claude app", who: "The Claude app", ours: false, totals: totals(desktop), cpu_one_core_pct: 14 },
    ],
    top: trees.slice(0, 3),
    others: trees.slice(3),
    heavy: trees.filter((t) => t.heavy_note),
    other_apps: [],
    leftover_processes: 0,
    leftover_ports: 0,
    scanned: true,
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
    expect(within(state).getAllByRole("meter").length).toBeGreaterThan(0);
    // The Mac's figures, each a card sized to its content.
    expect(screen.getByTestId("this-mac-cpu").textContent).toMatch(/Processor\d+% of all \d+ cores/);
    expect(screen.getByTestId("this-mac-memory-card").textContent).toContain("in use of 32 GB");
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
    expect(foldOf(groups[0]!).getAttribute("aria-expanded")).toBe("true");
    expect(groups[1]!.textContent).toContain("2 windows · not started by AI Juggler");
    expect(foldOf(groups[1]!).getAttribute("aria-expanded")).toBe("false");
    // D-08: a window AI Juggler didn't start is shown, never offered.
    const theirs = within(groups[1]!).getAllByTestId("this-mac-tree");
    for (const row of theirs) expect(within(row).queryByRole("button")).toBeNull();
  });

  // The maintainer's report (0.1.6): the Codex app was running and This
  // Mac listed only the Claude app and other apps. Each tool's app is its
  // own group, in its adapter's words, and so is each tool in a terminal.
  it("lists every tool's app and terminal sessions in their own groups", () => {
    const codex = tree({ root_pid: 601, group: "codex-app", label: "ChatGPT (Codex)", tool: "codex", account_label: null });
    const terminal = tree({ root_pid: 701, place: "terminal", group: "terminal:claude-code", label: "claude in a terminal", tool: "claude-code" });
    const value = info([ours("a"), tree({ root_pid: 501, group: "claude-app" }), codex, terminal], {
      by_tool: [
        { tool: "claude-code", label: "Claude Code", who: null, ours: true, totals: totals([ours("a")]), cpu_one_core_pct: 30 },
        { tool: "claude-app", label: "Claude app", who: "The Claude app", ours: false, totals: totals([tree({})]), cpu_one_core_pct: 14 },
        { tool: "codex-app", label: "ChatGPT (Codex)", who: "ChatGPT (Codex)", ours: false, totals: totals([codex]), cpu_one_core_pct: 14 },
        { tool: "terminal:claude-code", label: "claude in a terminal", who: "claude in a terminal", ours: false, totals: totals([terminal]), cpu_one_core_pct: 14 },
      ],
    });
    renderPage(value);
    const groups = screen.getAllByTestId("this-mac-group");
    expect(groups.map((group) => foldOf(group).textContent ?? "")).toEqual([
      expect.stringContaining("Claude Code"),
      expect.stringContaining("Claude app"),
      expect.stringContaining("ChatGPT (Codex)"),
      expect.stringContaining("claude in a terminal"),
    ]);
    expect(within(groups[2]!).getByTestId("this-mac-tree").textContent).toContain("ChatGPT (Codex)");
    expect(within(groups[3]!).getByTestId("this-mac-tree").textContent).toContain("claude in a terminal");
  });

  // AJ-E2407: the groups come from the Processes view's graph, so another
  // app's AI sessions (any tool) are listed by app and session title.
  it("lists another app's AI sessions by app, each by its title, and names it in an alert", () => {
    const chatgpt = tree({ root_pid: 7300, place: "desktop", tool: "codex", group: "app:ChatGPT:codex", label: "ChatGPT", title: "Landing page", rss_bytes: 195_000_000 });
    const value = info([ours("a"), chatgpt], {
      by_tool: [
        { tool: "claude-code", label: "Claude Code", who: null, ours: true, totals: totals([ours("a")]), cpu_one_core_pct: 30 },
        { tool: "app:ChatGPT:codex", label: "ChatGPT", who: "ChatGPT", ours: false, totals: totals([chatgpt]), cpu_one_core_pct: 1 },
      ],
    });
    renderPage(value);
    const groups = screen.getAllByTestId("this-mac-group");
    expect(groups[1]!.textContent).toContain("ChatGPT");
    expect(groups[1]!.textContent).toContain("1 session · not started by AI Juggler");
    expect(within(groups[1]!).getByTestId("this-mac-tree").textContent).toContain("Landing page");
    cleanup();
    const alert = {
      key: "cpu",
      kind: "cpu",
      title: "The processor has been flat out for minutes",
      detail: "97% busy for over two minutes",
      session_id: null,
      place: "app:ChatGPT:codex",
      subject_name: "ChatGPT",
      subject_bytes: null,
      subject_estimate: false,
      subject_cpu: 140,
      subject_windows: 2,
      other_apps: [],
      since_ms: NOW - 60_000,
    } as OverloadAlertInfo;
    renderPage(info([ours("a"), chatgpt], { alerts: [alert] }));
    expect(screen.getByTestId("this-mac-alert").textContent).toContain("ChatGPT is using CPU 140% across 2 sessions");
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

  // The 0.1.4 review: "Under memory pressure" beside "AI tools: about 16
  // GB of 103 GB" read as plenty left.
  it("shows the whole Mac's memory, explains pressure, and lists what else uses it by name, read-only", () => {
    const windows = [4_100_000_000, 3_600_000_000, 2_900_000_000, 2_300_000_000, 1_800_000_000, 1_300_000_000].map((rss, at) =>
      tree({ root_pid: 700 + at, rss_bytes: rss }),
    );
    const other_apps = [
      { name: "next-server", rss_bytes: 6_400_000_000, processes: 1 },
      { name: "Docker", rss_bytes: 4_000_000_000, processes: 3 },
      { name: "fseventsd", rss_bytes: 1_500_000_000, processes: 1 },
    ];
    renderPage(info(windows, { machine: mockMachineHealth("others", NOW), other_apps }));
    const state = screen.getByTestId("this-mac-state");
    const parts = [...within(state).getByTestId("this-mac-memory-parts").querySelectorAll("li")].map((part) => part.textContent);
    expect(parts).toEqual(["AI tools about 15 GB", "Other apps and macOS 22 GB", "Kept compressed 47 GB", "Available 12 GB"]);
    expect(within(state).getByTestId("this-mac-memory").textContent).toBe("84 GB in use of 96 GB");
    expect(screen.getByTestId("this-mac-swap").textContent).toContain("21 GB of 22 GB moved to disk");
    // A free figure under pressure never stands alone: the line, and the
    // rest behind "What it means".
    const note = within(state).getByTestId("this-mac-pressure-note");
    expect(note.textContent).toContain("Memory pressure is macOS's own measure of how hard it is working to make room.");
    expect(within(note).getByRole("button", { name: "What it means" }).getAttribute("aria-expanded")).toBe("false");
    expect(note.textContent).toContain("Pressure can be high while some memory still looks free");
    // What else: folded, by name, its figure the rest of what's in use.
    const group = screen.getByTestId("this-mac-other-apps");
    const fold = within(group).getByRole("button", { name: /Other apps and macOS/ });
    expect(fold.getAttribute("aria-expanded")).toBe("false");
    expect(fold.textContent).toContain("22 GB");
    expect(group.textContent).toContain("macOS keeps 47 GB more compressed for every app, AI tools too");
    const rows = within(group).getAllByTestId("this-mac-other-app");
    expect(rows.map((row) => row.textContent)).toEqual(["next-server6.0 GB", "Dockerabout 3.7 GB3 processes", "fseventsd1.4 GB"]);
    // D-08: another app is never offered anything.
    expect(within(group).getAllByRole("button", { hidden: true })).toEqual([fold]);
  });

  // 0.4.7 walk V2: the Overview opened on an empty Memory card,
  // "Processor — Not measured yet" and "No AI tools are running" with two
  // sessions running. Before the first sample it says it's measuring;
  // before the first look for tools, that it's looking, never "none".
  it("says it's measuring, never empty figures or no tools, before its first readings", () => {
    renderPage(info([], { machine: mockMachineHealth("unsampled", NOW), by_tool: [], scanned: false }));
    expect(screen.getByRole("status", { name: "Measuring this Mac" })).toBeTruthy();
    expect(screen.queryByTestId("this-mac-memory-card")).toBeNull();
    expect(document.body.textContent).not.toContain("Not measured yet");
    expect(document.body.textContent).not.toContain("No AI tools are running");
    cleanup();
    renderPage(info([], { machine: mockMachineHealth("plenty", NOW), by_tool: [], scanned: false }));
    const using = screen.getByTestId("this-mac-using");
    expect(within(using).getByRole("status", { name: "Looking for running AI tools" })).toBeTruthy();
    expect(using.textContent).not.toContain("No AI tools are running");
    expect(screen.getByTestId("this-mac-state").textContent).not.toContain("Nothing running");
    cleanup();
    renderPage(info([], { machine: mockMachineHealth("plenty", NOW), by_tool: [] }));
    expect(screen.getByTestId("this-mac-using").textContent).toContain("No AI tools are running.");
    expect(screen.getByTestId("this-mac-state").textContent).toContain("Nothing running — your Mac has room");
  });

  // D1 (0.4.8's persona run): "Plenty of room" in green beside swap at
  // 8.2 of 9.0 GB read as wrong. The card says why it doesn't count yet,
  // in the backend's words; a swap that isn't nearly full keeps the plain
  // line.
  it("says why a nearly full swap leaves plenty of room", () => {
    const GB = 1024 ** 3;
    const note = "Nearly full, but the system makes more swap as it needs it: it counts against room once it holds 24 GB (25% of your memory) or keeps growing.";
    const plenty = mockMachineHealth("plenty", NOW);
    renderPage(info([ours("a")], { machine: { ...plenty, memory_total: 96 * GB, swap_used: 8.2 * GB, swap_total: 9 * GB, swap_note: note } }));
    expect(screen.getByTestId("this-mac-state").textContent).toContain("Plenty of room");
    const swap = screen.getByTestId("this-mac-swap");
    expect(swap.textContent).toContain("8.2 GB of 9.0 GB moved to disk");
    expect(within(swap).getByTestId("this-mac-swap-note").textContent).toBe(note);
    cleanup();
    renderPage(info([ours("a")], { machine: plenty }));
    expect(screen.getByTestId("this-mac-swap-note").textContent).toBe("Memory macOS moved to disk to make room. Reading it back is slow.");
  });

  it("says no word of pressure while macOS reports none", () => {
    renderPage(info([ours("a")], { machine: mockMachineHealth("plenty", NOW) }));
    expect(screen.queryByTestId("this-mac-pressure-note")).toBeNull();
    expect(screen.getByTestId("this-mac-other-apps")).toBeTruthy();
  });

  it("says each alert with its subject in the one function's words, and the advice", () => {
    const alert: OverloadAlertInfo = {
      key: "pressure:a",
      kind: "heavy",
      title: "A session is using a lot of memory",
      detail: "Memory is getting tight",
      session_id: "a",
      place: null,
      subject_name: null,
      subject_bytes: 3_200_000_000,
      subject_estimate: false,
      subject_cpu: null,
      subject_windows: null,
      other_apps: [],
      since_ms: NOW - 60_000,
    } as OverloadAlertInfo;
    renderPage(info([ours("a")], { alerts: [alert], recommendations: ["Codex sessions here usually peak at 2 GB"] }));
    expect(screen.getByTestId("this-mac-alert").textContent).toContain("branch-a is using 3.0 GB of memory");
    expect(screen.getByTestId("this-mac-recommendations").textContent).toContain("Codex sessions here usually peak at 2 GB.");
  });

  /** The 0.2 walk (F13): every alert says what to do, names the app (never
   * a bundle id), says nothing is blocked, and leads to what's using it. */
  it("gives each alert its next step and a way to what's using it", () => {
    const alert = {
      key: "memory_pressure",
      kind: "memory_pressure",
      title: "Your Mac is under memory pressure",
      detail: "macOS reports memory pressure",
      session_id: null,
      place: null,
      subject_name: null,
      subject_bytes: null,
      subject_estimate: false,
      subject_cpu: null,
      subject_windows: null,
      other_apps: [{ name: "Virtual Machine (part of macOS)", rss_bytes: 2_700_000_000, processes: 1 }],
      since_ms: NOW - 60_000,
    } as OverloadAlertInfo;
    renderPage(info([ours("a")], { alerts: [alert] }));
    const next = screen.getByTestId("this-mac-alert-next").textContent!;
    expect(next).toBe("To make room, quit Virtual Machine (part of macOS) or another app you aren't using. Nothing is blocked, but everything is slower until it eases.");
    expect(screen.getByTestId("this-mac-alert").textContent).not.toMatch(/com\.apple/);
    expect(screen.getByTestId("this-mac-alert-using").textContent).toBe("See what's using it");
    screen.getByTestId("this-mac-alert-using").click();
    expect(document.activeElement?.id).toBe("this-mac-using-title");
  });
});
