/**
 * E14 vitest — This Mac on Home (AJ-E1404), the rail's heavy note and the
 * session header's resources (AJ-E1405). Calm until it matters: nothing
 * before the first sample, colour and Needs-you entries only when heavy
 * or under pressure, and a tool AI Juggler didn't start offers nothing to
 * click (D-08).
 */
import { cleanup, fireEvent, render, screen, within, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ThisMacPanel } from "./ThisMacPanel";
import { RailRow } from "./WorkflowRail";
import { HomeScreen } from "../screens/Home";
import { ThisMacContext, keptNotices, mockMachineHealth, type MachineNotice, type ThisMacInfo, type ToolTreeInfo } from "../lib/machine";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";

const NOW = 1_728_800_000_000;

function mk(id: string, overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id,
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: 12,
    rss_bytes: 1_100_000_000,
    child_count: 3,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 3_600_000,
    last_activity_ms: NOW - 60_000,
    machine_id: "this-mac",
    cwd: `/Users/dev/${id}`,
    kind: "terminal",
    isolation_label: null,
    project_id: null,
    conversation_id: null,
    ...overrides,
  } as InstanceInfo;
}

const tree = (overrides: Partial<ToolTreeInfo>): ToolTreeInfo => ({
  root_pid: null,
  session_id: null,
  place: "desktop",
  tool: null,
  group: overrides.place === null ? "claude-code" : (overrides.place ?? "desktop"),
  account_label: "Personal",
  cpu_pct: 14,
  rss_bytes: 1_600_000_000,
  helpers: 21,
  heavy_note: null,
  memory_estimate: overrides.session_id ? false : true,
  ...overrides,
});

function info(level: "plenty" | "tight" | "pressure", trees: ToolTreeInfo[], leftovers: [number, number] = [0, 0]): ThisMacInfo {
  const total = (list: ToolTreeInfo[]) => ({
    count: list.length,
    cpu_pct: 5,
    rss_bytes: list.reduce((sum, t) => sum + t.rss_bytes, 0),
    memory_estimate: list.some((t) => t.memory_estimate),
  });
  return {
    machine: mockMachineHealth(level, NOW),
    tools: total(trees),
    ours: total(trees.filter((t) => t.session_id)),
    theirs: total(trees.filter((t) => !t.session_id)),
    awake: { working: 0, kept_awake: false, on_battery: false },
    by_tool: [],
    top: trees.slice(0, 3),
    others: trees.slice(3),
    heavy: trees.filter((t) => t.heavy_note),
    leftover_processes: leftovers[0],
    leftover_ports: leftovers[1],
    alerts: [],
    recommendations: [],
  };
}

afterEach(cleanup);

