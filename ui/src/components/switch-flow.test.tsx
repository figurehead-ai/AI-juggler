import { act, cleanup, fireEvent, render, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SwitchFlow, showSwitchUndoToast } from "./SwitchFlow";
import { resetToasts, ToastHost } from "./Toast";
import { closeReportBug, useReportBug } from "../lib/report-bug";
import type { PreflightInfoData } from "../lib/ipc/switch-preflight";

// The backend's switch progress events, sent by hand (0.3.9).
const events = vi.hoisted(() => new Map<string, (event: { payload: unknown }) => void>());
vi.mock("../lib/ipc/transport", async (original) => ({
  ...(await original<typeof import("../lib/ipc/transport")>()),
  listen: (name: string, handler: (event: { payload: unknown }) => void) => {
    events.set(name, handler);
    return Promise.resolve(() => events.delete(name));
  },
}));

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
  move_line: "The whole conversation is carried over — all 42 messages, its subagents and file history.",
  secrets: { asks: false, reader: "Claude Code (Anthropic)" },
  target_tool: "claude-code",
};

const WIZARD: PreflightInfoData = {
  ...ONECLICK,
  fidelity: "F2",
  fidelity_words: "Same conversation, another app",
  door: "wizard",
  wizard_steps: ["Target", "What's carried", "Confirm"],
  wizard_reasons: ["This switch hasn't been proven on a real setup yet."],
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


/** A failed launch's words, as the backend sends them (`SwitchFailure::words`). */
const LAUNCH_FAILED = "The new session on Personal didn't start, so nothing moved. Try again, or resume it on Work where you left it.";

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
    const [sessionId, undo] = onSwitched.mock.calls[0]!;
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
    // 0.4: the conversation that moves is named before anything does.
    expect(view.getByTestId("switch-source").textContent).toBe("Copies “Fix the checkout flow” from Work");
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
    expect(onSwitched.mock.calls[0]![1]).toBeNull();
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
    expect(onUndone.mock.calls[0]![0]).toMatch(/^sess-undo-/);
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
    expect(view.getByTestId("switch-outside").textContent).toBe("The original stays open in Claude, untouched. It continues in a terminal (CLI).");
    // 0.3.5: the button says where it goes on, Desktop or CLI.
    expect(view.getByTestId("switch-confirm").textContent).toBe("Switch to CLI");
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
    expect(onSwitched.mock.calls[0]![1]).toBeNull();
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
    (window as never as { __AIJ_SWITCH_EXECUTE_FAIL__?: unknown }).__AIJ_SWITCH_EXECUTE_FAIL__ = { code: "switch_failed", retryable: true, user_message: LAUNCH_FAILED };
    const onSwitched = vi.fn();
    const view = mountWith(ONECLICK, vi.fn(), onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-failed")).toBeTruthy());
    expect(view.getByTestId("switch-failed-words").textContent).toBe(LAUNCH_FAILED);
    await waitFor(() => expect(document.activeElement).toBe(view.getByTestId("switch-retry")));
    delete (window as never as { __AIJ_SWITCH_EXECUTE_FAIL__?: string }).__AIJ_SWITCH_EXECUTE_FAIL__;
    fireEvent.click(view.getByTestId("switch-retry"));
    await waitFor(() => expect(onSwitched).toHaveBeenCalledTimes(1));
  });

  it("a failed switch says each thing once: a short title, then what happened and what to do next", async () => {
    (window as never as { __AIJ_SWITCH_EXECUTE_FAIL__?: unknown }).__AIJ_SWITCH_EXECUTE_FAIL__ = { code: "switch_failed", retryable: true, user_message: LAUNCH_FAILED };
    const view = mountWith(ONECLICK);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-failed")).toBeTruthy());
    const text = view.getByTestId("switch-failed").textContent ?? "";
    const times = (words: string) => text.split(words).length - 1;
    expect(view.getByRole("heading", { name: "The switch didn't complete" })).toBeTruthy();
    expect(times(LAUNCH_FAILED)).toBe(1);
    // What happened and what's next, each once (the 0.1.9 dialog said
    // "nothing moved", then "Nothing was changed … or try again").
    expect(times("nothing moved")).toBe(1);
    expect(text.match(/nothing (was )?changed/gi)).toBeNull();
    expect(text.match(/try again/gi)).toHaveLength(1);
    expect(text.match(/where you left it/gi)).toHaveLength(1);
    delete (window as never as { __AIJ_SWITCH_EXECUTE_FAIL__?: unknown }).__AIJ_SWITCH_EXECUTE_FAIL__;
  });

  it("marks each stage as the backend reaches it (0.3.9: a still screen for seconds)", async () => {
    (window as never as Seams).__AIJ_SWITCH_EXECUTE_HANG__ = true;
    const view = mountWith(ONECLICK);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(events.has("switch://progress")).toBe(true));
    const states = () => view.getAllByTestId("switch-stage").map((stage) => stage.getAttribute("data-state"));
    const now = () => states().indexOf("now");
    const first = now();
    act(() => events.get("switch://progress")!({ payload: "checking" }));
    expect(now()).toBe(first + 1);
    act(() => events.get("switch://progress")!({ payload: "starting" }));
    expect(now()).toBe(first + 2);
    expect(states().slice(0, first + 2).every((state) => state === "done")).toBe(true);
    // A late or repeated event never moves it back.
    act(() => events.get("switch://progress")!({ payload: "copying" }));
    expect(now()).toBe(first + 2);
    expect(view.getByTestId("switch-progress").getAttribute("aria-valuenow")).toBe(String(first + 2));
  });

  it("a failure a retry can't fix offers no Retry, and Copy details carries what the program said", async () => {
    const words = "This conversation is too large to carry to Personal in one piece, so nothing moved. Continue it with a summary on Personal instead.";
    (window as never as { __AIJ_SWITCH_EXECUTE_FAIL__?: unknown }).__AIJ_SWITCH_EXECUTE_FAIL__ = {
      code: "switch_failed",
      retryable: false,
      user_message: words,
      detail: "adapter: the session file is too large to read (at Stage)",
    };
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: (text: string) => (written.push(text), Promise.resolve()) } });
    const view = mountWith(ONECLICK);
    await waitFor(() => expect(view.getByTestId("switch-oneclick")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-failed")).toBeTruthy());
    expect(view.queryByTestId("switch-retry")).toBeNull();
    fireEvent.click(view.getByTestId("switch-copy-details"));
    await waitFor(() => expect(view.getByRole("status").textContent).toContain("Copied"));
    expect(written[0]).toContain("a switch to Personal didn't complete");
    expect(written[0]).toContain(words);
    expect(written[0]).toContain("too large to read (at Stage)");
    // AJ-E2911: Report a bug opens with what the screen showed.
    const asked = renderHook(() => useReportBug());
    act(() => fireEvent.click(view.getByTestId("switch-report-bug")));
    expect(asked.result.current).toMatchObject({ from: "switch", words, detail: "adapter: the session file is too large to read (at Stage)" });
    closeReportBug();
    delete (window as never as { __AIJ_SWITCH_EXECUTE_FAIL__?: unknown }).__AIJ_SWITCH_EXECUTE_FAIL__;
  });

  it("says the disk space it uses, and waits while there isn't enough (0.3.9)", async () => {
    type SpaceSeam = { __AIJ_SWITCH_SPACE__?: unknown };
    (window as never as SpaceSeam).__AIJ_SWITCH_SPACE__ = {
      kind: "short",
      words: "Not enough disk space: the copy needs about 8.0 GB and AI Juggler always leaves 4 GB free, but 10 GB is free.",
      advice: "Free about 2.3 GB (empty the Trash, or remove large files you don't need), then switch. Nothing has moved.",
      carried_bytes: 8e9,
      writes_bytes: 8e9,
      free_bytes: 1e10,
    };
    const onSwitched = vi.fn();
    const view = mountWith(ONECLICK, vi.fn(), onSwitched);
    await waitFor(() => expect(view.getByTestId("switch-disk-space")).toBeTruthy());
    expect(view.getByTestId("switch-disk-space").textContent).toContain("Not enough disk space");
    expect(view.getByTestId("switch-disk-space-advice").textContent).toContain("Free about 2.3 GB");
    const confirm = view.getByTestId("switch-confirm") as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    fireEvent.click(confirm);
    expect(onSwitched).not.toHaveBeenCalled();
    // Room made: Check again reads it afresh, and the switch can go.
    (window as never as SpaceSeam).__AIJ_SWITCH_SPACE__ = {
      kind: "plenty",
      words: "This conversation is 8.0 GB; the copy takes about 8.0 GB of disk space (30 GB free).",
      advice: null,
      carried_bytes: 8e9,
      writes_bytes: 8e9,
      free_bytes: 3e10,
    };
    fireEvent.click(view.getByTestId("switch-disk-space-again"));
    await waitFor(() => expect((view.getByTestId("switch-confirm") as HTMLButtonElement).disabled).toBe(false));
    expect(view.getByTestId("switch-disk-space").textContent).toContain("30 GB free");
    expect(view.queryByTestId("switch-disk-space-advice")).toBeNull();
    delete (window as never as SpaceSeam).__AIJ_SWITCH_SPACE__;
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
    expect(view.getByTestId("switch-move-line").textContent).toContain("The whole conversation is carried over");
    expect(view.getByTestId("switch-flow").textContent).not.toContain("usual account");
  });
});

