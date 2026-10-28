import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow, showSwitchUndoToast } from "./SwitchFlow";
import { resetToasts, ToastHost } from "./Toast";
import type { PreflightInfoData } from "../lib/bridge";

/**
 * AJ-E206 guardrails (pass2/02 §8, deep/09 H-5):
 *  - ZERO executions without a click: the door renders, nothing
 *    moves until the user presses Switch (and the wizard's Switch
 *    button exists only on the last step — never mid-preview).
 *  - Undo fires ONLY from the undo click (the toast's button), and
 *    the 10 s window closes itself.
 *  - The 1-click path hands the edge id to the undo window; the
 *    wizard path does not (its done modal navigates with undo=null).
 */

const ONECLICK: PreflightInfoData = {
  mechanism: "transplant",
  fidelity: "F4",
  fidelity_words: "Continues exactly",
  score: 1.0,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: true,
  source_keeps_running: false,
  experimental_copy:
    "Beta, not proven yet: the whole session moves, with its subagents and file history. The source stays resumable.",
  door: "one-click",
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: {
    carried: ["The whole conversation", "Project rules"],
    regenerated: [],
    dropped: [],
  },
  redaction: {
    global: "strip",
    override: null,
    effective: "strip",
    source: "global",
    line: "Keys and passwords for Claude Code: hidden (your global choice)",
  },
  redaction_items: [],
  carry_items: [
    { label: "The whole conversation", hint: "Every message from the start.", how: "moves" },
    { label: "Project rules", hint: "The rule files in the project folder.", how: "moves" },
  ],
  move_line: "The whole conversation moves — all 42 messages, its subagents and file history. The original stays as it was.",
  secrets: { asks: false, reader: "Claude Code (Anthropic)" },
  target_tool: "claude-code",
};

const WIZARD: PreflightInfoData = {
  ...ONECLICK,
  fidelity: "F2",
  fidelity_words: "Same conversation, new account",
  door: "wizard",
  wizard_steps: ["Target", "What's carried", "Confirm"],
  wizard_reasons: ["This move hasn't been proven on a real setup yet."],
  usual_account: false,
  carry: {
    carried: ["The whole conversation"],
    regenerated: [],
    dropped: ["The task list"],
  },
  carry_items: [
    { label: "The whole conversation", hint: "Every message from the start.", how: "moves" },
    { label: "The task list", hint: "It doesn't go along: the new session starts without it.", how: "stays" },
  ],
};

type Seams = {
  __AIJ_SWITCH_PREFLIGHT__?: PreflightInfoData;
  __AIJ_SWITCH_EXECUTE_HANG__?: boolean;
  __AIJ_SWITCH_UNDO_CALLS__?: string[];
  __AIJ_SWITCH_UNDO_FAIL__?: string;
};

