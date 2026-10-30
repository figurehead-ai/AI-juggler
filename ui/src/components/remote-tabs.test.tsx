/**
 * AJ-E705 vitest — remote tab bodies: every value names its machine and
 * when it was measured; a field the machine doesn't send reads
 * "Unavailable", never an estimate; View-only pairing disables Stop
 * with the reason; Stop shows the node's own verdict; freshness is
 * words + a shape, never colour alone.
 */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RemoteTabBody, freshnessText } from "./RemoteTabs";
import { resetToasts, ToastHost } from "./Toast";
import { STOP_HOLD_SECONDS } from "../lib/stop";
import type { RemoteMachineInfo } from "../lib/bridge";

const NOW = 1_800_000_000_000;
const w = () => window as unknown as Record<string, unknown>;

const machine = (over: Partial<RemoteMachineInfo> = {}): RemoteMachineInfo => ({
  machine_id: "machine-mini",
  label: "Mac mini",
  connection: "connected",
  freshness: "live",
  last_heard_ms: NOW - 1_000,
  permission: "control",
  resources: { cpu_percent: 23, memory_used_bytes: 8 * 1024 ** 3, memory_total_bytes: 32 * 1024 ** 3, measured_at_ms: NOW - 1_000 },
  instances: [
    {
      id: "inst-1",
      provider: "claude-code",
      account_label: "Work Max",
      status: "running",
      model: null,
      model_source: "unknown",
      project_name: null,
      started_at_ms: NOW - 3_600_000,
      last_activity_ms: null,
      cpu_percent: null,
      rss_bytes: null,
      process_count: null,
      kind: "desktop",
      measured_at_ms: NOW - 2_000,
    },
  ],
  alerts: [],
  processes: [],
  clock_skew_ms: 0,
  synced_at_ms: NOW - 1_000,
  ...over,
});

afterEach(() => {
  cleanup();
  resetToasts();
  vi.useRealTimers();
});
beforeEach(() => {
  delete w().__AIJ_REMOTE_STOP__;
  delete w().__AIJ_REMOTE_FOCUS__;
  delete w().__AIJ_FLEET_CALLS__;
});

const body = (tab: "overview" | "usage" | "history" | "quality" | "processes" | "actions", m = machine(), instance = true) =>
  render(
    <RemoteTabBody
      tab={tab}
      remote={{ machine: m, instance: instance ? m.instances[0]! : null }}
      now={NOW}
      onSelectInstance={() => {}}
      onActionDone={() => {}}
    />,
  );

describe("RemoteTabBody", () => {
  it("labels every value with its source and never estimates a missing one", () => {
    const view = body("overview");
    expect(view.container.textContent).toContain("from Mac mini · measured 2 s ago");
    expect(view.getByTestId("remote-model").textContent).toContain("Unavailable — not reported");
    expect(view.container.textContent).not.toMatch(/\$0|estimate/i);
  });

  it("tabs a machine doesn't share say so plainly", () => {
    for (const tab of ["usage", "history"] as const) {
      const view = body(tab);
      expect(view.getByTestId("remote-unavailable").textContent).toContain("Unavailable — Mac mini doesn't share");
      cleanup();
    }
  });

  it("View only disables Stop and names why", () => {
    const view = body("actions", machine({ permission: "view" }));
    expect((view.getByTestId("remote-stop") as HTMLButtonElement).disabled).toBe(true);
    expect(view.getByTestId("remote-stop-reason").textContent).toBe(
      "View-only pairing — change in Fleet settings on Mac mini",
    );
  });

  it("an offline machine can't be asked to act", () => {
    const view = body("actions", machine({ connection: "offline", freshness: "offline" }));
    expect((view.getByTestId("remote-stop") as HTMLButtonElement).disabled).toBe(true);
    expect(view.getByTestId("remote-stop-reason").textContent).toContain("isn't connected");
  });

  it("Focus on <machine> shows that machine's own outcome", async () => {
    w().__AIJ_REMOTE_FOCUS__ = { accepted: false, reason_code: "screen_locked", message: "Mac mini is locked — nothing was focused." };
    const view = body("actions");
    await act(async () => {
      fireEvent.click(view.getByTestId("remote-focus"));
    });
    expect(view.getByTestId("remote-outcome").textContent).toContain("Mac mini refused: Mac mini is locked — nothing was focused.");
    expect(w().__AIJ_FLEET_CALLS__).toContain("remote-focus:machine-mini:inst-1");
  });

  it("Stop is held 5 s with Undo, like a local Stop, then shows the node's own verdict", async () => {
    vi.useFakeTimers();
    w().__AIJ_REMOTE_STOP__ = { accepted: false, reason_code: "unknown_record", message: "No such session on this computer." };
    render(<ToastHost />);
    const view = body("actions");
    act(() => {
      fireEvent.click(view.getByTestId("remote-stop"));
    });
    expect(screen.getByTestId("stop-undo-toast").textContent).toContain("Stopping Work Max on Mac mini.");
    await act(async () => {
      vi.advanceTimersByTime(STOP_HOLD_SECONDS * 1000 - 1);
    });
    expect(w().__AIJ_FLEET_CALLS__).toBeUndefined();
    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(w().__AIJ_FLEET_CALLS__).toContain("remote-stop:machine-mini:inst-1");
    expect(view.getByTestId("remote-outcome").textContent).toContain("Mac mini refused: No such session on this computer.");
  });

  it("Undo in time and the remote Stop is never sent", async () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    const view = body("actions");
    act(() => {
      fireEvent.click(view.getByTestId("remote-stop"));
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("stop-undo"));
    });
    await act(async () => {
      vi.advanceTimersByTime(60_000);
    });
    expect(w().__AIJ_FLEET_CALLS__).toBeUndefined();
  });

  it("the processes tab points to where they are cleaned, and opens it", () => {
    const m = machine({ processes: [
        {
          instance_id: "inst-1",
          provider: "claude-code",
          account_label: "Work Max",
          pid: 4242,
          role: "dev server",
          state: "orphaned",
          state_label: "Left running",
          verified: true,
          name: "vite",
          age_ms: null,
          rss_bytes: null,
          cpu_percent: null,
          ports: "5173",
          measured_at_ms: NOW - 1_000,
        },
      ] });
    const view = body("processes", m);
    const link = view.getByTestId("remote-clean-link");
    expect(link.parentElement!.textContent).toBe("Clean verified processes in Processes › Your other Macs.");
    const opened = vi.fn();
    const listener = (event: Event) => opened((event as CustomEvent).detail);
    window.addEventListener("aij:open-screen", listener);
    fireEvent.click(link);
    window.removeEventListener("aij:open-screen", listener);
    expect(opened).toHaveBeenCalledWith({ id: "cleanup" });
  });

  it("the machine overview lists its Instances and resources", () => {
    const view = body("overview", machine(), false);
    expect(view.getByTestId("remote-machine-overview").textContent).toContain("View + control");
    expect(view.getAllByTestId("remote-machine-instance")).toHaveLength(1);
    expect(view.container.textContent).toContain("8.0 GB of 32 GB");
  });

  it("freshness is words and a shape for every state", () => {
    expect(freshnessText(machine(), NOW)).toBe("● Live — from Mac mini, updated 1 s ago");
    expect(freshnessText(machine({ freshness: "stale", last_heard_ms: NOW - 40_000 }), NOW)).toBe(
      "◐ Stale — last update from Mac mini 40 s ago",
    );
    expect(freshnessText(machine({ freshness: "offline", last_heard_ms: null }), NOW)).toBe(
      "○ Offline — Mac mini hasn't reported yet",
    );
  });
});
