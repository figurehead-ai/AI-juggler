import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, waitFor } from "@testing-library/react";
import {
  KillSheet,
  killDisplayName,
  type KillBridge,
  type KillExecuteData,
  type KillVerifyData,
} from "./KillSheet";
import type { CleanupRowInfo } from "./ProcessRow";

/**
 * AJ-E507: the verify-before-kill sheet — the three steps, the
 * copy-exact refusal with the button REMOVED from the DOM, the
 * cancellable grace countdown, the honest outcome report (already
 * gone = success; still-held = a fact), and the Windows wording.
 */

function verifyData(overrides: Partial<KillVerifyData> = {}): KillVerifyData {
  return {
    pid: 8123,
    session: "sess-kill",
    verified: true,
    refusal: null,
    exe_path: "/usr/local/bin/node",
    cmdline: "node /srv/web/node_modules/.bin/vite dev",
    cwd: "/srv/web",
    ports: [5173],
    pgid: 8100,
    escaped_group: false,
    members: [
      {
        pid: 8123,
        pgid: 8100,
        exe: "node",
        cmdline: "node vite dev",
        role_guess: "DevServer",
        unsaved_state_warning: null,
        from_ledger: true,
      },
      {
        pid: 8124,
        pgid: 8100,
        exe: "node",
        cmdline: "node mcp-server",
        role_guess: "McpServer",
        unsaved_state_warning: "Includes a McpServer that may hold unsaved state.",
        from_ledger: true,
      },
    ],
    tree_pids: [8100, 8123, 8124],
    platform: "TERM → grace → KILL",
    grace_secs: 5,
    provider: "claude-code",
    account_id: "Work Max",
    project_id: "web",
    ...overrides,
  };
}

function executeData(overrides: Partial<KillExecuteData> = {}): KillExecuteData {
  return {
    refusal: null,
    members: [{ pid: 8123, outcome: "exited on TERM" }],
    cancelled: false,
    all_succeeded: true,
    still_holding: [],
    reoffer_pid: null,
    ...overrides,
  };
}

function bridge(overrides: Partial<KillBridge> = {}): KillBridge {
  return {
    verify: vi.fn(() => Promise.resolve(verifyData())),
    execute: vi.fn(() => Promise.resolve(executeData())),
    cancel: vi.fn(() => Promise.resolve()),
    liveness: vi.fn(() => Promise.resolve([false])),
    ...overrides,
  };
}

function focusableRow(): HTMLElement {
  // The originating row element (focus returns here on close).
  const element = document.createElement("button");
  document.body.appendChild(element);
  return element;
}