describe("This Mac panel (AJ-E1404)", () => {
  const nameOf = (id: string) => `name-of-${id}`;

  it("says nothing before the first sample", () => {
    const unsampled = { ...info("plenty", []), machine: mockMachineHealth("unsampled") };
    const { container } = render(<ThisMacPanel info={unsampled} nameOf={nameOf} onOpenSession={vi.fn()} />);
    expect(container.innerHTML).toBe("");
    const none = render(<ThisMacPanel info={null} nameOf={nameOf} onOpenSession={vi.fn()} />);
    expect(none.container.innerHTML).toBe("");
  });

  it("is calm with room to spare and nothing running", () => {
    render(<ThisMacPanel info={info("plenty", [])} nameOf={nameOf} onOpenSession={vi.fn()} />);
    expect((screen.getByRole("status"))!.textContent).toContain("Nothing running — your Mac has room");
    expect(screen.queryByTestId("this-mac-reasons")).toBeNull();
    expect(screen.queryByTestId("this-mac-leftovers")).toBeNull();
    expect((screen.getByTestId("this-mac-memory"))!.textContent).toContain("Memory: 22 GB of 32 GB in use");
  });

  it("names the state, its reasons, and splits ours from the tools we didn't start", () => {
    const open = vi.fn();
    render(
      <ThisMacPanel
        info={info("tight", [tree({ session_id: "s1", place: null, account_label: "Work Max", rss_bytes: 3_200_000_000, cpu_pct: 96, heavy_note: "CPU 96% · 3.2 GB" }), tree({})])}
        nameOf={nameOf}
        onOpenSession={open}
      />,
    );
    expect((screen.getByRole("status"))!.textContent).toContain("Getting tight");
    // "macOS" keeps its lower-case m in the sentence.
    expect(screen.getByTestId("this-mac-reasons").textContent).toBe("macOS reports memory pressure; swap is growing — 140 MB a minute");
    const [ours, theirs] = screen.getAllByTestId("this-mac-tree");
    fireEvent.click(within(ours!).getAllByRole("button")[0]!);
    expect(open).toHaveBeenCalledWith("s1");
    // AJ-E1537: its menu goes to the session; theirs says it isn't ours.
    fireEvent.contextMenu(ours!);
    expect(screen.getByRole("menuitem", { name: "Go to the session" })).toBeTruthy();
    fireEvent.keyDown(window, { key: "Escape" });
    fireEvent.contextMenu(theirs!);
    const line = screen.getByRole("menuitem", { name: "Not started by AI Juggler" }) as HTMLButtonElement;
    expect(line.disabled).toBe(true);
    fireEvent.keyDown(window, { key: "Escape" });
    expect((ours)!.textContent).toContain("name-of-s1");
    expect((ours)!.textContent).toContain("heavy: CPU 96% · 3.2 GB");
    // D-08: shown and counted, never actionable.
    expect((theirs)!.textContent).toContain("Claude app");
    expect((theirs)!.textContent).toContain("Personal · not started by AI Juggler");
    expect(within(theirs!).queryByRole("button")).toBeNull();
    // Its memory is summed over helpers that share pages: an estimate.
    expect(theirs!.textContent).toContain("about 1.6 GB");
    expect(ours!.textContent).not.toContain("about");
    expect(screen.getByTestId("this-mac-memory").textContent).toContain("AI tools: about 4.8 GB");
  });

  it("names what memory in use counts, and shows nothing when it couldn't be read", () => {
    const measured = info("plenty", []);
    const { unmount } = render(<ThisMacPanel info={measured} nameOf={nameOf} onOpenSession={vi.fn()} />);
    const line = screen.getByTestId("this-mac-memory");
    expect(line.getAttribute("title")).toContain("app memory, wired and compressed");
    expect(screen.getByRole("meter").getAttribute("aria-label")).toContain("close to Activity Monitor");
    unmount();
    const unread = info("plenty", []);
    unread.machine = { ...unread.machine, memory_used: null, memory_basis: null };
    render(<ThisMacPanel info={unread} nameOf={nameOf} onOpenSession={vi.fn()} />);
    expect(screen.queryByRole("meter")).toBeNull();
    expect(screen.getByTestId("this-mac-memory").textContent).toBe("Memory: 32 GB");
  });

  // AJ-E1526: a tool line carries its tool's mark beside the name, and a
  // line whose tool isn't known carries none.
  it("marks each tool line with its tool's mark, decoratively", () => {
    render(
      <ThisMacPanel
        info={info("plenty", [tree({ tool: "claude-code" }), tree({ place: "terminal", rss_bytes: 1_000 })])}
        nameOf={nameOf}
        onOpenSession={vi.fn()}
      />,
    );
    const [marked, unmarked] = screen.getAllByTestId("this-mac-tree");
    expect(marked!.querySelector("[data-mark]")?.getAttribute("aria-hidden")).toBe("true");
    expect(marked!.textContent).toContain("Claude app");
    expect(unmarked!.querySelector("[data-mark]")).toBeNull();
  });

  it("offers Clean up only when something was left running", () => {
    const clean = vi.fn();
    render(<ThisMacPanel info={info("plenty", [tree({})], [2, 1])} nameOf={nameOf} onOpenSession={vi.fn()} onOpenCleanup={clean} />);
    expect((screen.getByTestId("this-mac-leftovers"))!.textContent).toContain("2 processes left running after their session closed · holding a port");
    fireEvent.click(screen.getByRole("button", { name: "Clean up" }));
    expect(clean).toHaveBeenCalled();
  });

  it("mentions tools we didn't start when none is among the heaviest", () => {
    const ours = [1, 2, 3].map((n) => tree({ session_id: `s${n}`, place: null, rss_bytes: 3e9 }));
    const all = info("plenty", [...ours, tree({ rss_bytes: 1e8 })]);
    all.top = ours;
    render(<ThisMacPanel info={all} nameOf={nameOf} onOpenSession={vi.fn()} />);
    expect(screen.getByText("Also counted: 1 Claude tool not started by AI Juggler.")).toBeTruthy();
  });
});

