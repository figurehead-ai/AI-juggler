/**
 * E14 vitest — Home's Needs you for a heavy session (AJ-E1404), the rail's heavy note and the
 * session header's resources (AJ-E1405). Calm until it matters: nothing
 * before the first sample, colour and Needs-you entries only when heavy
 * or under pressure, and a tool AI Juggler didn't start offers nothing to
 * click (D-08).
 */
import { cleanup, fireEvent, render, screen, within, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RailRow } from "./WorkflowRail";
import { HomeScreen } from "../screens/Home";
import { ThisMacScreen } from "../screens/ThisMacScreen";
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
  label: overrides.session_id || overrides.place === null ? null : "Claude app",
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
    own: { rss_bytes: 214_000_000, watchdog: true, opens_at_login: false },
    by_tool: [],
    top: trees.slice(0, 3),
    others: trees.slice(3),
    heavy: trees.filter((t) => t.heavy_note),
    other_apps: [],
    leftover_processes: leftovers[0],
    leftover_ports: leftovers[1],
    alerts: [],
    recommendations: [],
  };
}

afterEach(cleanup);

describe("Home's Needs you (AJ-E1404 Req.3)", () => {
  const accounts = [{ id: "work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in" }] as AccountInfo[];
  const projects: ProjectInfo[] = [];

  it("lists a heavy session once, with one action", () => {
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
    expect((screen.getByRole("heading", { level: 1 }))!.textContent).toBe("1 thing needs you");
    const item = screen.getByTestId("home-need-heavy");
    expect((item)!.textContent).toContain("Working your Mac hard — 3.2 GB");
    expect((item)!.textContent).toContain("heavy-one · Work Max · 5 more processes");
    fireEvent.click(within(item).getByRole("button", { name: /Take a look/ }));
    expect(open).toHaveBeenCalledWith("heavy-one");
    expect(screen.getAllByTestId("home-need-heavy")).toHaveLength(1);
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
    // Said once, in Needs you; the headline counts it (AJ-E2105).
    expect((screen.getByRole("heading", { level: 1 }))!.textContent).toBe("1 thing needs you");
    const item = screen.getByTestId("home-need-mac");
    expect(item.textContent).toContain("Your Mac is under pressure");
    expect(item.textContent).toContain("macOS reports critical memory pressure; swap is growing — 620 MB a minute");
    // AJ-E1528: it opens the This Mac page.
    fireEvent.click(within(item).getByRole("button", { name: /^See what's using it/ }));
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
    // AJ-E2105: This Mac's line is the top bar's; Home doesn't repeat it.
    expect(screen.queryByTestId("home-this-mac")).toBeNull();
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
    subject_name: null as string | null,
    subject_bytes: 3_200_000_000,
    subject_estimate: false,
    subject_cpu: null,
    subject_windows: null,
    other_apps: [] as import("../lib/machine-memory").OtherAppInfo[],
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
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("1 thing needs you");
    const item = screen.getByTestId("home-need-mac");
    expect(item.textContent).toContain("Your Mac is under memory pressure");
    expect(screen.getAllByText("Your Mac is under memory pressure")).toHaveLength(1);
    expect(item.textContent).toContain("macOS reports memory pressure. busy-one is using 3.0 GB");
    // A tight Mac with a sustained alert is amber, not red.
    expect(item.textContent).toContain("▲");
  });

  it("says a tool we didn't start uses about its memory, since that sum is an estimate", async () => {
    const { alertSubject } = await import("../lib/machine");
    const theirs = alert({ session_id: null, place: "claude-app", subject_name: "The Claude app", subject_bytes: 2_000_000_000, subject_estimate: true });
    expect(alertSubject(theirs, () => null)).toBe("The Claude app is using about 1.9 GB");
    // Every tool's app in its adapter's words (the Codex app in ChatGPT.app).
    expect(alertSubject({ ...theirs, place: "codex-app", subject_name: "ChatGPT (Codex)" }, () => null)).toBe("ChatGPT (Codex) is using about 1.9 GB");
    // The app as a whole: its windows' total, and how many (0.0.12).
    expect(alertSubject({ ...theirs, subject_bytes: 16_100_000_000, subject_windows: 3 }, () => null)).toBe(
      "The Claude app is using about 15 GB across 3 windows",
    );
    expect(alertSubject(alert({}), (id) => id)).toBe("busy-one is using 3.0 GB");
  });

  it("names the other apps, and no AI tool, when they hold most of the memory (0.1.4 review)", async () => {
    const { alertSubject, alertWords } = await import("../lib/machine");
    const others = alert({
      session_id: null,
      place: null,
      subject_bytes: null,
      other_apps: [
        { name: "next-server", rss_bytes: 6_400_000_000, processes: 1 },
        { name: "Docker", rss_bytes: 4_000_000_000, processes: 3 },
        { name: "fseventsd", rss_bytes: 1_500_000_000, processes: 1 },
      ],
    });
    expect(alertSubject(others, () => null)).toBe("Other apps are using most of the memory: next-server 6.0 GB, Docker about 3.7 GB, fseventsd 1.4 GB");
    const words = alertWords([others, { ...others, key: "swap", kind: "swap", detail: "swap is growing — 3004 MB a minute" }], () => null)!;
    expect(words.detail).toBe(
      "macOS reports memory pressure; swap is growing — 3004 MB a minute. Other apps are using most of the memory: next-server 6.0 GB, Docker about 3.7 GB, fseventsd 1.4 GB",
    );
    expect(words.detail).not.toContain("Claude");
  });

  it("tells a fresh alert once in the notification region, and dismisses it", async () => {
    const { NotificationRegion } = await import("./NotificationRegion");
    (window as unknown as Record<string, unknown>).__AIJ_ALERTS_FRESH__ = [
      alert({ key: "cpu", kind: "cpu", title: "The processor has been flat out for minutes", detail: "97% busy for over two minutes", session_id: null, place: "claude-app", subject_name: "The Claude app", subject_bytes: null, subject_cpu: 140 }),
    ];
    render(<NotificationRegion />);
    const notice = await screen.findByTestId("machine-alert");
    expect(notice.textContent).toContain("The processor has been flat out for minutes — 97% busy for over two minutes. The Claude app is using CPU 140%.");
    fireEvent.click(within(notice).getByRole("button", { name: /^Close/ }));
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
    fireEvent.click(within(screen.getByTestId("machine-alert")).getByRole("button", { name: /^Close/ }));
  });

  it("tells a CPU-heavy session by its CPU, not its memory", async () => {
    const { alertSubject } = await import("../lib/machine");
    const heavy = alert({ key: "heavy:busy-one", kind: "heavy", title: "A session is working your Mac hard", detail: "CPU 96%", subject_bytes: null, subject_cpu: 96 });
    expect(alertSubject(heavy, (id) => id)).toBe("busy-one is using CPU 96%");
    expect(alertSubject({ ...heavy, detail: "CPU 96% · 3.2 GB", subject_bytes: 3_200_000_000 }, (id) => id)).toBe("busy-one is using CPU 96% and 3.0 GB");
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

  it("shows advice from history only when there is some", () => {
    const mac = info("plenty", [tree({})]);
    // The This Mac page (Home's panel went with AJ-E2105).
    const page = (value: typeof mac) => (
      <ThisMacContext.Provider value={value}>
        <ThisMacScreen nameOf={() => "x"} onOpenSession={vi.fn()} />
      </ThisMacContext.Provider>
    );
    const { rerender } = render(page(mac));
    expect(screen.queryByTestId("this-mac-recommendations")).toBeNull();
    mac.recommendations = [
      "This Mac comfortably runs about 5 sessions at once (about 2.0 GB each, from your last 12)",
      "“Deep work” (Work Max) is the heaviest launch setting — about 3.1 GB a session",
    ];
    rerender(page({ ...mac }));
    const list = screen.getByTestId("this-mac-recommendations");
    expect(list.textContent).toContain("comfortably runs about 5 sessions at once");
    expect(list.textContent).toContain("is the heaviest launch setting");
  });
});
