import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow, SwitchUndoToast } from "./SwitchFlow";
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
  score: 1.0,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: true,
  experimental_copy:
    "Experimental: only the main transcript moves. Subagent files and rewind history stay on the source. The source stays resumable.",
  door: "one-click",
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: true,
  carry: {
    carried: ["chat transcript", "rules & guidelines"],
    regenerated: [],
    dropped: [],
  },
  redaction: {
    global: "strip",
    override: null,
    effective: "strip",
    source: "global",
    line: "Redaction for claude-code: Strip — replace keys with ••• (your global choice)",
  },
  redaction_items: [],
};

const WIZARD: PreflightInfoData = {
  ...ONECLICK,
  fidelity: "F2",
  door: "wizard",
  wizard_steps: ["Target", "What's carried", "Confirm"],
  wizard_reasons: ["This isn't the usual account for this project."],
  usual_account: false,
  carry: {
    carried: ["chat transcript"],
    regenerated: [],
    dropped: ["todo list"],
  },
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
afterEach(cleanup);

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
    expect(view.getByTestId("switch-carry-dropped").textContent).toContain("todo list");
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
    fireEvent.click(view.getByRole("button", { name: /Go to the new instance/ }));
    expect(onSwitched).toHaveBeenCalledTimes(1);
    expect(onSwitched.mock.calls[0][1]).toBeNull();
  });
});

describe("AJ-E206 undo window", () => {
  it("undoes ONLY from the click (never on render), then hands back the resumed session", async () => {
    const onUndone = vi.fn();
    const onDismiss = vi.fn();
    const view = render(
      <SwitchUndoToast edgeId="edge-42" targetLabel="Personal" onDismiss={onDismiss} onUndone={onUndone} />,
    );
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
    (window as never as Seams).__AIJ_SWITCH_UNDO_FAIL__ = "S-F7: the source session is gone";
    const view = render(
      <SwitchUndoToast edgeId="edge-42" targetLabel="Personal" onDismiss={() => {}} onUndone={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("switch-undo-toast")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-undo"));
    await waitFor(() => expect(view.getByText("Undo didn’t complete")).toBeTruthy());
  });

  it("closes itself after the 10 s window", async () => {
    vi.useFakeTimers();
    try {
      const onDismiss = vi.fn();
      const view = render(
        <SwitchUndoToast edgeId="edge-42" targetLabel="Personal" onDismiss={onDismiss} onUndone={() => {}} />,
      );
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