describe("Home's Needs you (AJ-E1404 Req.3)", () => {
  const accounts = [{ id: "work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in" }] as AccountInfo[];
  const projects: ProjectInfo[] = [];

  it("lists a heavy session once, with one action, and takes it out of Running", () => {
    const open = vi.fn();
    const sessions = [mk("heavy-one"), mk("calm-one")];
    render(
      <HomeScreen
        sessions={sessions}
        accounts={accounts}
        projects={projects}
        now={NOW}
        onOpenSession={open}
        onNewSession={vi.fn()}
        onAddAccounts={vi.fn()}
        thisMac={info("plenty", [tree({ session_id: "heavy-one", place: null, heavy_note: "3.2 GB", rss_bytes: 3.2e9, helpers: 5 })])}
      />,
    );
    expect((screen.getByRole("heading", { level: 1 }))!.textContent).toContain("1 session needs you");
    const item = screen.getByTestId("home-heavy");
    expect((item)!.textContent).toContain("Working your Mac hard — 3.2 GB");
    expect((item)!.textContent).toContain("heavy-one · Work Max · 5 helpers");
    fireEvent.click(within(item).getByRole("button", { name: /Take a look/ }));
    expect(open).toHaveBeenCalledWith("heavy-one");
    const running = screen.getByTestId("home-running");
    expect((running)!.textContent).toContain("calm-one");
    expect((running)!.textContent).not.toContain("heavy-one");
  });

  it("raises a Mac under pressure and leads to This Mac", () => {
    const openPage = vi.fn();
    render(
      <HomeScreen
        sessions={[mk("one")]}
        accounts={accounts}
        projects={projects}
        now={NOW}
        onOpenSession={vi.fn()}
        onNewSession={vi.fn()}
        onAddAccounts={vi.fn()}
        thisMac={info("pressure", [tree({ session_id: "one", place: null })])}
        onOpenThisMac={openPage}
      />,
    );
    // Said once, in Needs you; the headline only points there (0.0.12).
    expect((screen.getByRole("heading", { level: 1 }))!.textContent).toBe("Your Mac needs a look");
    const item = screen.getByTestId("home-mac-pressure");
    expect(item.textContent).toContain("Your Mac is under pressure");
    expect(item.textContent).toContain("macOS reports critical memory pressure; swap is growing — 620 MB a minute");
    // AJ-E1528: it opens the This Mac page.
    fireEvent.click(within(item).getByRole("button", { name: "See what's using it" }));
    expect(openPage).toHaveBeenCalledOnce();
  });

  it("stays quiet when the Mac is only getting tight", () => {
    render(
      <HomeScreen
        sessions={[mk("one")]}
        accounts={accounts}
        projects={projects}
        now={NOW}
        onOpenSession={vi.fn()}
        onNewSession={vi.fn()}
        onAddAccounts={vi.fn()}
        thisMac={info("tight", [tree({ session_id: "one", place: null })])}
      />,
    );
    expect(screen.queryByTestId("home-needs")).toBeNull();
    expect(screen.getByTestId("home-this-mac").getAttribute("data-level")).toBe("tight");
  });
});

describe("the rail's heavy note (AJ-E1405)", () => {
  const row = (note: string | null) =>
    render(
      <ThisMacContext.Provider value={info("plenty", note ? [tree({ session_id: "s1", place: null, heavy_note: note })] : [])}>
        <RailRow session={mk("s1")} selected={false} onSelect={vi.fn()} density="comfortable" now={NOW} />
      </ThisMacContext.Provider>,
    );

  it("adds one figure only while the session is heavy", () => {
    row("CPU 96% · 3.2 GB");
    expect((screen.getByTestId("rail-heavy"))!.textContent).toContain("3.2 GB");
    expect(screen.getByRole("button").textContent).toContain("working your Mac hard: CPU 96% · 3.2 GB");
    cleanup();
    row(null);
    expect(screen.queryByTestId("rail-heavy")).toBeNull();
  });
});

