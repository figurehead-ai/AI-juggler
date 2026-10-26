/**
 * QA (P19, on main): the Mac's state read four ways at once:
 * - the alert said "under memory pressure";
 * - Needs you blamed one Claude app window;
 * - This Mac said "Getting tight";
 * - the rail footer said "this-mac · healthy", and the collapsed rail
 *   "All clear".
 *
 * One fixture per state, every surface that shows the Mac's state rendered
 * from it: they must agree. The surfaces are the alert banner, Needs you
 * with its headline, This Mac's line (the top bar's, AJ-E2105: Home no
 * longer repeats This Mac), the collapsed rail's dot and its "All clear".
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MachineStatusLine } from "./components/MachineStatusLine";
import { RailCalm } from "./components/RailCalm";
import { HomeScreen } from "./screens/Home";
import { ThisMacContext, mockMachineHealth, mockThisMac, type OverloadAlertInfo, type ThisMacInfo, type ToolTreeInfo } from "./lib/machine";
import type { AccountInfo } from "./lib/bridge";

const NOW = 1_728_800_000_000;
const ACCOUNTS = [{ id: "work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in" }] as AccountInfo[];

// Three Claude app windows AI Juggler didn't start: the alert names them
// as one tool, with their total.
const appWindow = (rss: number, pid: number): ToolTreeInfo => ({
  root_pid: pid,
  session_id: null,
  place: "desktop",
  tool: "claude-code",
  group: "desktop",
  account_label: "Personal",
  cpu_pct: 20,
  rss_bytes: rss,
  helpers: 30,
  heavy_note: null,
  memory_estimate: true,
});
const OBSERVED = [appWindow(7_400_000_000, 501), appWindow(5_200_000_000, 502), appWindow(3_500_000_000, 503)];

const PRESSURE_ALERT: OverloadAlertInfo = {
  key: "memory_pressure",
  kind: "memory_pressure",
  title: "Your Mac is under memory pressure",
  detail: "macOS reports memory pressure",
  session_id: null,
  place: "desktop",
  subject_bytes: 16_100_000_000,
  subject_estimate: true,
  subject_cpu: null,
  subject_windows: 3,
  other_apps: [],
  since_ms: NOW - 60_000,
};

function state(level: "plenty" | "tight" | "pressure"): ThisMacInfo {
  return mockThisMac([], mockMachineHealth(level, NOW), OBSERVED, [0, 0], level === "pressure" ? [PRESSURE_ALERT] : []);
}

function renderAll(info: ThisMacInfo) {
  return render(
    <ThisMacContext.Provider value={info}>
      <HomeScreen
        sessions={[]}
        accounts={ACCOUNTS}
        projects={[]}
        now={NOW}
        onOpenSession={vi.fn()}
        onNewSession={vi.fn()}
        onAddAccounts={vi.fn()}
        thisMac={info}
      />
      <MachineStatusLine minimal={false} />
      <MachineStatusLine minimal />
      <RailCalm />
    </ThisMacContext.Provider>,
  );
}

afterEach(cleanup);

describe("every surface says the same state", () => {
  it.each(["plenty", "tight", "pressure"] as const)("%s", (level) => {
    const info = state(level);
    const words = info.machine.summary.words;
    renderAll(info);

    // This Mac's line, and the collapsed rail's dot.
    expect(screen.getByTestId("machine-status").textContent).toBe(`This Mac · ${words}`);
    expect(screen.getByTestId("machine-status-dot").getAttribute("title")).toBe(`This Mac · ${words}`);
    expect(screen.getByTestId("machine-status").getAttribute("data-level")).toBe(info.machine.summary.level);
    // Memory pressure is explained where macOS reports it, and only there.
    expect(screen.getByTestId("machine-status").getAttribute("title")?.startsWith("Memory pressure is macOS's own measure") ?? false).toBe(info.machine.pressure !== "normal");
    // "All clear" only while the Mac is calm.
    expect(screen.queryByTestId("rail-all-clear") !== null).toBe(info.machine.summary.level === "plenty");
    // Nobody says "healthy" while it isn't.
    expect(document.body.textContent).not.toContain("healthy");

    const need = document.querySelector<HTMLElement>('[data-kind="mac"]');
    if (level === "pressure") {
      // The alert and This Mac: one severity, one figure (the group's).
      expect(words).toBe("Under memory pressure");
      expect(need!.textContent).toContain("Your Mac is under memory pressure");
      expect(need!.textContent).toContain("The Claude app is using about 15 GB across 3 windows");
      expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("1 thing needs you");
    } else {
      expect(need).toBeNull();
    }
  });

  it("before the first sample: no state is claimed anywhere", () => {
    const info = { ...state("plenty"), machine: mockMachineHealth("unsampled", NOW) };
    renderAll(info);
    expect(document.querySelector('[data-kind="mac"]')).toBeNull();
    expect(screen.getByTestId("machine-status").textContent).toBe("This Mac");
    expect(screen.getByTestId("machine-status").getAttribute("data-level")).toBe("unsampled");
    expect(document.body.textContent).not.toContain("healthy");
  });
});