/** The 0.2 re-render: the confirm said "The original stays" twice (the
 * move line, then the original's own line). Each step says it once. */
describe("the original, said once", () => {
  // What happens to the original: it stays, stops, or keeps running.
  const times = (text: string | null | undefined) => (text ?? "").match(/The original|can't stop this session/g)?.length ?? 0;
  const cases: Array<[string, Partial<PreflightInfoData>]> = [
    ["at rest", { source_live: false }],
    ["running here", { source_live: true }],
    ["one it can't stop", { source_keeps_running: true }],
    ["one another app runs", { source_keeps_running: true, source_outside: true, source_app: "Claude" }],
  ];
  for (const [name, change] of cases) {
    it(`on the confirm: ${name}`, async () => {
      const view = mount({ ...ONECLICK, ...change }, vi.fn());
      await waitFor(() => expect(view.getByTestId("switch-summary")).toBeTruthy());
      expect(times(view.getByTestId("switch-summary").textContent)).toBe(1);
    });
    it(`on the Target step: ${name}`, async () => {
      (window as never as Seams).__AIJ_SWITCH_PREFLIGHT__ = { ...WIZARD, ...change };
      const view = render(<SwitchFlow sessionId="sess-1" targetProfileId="p-1" targetLabel="Personal" onClose={vi.fn()} onSwitched={vi.fn()} />);
      await waitFor(() => expect(view.getByTestId("switch-step-target")).toBeTruthy());
      expect(times(view.getByTestId("switch-step-target").textContent)).toBe(1);
    });
  }
});

describe("0.4.1 (rc5): every screen says the same about the original", () => {
  // The walk: step 1 said the original stays, step 3 and the done screen
  // that it stops. Both are true of a running original (it stops, then
  // stays); one at rest only stays, so no screen says it stops.
  const walk = async (change: Partial<PreflightInfoData>) => {
    const view = mount({ ...WIZARD, ...change }, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-step-target")).toBeTruthy());
    const target = view.getByTestId("switch-move-line").textContent ?? "";
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-continue"));
    await waitFor(() => expect(view.getByTestId("switch-original")).toBeTruthy());
    const confirm = view.getByTestId("switch-original").textContent ?? "";
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-done-words")).toBeTruthy());
    return [target, confirm, view.getByTestId("switch-done-words").textContent ?? ""];
  };

  it("a running original stops, then stays as it was, on all three", async () => {
    const screens = await walk({ source_live: true });
    for (const words of screens) {
      expect(words).toMatch(/stop/);
      expect(words).toContain("stays as it was");
      expect(words).toContain("You can switch back to it any time.");
    }
  });

  it("an original at rest only stays: no screen says it stops", async () => {
    const screens = await walk({ source_live: false });
    for (const words of screens) {
      expect(words).not.toMatch(/stop/);
      expect(words).toContain("The original stays as it was. You can switch back to it any time.");
    }
  });

  it("the confirm says the conversation is copied, never moved", async () => {
    const view = mount(ONECLICK, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-source")).toBeTruthy());
    expect(view.getByTestId("switch-source").textContent).toMatch(/^Copies /);
  });
});

describe("0.3 what the other tool was told", () => {
  afterEach(() => {
    delete (window as never as Record<string, unknown>).__AIJ_HANDOVER_SUMMARY__;
  });

  it("a done switch that wrote a handover says what was told and opens it", async () => {
    (window as never as Record<string, unknown>).__AIJ_HANDOVER_SUMMARY__ = {
      "*": { to_tool: "Codex", line: "Hidden from Codex: 2 API keys" },
    };
    const view = mount(WIZARD, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("handover-line").textContent).toBe("Hidden from Codex: 2 API keys"));
    expect(view.getByRole("button", { name: "Open the handover" })).toBeTruthy();
  });

  it("a done switch on the same tool shows nothing of the kind", async () => {
    const view = mount(WIZARD, vi.fn());
    await waitFor(() => expect(view.getByTestId("switch-wizard")).toBeTruthy());
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-continue"));
    fireEvent.click(view.getByTestId("switch-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-done")).toBeTruthy());
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(view.queryByTestId("switch-handover")).toBeNull();
  });
});