describe("KillSheet (AJ-E507)", () => {
  it("step 1 verifies and shows the re-read-now facts, collateral, and unsaved-state warnings", async () => {
    const b = bridge();
    const view = render(
      <KillSheet session="sess-kill" pid={8123} goalPort={5173} bridge={b} onClose={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-ready-8123")).toBeTruthy());
    const text = view.getByTestId("kill-ready-8123").textContent ?? "";
    expect(text).toContain("node /srv/web/node_modules/.bin/vite dev");
    expect(text).toContain("/srv/web");
    expect(text).toContain(":5173");
    expect(text).toContain("free :5173");
    expect(view.getByTestId("kill-member-8124").textContent ?? "").toContain("McpServer");
    expect(view.getByTestId("kill-unsaved-8124").textContent).toBe(
      "Includes a McpServer that may hold unsaved state.",
    );
    // The ladder wording, verbatim platform string.
    expect(view.getByTestId("kill-platform-8123").textContent ?? "").toContain("TERM → grace → KILL");
    // Scope defaults to the narrowest.
    expect(
      (view.getByTestId("kill-scope-pid-8123") as HTMLInputElement).checked,
    ).toBe(true);
    // Consequence-named button (Req.1's exact example shape).
    expect(view.getByTestId("kill-confirm-8123").textContent).toBe(
      "Terminate vite (PID 8123) and free :5173",
    );
    view.unmount();
  });

  it("step 2/3: ⌘⏎ confirms with the chosen scope, goal port, and a fresh attempt id", async () => {
    const b = bridge();
    const view = render(
      <KillSheet session="sess-kill" pid={8123} goalPort={5173} bridge={b} onClose={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-confirm-8123")).toBeTruthy());
    fireEvent.click(view.getByTestId("kill-scope-pgid-8123"));
    fireEvent.keyDown(view.getByTestId("kill-sheet-8123"), { key: "Enter", metaKey: true });
    await waitFor(() => expect(view.getByTestId("kill-report-8123")).toBeTruthy());
    expect(b.execute).toHaveBeenCalledWith(
      "sess-kill",
      8123,
      "pgid",
      expect.stringMatching(/^kill-8123-/),
      5173,
    );
    view.unmount();
  });

  it("failed verification becomes the copy-exact explanation with the kill button REMOVED from the DOM", async () => {
    const b = bridge({
      verify: () =>
        Promise.resolve(
          verifyData({
            verified: false,
            refusal:
              "This PID now belongs to a different program — Juggler will not touch it.",
            members: [],
          }),
        ),
    });
    const view = render(
      <KillSheet session="sess-kill" pid={8123} bridge={b} onClose={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-refused-8123")).toBeTruthy());
    expect(view.getByTestId("kill-refused-8123").textContent ?? "").toContain(
      "This PID now belongs to a different program — Juggler will not touch it.",
    );
    // REMOVED, not disabled-with-tooltip (Req.2).
    expect(view.queryByTestId("kill-confirm-8123")).toBeNull();
    view.unmount();
  });

  it("the grace window is visible and cancellable, with per-process exit ticks", async () => {
    let release: (value: KillExecuteData) => void = () => {};
    const b = bridge({
      execute: () =>
        new Promise<KillExecuteData>((resolve) => {
          release = resolve;
        }),
      liveness: vi.fn((pids: number[]) => Promise.resolve(pids.map(() => true))),
    });
    const view = render(
      <KillSheet session="sess-kill" pid={8123} bridge={b} onClose={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-confirm-8123")).toBeTruthy());
    fireEvent.click(view.getByTestId("kill-confirm-8123"));
    await waitFor(() => expect(view.getByTestId("kill-running-8123")).toBeTruthy());
    // The countdown announces politely at 1 s intervals; ticks poll.
    expect(view.getByTestId("kill-countdown-8123").textContent ?? "").toContain("5s");
    expect(view.getByTestId("kill-tick-8123").textContent ?? "").toContain("running");
    fireEvent.click(view.getByRole("button", { name: /Cancel — stop the escalation/ }));
    expect(b.cancel).toHaveBeenCalledWith(expect.stringMatching(/^kill-8123-/));
    release(executeData({ cancelled: true, members: [{ pid: 8123, outcome: "TERM'd — outcome unknown" }] }));
    await waitFor(() => expect(view.getByTestId("kill-report-8123")).toBeTruthy());
    expect(view.getByTestId("kill-report-8123").textContent ?? "").toContain(
      "Cancelled during grace",
    );
    view.unmount();
  });

  it("the outcome report: per-process vocabulary, already gone is success, ports honestly still held, re-offer", async () => {
    const b = bridge({
      execute: () =>
        Promise.resolve(
          executeData({
            members: [
              { pid: 8100, outcome: "already gone" },
              { pid: 8123, outcome: "needed KILL" },
            ],
            all_succeeded: true,
            still_holding: [{ port: 5173, pid: 9100, exe: "node" }],
            reoffer_pid: 9100,
          }),
        ),
    });
    const view = render(
      <KillSheet session="sess-kill" pid={8123} goalPort={5173} bridge={b} onClose={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-confirm-8123")).toBeTruthy());
    fireEvent.click(view.getByTestId("kill-confirm-8123"));
    await waitFor(() => expect(view.getByTestId("kill-report-8123")).toBeTruthy());
    const report = view.getByTestId("kill-report-8123").textContent ?? "";
    expect(report).toContain("PID 8100 — already gone");
    expect(report).toContain("PID 8123 — needed KILL");
    expect(report).toContain("Still holding");
    expect(report).toContain(":5173 by node (PID 9100)");
    // Success is NOT claimed for a still-held port.
    expect(view.queryByTestId("kill-success-8123")).toBeNull();
    expect(view.getByTestId("kill-reoffer-8123").textContent ?? "").toContain(
      "Re-verify PID 9100",
    );
    view.unmount();
  });

  it("already-gone-only outcomes report success for the goal port", async () => {
    const b = bridge({
      execute: () =>
        Promise.resolve(
          executeData({ members: [{ pid: 8123, outcome: "already gone" }] }),
        ),
    });
    const view = render(
      <KillSheet session="sess-kill" pid={8123} goalPort={5173} bridge={b} onClose={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-confirm-8123")).toBeTruthy());
    fireEvent.click(view.getByTestId("kill-confirm-8123"));
    await waitFor(() => expect(view.getByTestId("kill-report-8123")).toBeTruthy());
    expect(view.getByTestId("kill-success-8123").textContent ?? "").toContain(":5173 is free.");
    view.unmount();
  });

  it("the Windows sheet says TerminateProcess only, verbatim", async () => {
    const b = bridge({
      verify: () =>
        Promise.resolve(verifyData({ platform: "TerminateProcess only", grace_secs: 0 })),
    });
    const view = render(
      <KillSheet session="sess-kill" pid={8123} bridge={b} onClose={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-ready-8123")).toBeTruthy());
    expect(view.getByTestId("kill-platform-8123").textContent ?? "").toContain(
      "Windows: TerminateProcess only — there is no grace window here.",
    );
    view.unmount();
  });

  it("Esc closes at any step and focus returns to the originating row (2.4.3)", async () => {
    const b = bridge();
    const trigger = focusableRow();
    const close = vi.fn();
    const view = render(
      <KillSheet session="sess-kill" pid={8123} bridge={b} trigger={trigger} onClose={close} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-ready-8123")).toBeTruthy());
    fireEvent.keyDown(view.getByTestId("kill-sheet-8123"), { key: "Escape" });
    expect(close).toHaveBeenCalledWith(false);
    expect(document.activeElement).toBe(trigger);
    view.unmount();
    trigger.remove();
  });

  it("derives the display name for the consequence-named button", () => {
    expect(killDisplayName("node /srv/node_modules/.bin/vite dev", "/x/node")).toBe("vite");
    expect(killDisplayName("", "/usr/bin/python3")).toBe("python3");
    expect(killDisplayName("C:\\tools\\srv.exe serve", "")).toBe("srv.exe");
  });
});

// The row DTO type rides along for the affordance test (same file
// keeps the fixture shape honest).
export type { CleanupRowInfo };