beforeEach(() => {
  delete (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__;
  delete (window as never as Seams).__AIJ_SWITCH_EXECUTE_HANG__;
  delete (window as never as Seams).__AIJ_SWITCH_UNDO_CALLS__;
  delete (window as never as Seams).__AIJ_SWITCH_UNDO_FAIL__;
});
afterEach(() => {
  cleanup();
  resetToasts();
});

function mount(
  preflight: PreflightInfoData,
  onSwitched: (sessionId: string, undo: { edgeId: string } | null) => void,
) {
  (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__ = preflight;
  return render(
    <SwitchFlow
      sessionId="sess-1"
      targetProfileId="p-1"
      targetLabel="Personal"
      onClose={() => {}}
      onSwitched={onSwitched}
    />,
  );
}

describe("AJ-E206 door", () => {
  it("counts the door it shows once, from the preflight's own answer", async () => {
    (window as never as { __AIJ_DOOR_COUNTS__?: unknown[] }).__AIJ_DOOR_COUNTS__ = [];
    const view = mount(WIZARD, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-continue"));
    await waitFor(() => expect(view.getByTestId("switch-steps").textContent).toContain("Step 2 of 3"));
    // Moving through the steps counts nothing more.
    expect((window as never as { __AIJ_DOOR_COUNTS__: unknown[] }).__AIJ_DOOR_COUNTS__).toEqual([
      { door: "wizard", steps: ["Target", "What's carried", "Confirm"] },
    ]);
  });

  it("executes nothing until the user clicks (1-click popover)", async () => {
    (window as never as Seams).__AIJ_SWITCH_EXECUTE_HANG__ = true;
    const onSwitched = vi.fn();
    const view = mount(ONECLICK, onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    // The guardrail: the popover is up, the engine has NOT been called.
    expect(view.queryByTestId("switch-executing")).toBeNull();
    expect(onSwitched).not.toHaveBeenCalled();
    // One click — the only path to execution.
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-executing")).toBeTruthy());
    expect(onSwitched).not.toHaveBeenCalled();
  });

  it("1-click hands the edge id to the undo window on success", async () => {
    const onSwitched = vi.fn();
    const view = mount(ONECLICK, onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(onSwitched).toHaveBeenCalledTimes(1));
    const [sessionId, undo] = onSwitched.mock.calls[0];
    expect(sessionId).toMatch(/^sess-preview-/);
    expect(undo?.edgeId).toMatch(/^edge-preview-/);
  });

  it("wizard shows only rendered steps; the Switch button lives on the last step only", async () => {
    const onSwitched = vi.fn();
    const view = mount(WIZARD, onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    expect(view.getByTestId("switch-steps").textContent).toContain("Step 1 of 3: Target");
    // No execution control before the preview has been seen (H-5).
    expect(view.queryByTestId("switch-confirm")).toBeNull();
    fireEvent.click(view.getByTestId("switch-continue"));
    await waitFor(() =>
      expect(view.getByTestId("switch-steps").textContent).toContain("Step 2 of 3"),
    );
    expect(view.getByTestId("switch-carry-dropped").textContent).toContain("The task list");
    expect(view.queryByTestId("switch-confirm")).toBeNull();
    fireEvent.click(view.getByTestId("switch-continue"));
    await waitFor(() => expect(view.getByTestId("switch-step-confirm")).toBeTruthy());
    expect(view.getByTestId("switch-confirm")).toBeTruthy();
  });

  it("wizard done navigates without an undo window (modal keeps the graph path)", async () => {
    const onSwitched = vi.fn();
    const view = mount(WIZARD, onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-done")).toBeTruthy());
    expect(onSwitched).not.toHaveBeenCalled();
    fireEvent.click(view.getByRole("button", { name: /Go to the new session/ }));
    expect(onSwitched).toHaveBeenCalledTimes(1);
    expect(onSwitched.mock.calls[0][1]).toBeNull();
  });
});

describe("AJ-E206 undo window", () => {
  it("undoes ONLY from the click (never on render), then hands back the resumed session", async () => {
    const onUndone = vi.fn();
    const onDismiss = vi.fn();
    const view = render(<ToastHost />);
    act(() => void showSwitchUndoToast({ edgeId: "edge-42", targetLabel: "Personal", onClose: onDismiss, onUndone }));
    await waitFor(() => expect(view.getByTestId("switch-undo-toast")).toBeTruthy());
    // Rendered ≠ undone: the seam records calls and has none.
    expect((window as never as Seams).__AIJ_SWITCH_UNDO_CALLS__).toBeUndefined();
    expect(onUndone).not.toHaveBeenCalled();
    fireEvent.click(view.getByTestId("switch-undo"));
    await waitFor(() =>
      expect((window as never as Seams).__AIJ_SWITCH_UNDO_CALLS__).toEqual(["edge-42"]),
    );
    await waitFor(() => expect(onUndone).toHaveBeenCalledTimes(1));
    expect(onUndone.mock.calls[0][0]).toMatch(/^sess-undo-/);
  });

  it("keeps the failure honest when undo fails (source stays resumable)", async () => {
    (window as never as Seams).__AIJ_SWITCH_UNDO_FAIL__ = "the original conversation is gone";
    const view = render(<ToastHost />);
    act(() => void showSwitchUndoToast({ edgeId: "edge-42", targetLabel: "Personal", onUndone: () => {} }));
    await waitFor(() => expect(view.getByTestId("switch-undo-toast")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-undo"));
    await waitFor(() => expect(view.getByText("Undo didn’t complete")).toBeTruthy());
  });

  it("closes itself after the 10 s window", async () => {
    vi.useFakeTimers();
    try {
      const onDismiss = vi.fn();
      const view = render(<ToastHost />);
      act(() => void showSwitchUndoToast({ edgeId: "edge-42", targetLabel: "Personal", onClose: onDismiss, onUndone: () => {} }));
      expect(view.getByTestId("switch-undo").textContent).toContain("Undo (10s)");
      // One second per act flush — the countdown re-arms via effect,
      // so each tick needs its own commit.
      for (let tick = 0; tick < 10; tick += 1) {
        act(() => {
          vi.advanceTimersByTime(1_000);
        });
      }
      await act(async () => {});
      expect(onDismiss).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("AJ-E1540 a source AI Juggler can't stop", () => {
  it("says it keeps running and offers no stop choice", async () => {
    const view = mount({ ...ONECLICK, source_keeps_running: true }, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    expect(view.getByTestId("switch-keeps-running").textContent).toContain(
      "AI Juggler can't stop this session",
    );
    expect(view.queryByRole("radio", { name: /Stop it now/ })).toBeNull();
  });

  it("offers the choice for a session it started", async () => {
    const view = mount(ONECLICK, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    expect(view.queryByTestId("switch-keeps-running")).toBeNull();
    expect(view.getByRole("radio", { name: /Stop it now/ })).toBeTruthy();
  });
});

describe("AJ-E2108 a session another app runs", () => {
  const OUTSIDE: PreflightInfoData = { ...ONECLICK, source_keeps_running: true, source_outside: true, source_app: "Claude" };

  it("says the original stays open in its app, untouched, and where the new session runs", async () => {
    const view = mount(OUTSIDE, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    expect(view.getByTestId("switch-outside").textContent).toBe(
      "The original stays open in Claude, untouched: AI Juggler only reads its conversation. Personal continues it as a new session here in AI Juggler.",
    );
    expect(view.queryByRole("radio", { name: /Stop it now/ })).toBeNull();
  });

  it("ends on the done step that says so, with no undo window (nothing of ours to go back to)", async () => {
    const onSwitched = vi.fn();
    const view = mount(OUTSIDE, onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-done")).toBeTruthy());
    expect(view.getByTestId("switch-done-words").textContent).toContain("The original stays open in Claude, untouched");
    expect(onSwitched).not.toHaveBeenCalled();
    fireEvent.click(view.getByRole("button", { name: /Go to the new session/ }));
    expect(onSwitched.mock.calls[0][1]).toBeNull();
  });

  it("says so from the wizard's first step", async () => {
    const view = mount({ ...WIZARD, source_keeps_running: true, source_outside: true, source_app: "ChatGPT" }, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    expect(view.getByTestId("switch-outside-target").textContent).toContain("The original stays open in ChatGPT, untouched");
  });

  it("one that isn't open says it stays as it is", async () => {
    const view = mount({ ...OUTSIDE, source_live: false, source_keeps_running: false, source_app: null }, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-outside")).toBeTruthy());
    expect(view.getByTestId("switch-outside").textContent).toContain("The original stays as it is, untouched");
  });
});

/**
 * The maintainer's walk of 0.1.8 (2026-10-27) and §4.9: every step can
 * be cancelled, the dialog has a close ×, but the copy itself can't be
 * walked away from; a failed switch offers Retry.
 */
describe("leaving the switch", () => {
  function mountWith(preflight: PreflightInfoData, onClose = vi.fn(), onSwitched = vi.fn()) {
    (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__ = preflight;
    return render(<SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" onClose={onClose} onSwitched={onSwitched} />);
  }

  it("has Cancel on every wizard step, and a close button", async () => {
    const onClose = vi.fn();
    const view = mountWith(WIZARD, onClose);
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    for (const _ of WIZARD.wizard_steps) {
      expect(view.getByTestId("switch-cancel")).toBeTruthy();
      expect(view.getByTestId("switch-close")).toBeTruthy();
      const next = view.queryByTestId("switch-continue");
      if (next) fireEvent.click(next);
    }
    fireEvent.click(view.getByTestId("switch-cancel"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("keeps the step marks and the buttons out of the scrolling part, on every step", async () => {
    const view = mountWith(WIZARD);
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    const header = view.getByTestId("switch-header");
    const body = view.getByTestId("switch-body");
    for (const _ of WIZARD.wizard_steps) {
      expect(header.contains(view.getByTestId("switch-steps"))).toBe(true);
      const footer = view.getByTestId("switch-footer");
      expect(footer.contains(view.getByTestId("switch-cancel"))).toBe(true);
      expect(body.querySelectorAll("[data-testid=switch-cancel], [data-testid=switch-continue], [data-testid=switch-confirm]").length).toBe(0);
      const next = view.queryByTestId("switch-continue");
      if (next) fireEvent.click(next, { detail: 1 });
    }
    expect(view.getByTestId("switch-footer").contains(view.getByTestId("switch-confirm"))).toBe(true);
  });

  it("keeps Tab inside the dialog: from the last button back to the close", async () => {
    const view = mountWith(ONECLICK);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    const confirm = view.getByTestId("switch-confirm");
    confirm.focus();
    fireEvent.keyDown(window, { key: "Tab" });
    expect(document.activeElement).toBe(view.getByTestId("switch-close"));
    fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(confirm);
  });

  it("waits while the copy runs: Esc says it's finishing, and there's no close", async () => {
    (window as never as Seams).__AIJ_SWITCH_EXECUTE_HANG__ = true;
    const onClose = vi.fn();
    const view = mountWith(ONECLICK, onClose);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-executing")).toBeTruthy());
    expect(view.queryByTestId("switch-close")).toBeNull();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).not.toHaveBeenCalled();
    expect(view.getByTestId("switch-holding").textContent).toContain("Finishing copy");
  });

  it("names its stages in plain words while it runs", async () => {
    (window as never as Seams).__AIJ_SWITCH_EXECUTE_HANG__ = true;
    const view = mountWith(ONECLICK);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-executing")).toBeTruthy());
    const words = view.getByTestId("switch-executing").textContent ?? "";
    expect(words).toContain("Copying the conversation");
    expect(words).not.toMatch(/freeze|stage →|verify →/i);
  });

  it("offers Retry after a failure, and Retry runs the switch again", async () => {
    (window as never as { __AIJ_SWITCH_EXECUTE_FAIL__?: string }).__AIJ_SWITCH_EXECUTE_FAIL__ = "The new session didn't start.";
    const onSwitched = vi.fn();
    const view = mountWith(ONECLICK, vi.fn(), onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-failed")).toBeTruthy());
    expect(view.getByTestId("switch-failed").textContent).toContain("Nothing was changed");
    await waitFor(() => expect(document.activeElement).toBe(view.getByTestId("switch-retry")));
    delete (window as never as { __AIJ_SWITCH_EXECUTE_FAIL__?: string }).__AIJ_SWITCH_EXECUTE_FAIL__;
    fireEvent.click(view.getByTestId("switch-retry"));
    await waitFor(() => expect(onSwitched).toHaveBeenCalledTimes(1));
  });

  it("a double-click on Continue never starts the switch (H-5)", async () => {
    (window as never as Seams).__AIJ_SWITCH_EXECUTE_HANG__ = true;
    const view = mountWith(WIZARD);
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-continue"), { detail: 1 });
    // The second click of a double-click lands on Switch now: ignored.
    fireEvent.click(view.getByTestId("switch-confirm"), { detail: 2 });
    expect(view.queryByTestId("switch-executing")).toBeNull();
    fireEvent.click(view.getByTestId("switch-confirm"), { detail: 1 });
    await waitFor(() => expect(view.getByTestId("switch-executing")).toBeTruthy());
  });

  it("the Target step opens with what moves, and another account is no warning", async () => {
    const view = mountWith(WIZARD);
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    expect(view.getByTestId("switch-move-line").textContent).toContain("The whole conversation moves");
    expect(view.getByTestId("switch-flow").textContent).not.toContain("usual account");
  });
});
