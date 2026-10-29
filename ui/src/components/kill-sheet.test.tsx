import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, waitFor } from "@testing-library/react";
import {
  KillSheet,
  killDisplayName,
  type KillBridge,
  type KillExecuteData,
  type KillVerifyData,
} from "./KillSheet";

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
        role_guess: "dev server",
        unsaved_state_warning: null,
        from_ledger: true,
      },
      {
        pid: 8124,
        pgid: 8100,
        exe: "node",
        cmdline: "node mcp-server",
        role_guess: "MCP server",
        unsaved_state_warning: "Includes an MCP server that may hold unsaved state.",
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
    // One plain proof line; the facts behind "Details".
    expect(view.getByTestId("kill-proof-8123").textContent).toBe(
      "Checked: the process AI Juggler started (same start time and program) · stopping frees :5173",
    );
    expect(view.getByTestId("kill-details-8123").textContent ?? "").toContain("folder");
    expect(view.getByTestId("kill-member-8124").textContent ?? "").toContain("probably an MCP server");
    expect(view.getByTestId("kill-unsaved-8124").textContent).toBe(
      "Includes an MCP server that may hold unsaved state.",
    );
    // The ladder wording, verbatim platform string.
    // The ladder in plain words (D-25: no TERM/KILL on screen).
    expect(view.getByTestId("kill-platform-8123").textContent ?? "").toContain("asked to close, and AI Juggler waits");
    expect(view.getByTestId("kill-platform-8123").textContent ?? "").not.toMatch(/SIGTERM|KILL|Ladder/);
    // Scope defaults to the narrowest.
    expect(
      (view.getByTestId("kill-scope-pid-8123") as HTMLInputElement).checked,
    ).toBe(true);
    // Consequence-named button (Req.1's exact example shape).
    expect(view.getByTestId("kill-confirm-8123").textContent).toBe(
      "Stop vite (PID 8123) and free :5173",
    );
    expect(view.getByRole("dialog").getAttribute("aria-label")).toBe("Stop vite?");
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
    expect(view.getByTestId("kill-countdown-8123").textContent ?? "").toContain("5 s");
    expect(view.getByTestId("kill-tick-8123").textContent ?? "").toContain("running");
    fireEvent.click(view.getByRole("button", { name: /Cancel: don't force it/ }));
    expect(b.cancel).toHaveBeenCalledWith(expect.stringMatching(/^kill-8123-/));
    release(executeData({ cancelled: true, members: [{ pid: 8123, outcome: "TERM'd — outcome unknown" }] }));
    await waitFor(() => expect(view.getByTestId("kill-report-8123")).toBeTruthy());
    expect(view.getByTestId("kill-report-8123").textContent ?? "").toContain(
      "Cancelled: it was asked to stop",
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
    expect(report).toContain("PID 8100 — Had already ended");
    expect(report).toContain("PID 8123 — Needed a force stop");
    expect(report).toContain("Still in use");
    expect(report).toContain(":5173 by node (PID 9100)");
    // Success is NOT claimed for a still-held port.
    expect(view.queryByTestId("kill-success-8123")).toBeNull();
    expect(view.getByTestId("kill-reoffer-8123").textContent ?? "").toContain(
      "Check PID 9100",
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
      "a stop is immediate: there's no waiting time to cancel in.",
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

  it("an unsaved-state warning shows for the process itself, alone in its group", async () => {
    const b = bridge({
      verify: () =>
        Promise.resolve(
          verifyData({
            members: [
              {
                pid: 8123,
                pgid: 8123,
                exe: "postgres",
                cmdline: "postgres -D data",
                role_guess: "database",
                unsaved_state_warning: "Includes a database that may hold unsaved state.",
                from_ledger: true,
              },
            ],
          }),
        ),
    });
    const view = render(
      <KillSheet session="sess-kill" pid={8123} bridge={b} observed sessionEnded onClose={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("kill-ready-8123")).toBeTruthy());
    expect(view.getByTestId("kill-unsaved-8123").textContent).toBe(
      "Includes a database that may hold unsaved state.",
    );
    // No group list for a process alone in its group.
    expect(view.queryByTestId("kill-members-8123")).toBeNull();
    expect(view.getByTestId("kill-proof-8123").textContent).toBe(
      "Checked: the process AI Juggler saw in this session (same start time, program and boot) · its session has ended · stopping frees :5173",
    );
    view.unmount();
  });

  it("a group member AI Juggler didn't start is named as left running and never ticked", async () => {
    const b = bridge({
      verify: () =>
        Promise.resolve(
          verifyData({
            members: [
              { pid: 8123, pgid: 8100, exe: "node", cmdline: "node vite dev", role_guess: "dev server", unsaved_state_warning: null, from_ledger: true },
              { pid: 8125, pgid: 8100, exe: "zsh", cmdline: "zsh", role_guess: "terminal shell", unsaved_state_warning: "Includes a terminal shell that may hold unsaved state.", from_ledger: false },
            ],
          }),
        ),
      execute: () => new Promise<KillExecuteData>(() => {}),
      liveness: vi.fn((pids: number[]) => Promise.resolve(pids.map(() => true))),
    });
    const view = render(<KillSheet session="sess-kill" pid={8123} bridge={b} onClose={() => {}} />);
    await waitFor(() => expect(view.getByTestId("kill-member-8125")).toBeTruthy());
    expect(view.getByTestId("kill-member-8125").textContent ?? "").toContain("not started by AI Juggler: left running");
    // Its warning is moot: it won't be stopped.
    expect(view.queryByTestId("kill-unsaved-8125")).toBeNull();
    fireEvent.click(view.getByTestId("kill-scope-pgid-8123"));
    fireEvent.click(view.getByTestId("kill-confirm-8123"));
    await waitFor(() => expect(view.getByTestId("kill-tick-8123")).toBeTruthy());
    expect(view.queryByTestId("kill-tick-8125")).toBeNull();
    view.unmount();
  });

  it("the report says what a wider stop left alone and links to Cleanup history", async () => {
    const showHistory = vi.fn();
    const b = bridge({
      execute: () =>
        Promise.resolve(
          executeData({
            members: [
              { pid: 8123, outcome: "exited on TERM" },
              { pid: 8130, outcome: "already gone" },
              { pid: 8131, outcome: "left alone: not started by AI Juggler" },
            ],
          }),
        ),
    });
    const view = render(<KillSheet session="sess-kill" pid={8123} bridge={b} onShowHistory={showHistory} onClose={() => {}} />);
    await waitFor(() => expect(view.getByTestId("kill-confirm-8123")).toBeTruthy());
    fireEvent.click(view.getByTestId("kill-scope-tree-8123"));
    fireEvent.click(view.getByTestId("kill-confirm-8123"));
    await waitFor(() => expect(view.getByTestId("kill-report-8123")).toBeTruthy());
    expect(view.getByRole("dialog").getAttribute("aria-label")).toBe("Stopped");
    expect(view.getByTestId("kill-outcome-8123").textContent).toBe("PID 8123 — Stopped when asked");
    expect(view.getByTestId("kill-outcome-8131").textContent).toBe("PID 8131 — Left running: AI Juggler didn't start it");
    expect(view.getByTestId("kill-left-alone").textContent).toBe(
      "1 process had already ended. 1 process left running: not confirmed as one AI Juggler started.",
    );
    // No developer words in the report.
    expect(view.getByTestId("kill-report-8123").textContent ?? "").not.toMatch(/SIGKILL|TERM|KILL|Terminat/);
    fireEvent.click(view.getByTestId("kill-history-link-8123"));
    expect(showHistory).toHaveBeenCalledTimes(1);
    view.unmount();
  });

  it("a refused stop saves nothing, so it names no history", async () => {
    const b = bridge({
      execute: () => Promise.resolve(executeData({ refusal: "This process has already ended — there's nothing to stop.", members: [{ pid: 8123, outcome: "refused" }], all_succeeded: false })),
    });
    const view = render(<KillSheet session="sess-kill" pid={8123} bridge={b} onShowHistory={() => {}} onClose={() => {}} />);
    await waitFor(() => expect(view.getByTestId("kill-confirm-8123")).toBeTruthy());
    fireEvent.click(view.getByTestId("kill-confirm-8123"));
    await waitFor(() => expect(view.getByTestId("kill-report-8123")).toBeTruthy());
    expect(view.getByRole("dialog").getAttribute("aria-label")).toBe("Not stopped");
    expect(view.queryByTestId("kill-history-8123")).toBeNull();
    view.unmount();
  });

  it("derives the display name for the consequence-named button", () => {
    expect(killDisplayName("node /srv/node_modules/.bin/vite dev", "/x/node")).toBe("vite");
    expect(killDisplayName("", "/usr/bin/python3")).toBe("python3");
    expect(killDisplayName("C:\\tools\\srv.exe serve", "")).toBe("srv.exe");
  });
});

// The row DTO type rides along for the affordance test (same file
// keeps the fixture shape honest).