describe("overload alerts and recommendations (AJ-E1408, AJ-E1409)", () => {
  const accounts = [{ id: "work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in" }] as AccountInfo[];
  const alert = (overrides: Partial<import("../lib/machine").OverloadAlertInfo>) => ({
    key: "memory_pressure",
    kind: "memory_pressure",
    title: "Your Mac is under memory pressure",
    detail: "macOS reports memory pressure",
    session_id: "busy-one",
    place: null,
    subject_bytes: 3_200_000_000,
    subject_estimate: false,
    subject_cpu: null,
    subject_windows: null,
    since_ms: NOW,
    ...overrides,
  });

  it("puts a sustained machine alert under Needs you, naming the biggest user", () => {
    const mac = info("tight", [tree({ session_id: "busy-one", place: null })]);
    mac.alerts = [alert({})];
    render(
      <HomeScreen
        sessions={[mk("busy-one")]}
        accounts={accounts}
        projects={[]}
        now={NOW}
        onOpenSession={vi.fn()}
        onNewSession={vi.fn()}
        onAddAccounts={vi.fn()}
        thisMac={mac}
      />,
    );
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Your Mac needs a look");
    const item = screen.getByTestId("home-mac-pressure");
    expect(item.textContent).toContain("Your Mac is under memory pressure");
    expect(screen.getAllByText("Your Mac is under memory pressure")).toHaveLength(1);
    expect(item.textContent).toContain("macOS reports memory pressure. busy-one is using 3.2 GB");
    // A tight Mac with a sustained alert is amber, not red.
    expect(item.textContent).toContain("▲");
  });

  it("says a tool we didn't start uses about its memory, since that sum is an estimate", async () => {
    const { alertSubject } = await import("../lib/machine");
    const theirs = alert({ session_id: null, place: "desktop", subject_bytes: 2_000_000_000, subject_estimate: true });
    expect(alertSubject(theirs, () => null)).toBe("The Claude app is using about 2.0 GB");
    // The app as a whole: its windows' total, and how many (0.0.12).
    expect(alertSubject({ ...theirs, subject_bytes: 16_100_000_000, subject_windows: 3 }, () => null)).toBe(
      "The Claude app is using about 16 GB across 3 windows",
    );
    expect(alertSubject(alert({}), (id) => id)).toBe("busy-one is using 3.2 GB");
  });

  it("tells a fresh alert once in the notification region, and dismisses it", async () => {
    const { NotificationRegion } = await import("./NotificationRegion");
    (window as unknown as Record<string, unknown>).__AIJ_ALERTS_FRESH__ = [
      alert({ key: "cpu", kind: "cpu", title: "The processor has been flat out for minutes", detail: "97% busy for over two minutes", session_id: null, place: "desktop", subject_bytes: null, subject_cpu: 140 }),
    ];
    render(<NotificationRegion />);
    const notice = await screen.findByTestId("machine-alert");
    expect(notice.textContent).toContain("The processor has been flat out for minutes — 97% busy for over two minutes. The Claude app is using CPU 140%.");
    fireEvent.click(within(notice).getByRole("button", { name: "dismiss" }));
    expect(screen.queryByTestId("machine-alert")).toBeNull();
    delete (window as unknown as Record<string, unknown>).__AIJ_ALERTS_FRESH__;
  });

  it("on Home the banner leaves the Mac's alert to Needs you; other screens keep it (0.0.12)", async () => {
    const { NotificationRegion } = await import("./NotificationRegion");
    (window as unknown as Record<string, unknown>).__AIJ_ALERTS_FRESH__ = [alert({})];
    const view = render(<NotificationRegion hideMachineAlerts />);
    // The fresh alert is taken (and kept) but not shown here.
    await waitFor(() => expect((window as unknown as Record<string, unknown>).__AIJ_ALERTS_FRESH__).toEqual([]));
    expect(screen.queryByTestId("machine-alert")).toBeNull();
    // The same notice shows once Home is left.
    view.rerender(<NotificationRegion />);
    expect((await screen.findByTestId("machine-alert")).textContent).toContain("Your Mac is under memory pressure");
    fireEvent.click(within(screen.getByTestId("machine-alert")).getByRole("button", { name: "dismiss" }));
  });

  it("tells a CPU-heavy session by its CPU, not its memory", async () => {
    const { alertSubject } = await import("../lib/machine");
    const heavy = alert({ key: "heavy:busy-one", kind: "heavy", title: "A session is working your Mac hard", detail: "CPU 96%", subject_bytes: null, subject_cpu: 96 });
    expect(alertSubject(heavy, (id) => id)).toBe("busy-one is using CPU 96%");
    expect(alertSubject({ ...heavy, detail: "CPU 96% · 3.2 GB", subject_bytes: 3_200_000_000 }, (id) => id)).toBe("busy-one is using CPU 96% and 3.2 GB");
  });

  it("keeps the newest notice per kind, and lets one go once its condition ends", () => {
    const notice = (alertKey: string, kind: string, toldAt: number): MachineNotice => ({ key: `${alertKey}-${toldAt}`, alertKey, kind, text: alertKey, toldAt });
    // Queued while hidden: two heavy sessions land together — the newest stays.
    const queued = [notice("heavy:b", "heavy", 200), notice("heavy:a", "heavy", 100), notice("cpu", "cpu", 100)];
    expect(keptNotices(queued, null, 300).map((n) => n.alertKey)).toEqual(["heavy:b", "cpu"]);
    // This Mac says only cpu is still on: after the settle time, heavy:b goes …
    expect(keptNotices(queued, new Set(["cpu"]), 20_000).map((n) => n.alertKey)).toEqual(["cpu"]);
    // … but not in its first 10 s (that read may predate the alert).
    expect(keptNotices(queued, new Set(["cpu"]), 5_000).map((n) => n.alertKey)).toEqual(["heavy:b", "cpu"]);
  });

  it("clears a told notice when This Mac no longer has its alert", async () => {
    const { NotificationRegion } = await import("./NotificationRegion");
    (window as unknown as Record<string, unknown>).__AIJ_ALERTS_FRESH__ = [alert({})];
    const pressured = info("tight", []);
    pressured.alerts = [alert({})];
    const { rerender } = render(
      <ThisMacContext.Provider value={pressured}>
        <NotificationRegion />
      </ThisMacContext.Provider>,
    );
    await screen.findByTestId("machine-alert");
    delete (window as unknown as Record<string, unknown>).__AIJ_ALERTS_FRESH__;
    // Still under pressure: it stays.
    rerender(
      <ThisMacContext.Provider value={{ ...pressured }}>
        <NotificationRegion />
      </ThisMacContext.Provider>,
    );
    expect(screen.queryByTestId("machine-alert")).not.toBeNull();
    // The pressure ended (and the notice is past its first 10 s): it goes.
    const later = Date.now() + 15_000;
    vi.spyOn(Date, "now").mockReturnValue(later);
    rerender(
      <ThisMacContext.Provider value={{ ...info("plenty", []), alerts: [] }}>
        <NotificationRegion />
      </ThisMacContext.Provider>,
    );
    expect(screen.queryByTestId("machine-alert")).toBeNull();
  });

  it("never drops a row: six Claude app windows show three, then 'and 3 more' opens the rest (P6)", () => {
    const windows = [7.4e9, 5.2e9, 3.5e9, 2.0e9, 1.1e9, 0.6e9].map((rss, i) =>
      tree({ account_label: `Window ${i + 1}`, rss_bytes: rss }),
    );
    render(<ThisMacPanel info={info("plenty", windows)} nameOf={() => "x"} onOpenSession={vi.fn()} />);
    const list = screen.getByTestId("this-mac-top");
    expect(within(list).getAllByRole("listitem")).toHaveLength(3);
    const more = screen.getByTestId("this-mac-more");
    expect(more.textContent).toBe("and 3 more");
    expect(more.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(more);
    expect(within(screen.getByTestId("this-mac-top")).getAllByRole("listitem")).toHaveLength(6);
    expect(screen.getByTestId("this-mac-more").textContent).toBe("Show fewer");
    // Three or fewer: no button at all.
    cleanup();
    render(<ThisMacPanel info={info("plenty", windows.slice(0, 3))} nameOf={() => "x"} onOpenSession={vi.fn()} />);
    expect(screen.queryByTestId("this-mac-more")).toBeNull();
  });

  it("two app windows that look alike are two rows, each keyed by its process (#126 review, L6)", () => {
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    const twins = [501, 502].map((pid) => tree({ root_pid: pid, account_label: "Personal", rss_bytes: 2e9 }));
    render(<ThisMacPanel info={info("plenty", twins)} nameOf={() => "x"} onOpenSession={vi.fn()} />);
    expect(within(screen.getByTestId("this-mac-top")).getAllByRole("listitem")).toHaveLength(2);
    expect(errors.mock.calls.flat().join(" ")).not.toContain("same key");
    errors.mockRestore();
  });

  it("shows advice from history only when there is some", () => {
    const mac = info("plenty", [tree({})]);
    const { rerender } = render(<ThisMacPanel info={mac} nameOf={() => "x"} onOpenSession={vi.fn()} />);
    expect(screen.queryByTestId("this-mac-recommendations")).toBeNull();
    mac.recommendations = [
      "This Mac comfortably runs about 5 sessions at once (about 2.0 GB each, from your last 12)",
      "“Deep work” (Work Max) is the heaviest launch setting — about 3.1 GB a session",
    ];
    rerender(<ThisMacPanel info={{ ...mac }} nameOf={() => "x"} onOpenSession={vi.fn()} />);
    const list = screen.getByTestId("this-mac-recommendations");
    expect(list.textContent).toContain("comfortably runs about 5 sessions at once");
    expect(list.textContent).toContain("is the heaviest launch setting");
  });
});
