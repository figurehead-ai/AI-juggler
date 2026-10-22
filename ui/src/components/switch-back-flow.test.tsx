import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SwitchBackFlow } from "./SwitchBackFlow";
import type { SwitchBackPlanData } from "../lib/bridge";

/**
 * AJ-E207 guardrails: the return executes ONLY from the click (the
 * suggestion is a card, never an action); conflicts are radio trios
 * resolvable keyboard-only; the return stays blocked (S-F8 copy)
 * until every conflict has a choice; and the away instance's
 * freeze-or-fork choice is explicit before resume.
 */

const PLAN: SwitchBackPlanData = {
  suggestion:
    "Work Max has reset. Resume the original conversation there? Work from Personal arrives as one 'while you were away' update.",
  original_account: "Work Max",
  away_account: "Personal",
  original_native_id: "native-5",
  original_alive: true,
  sf7_fallback: "",
  sf7_copy: "",
  away_live: true,
  both_live_copy:
    "Both accounts are now live on this work item: Work Max (resumed) and Personal (still running). Check which one you type into.",
  divergence:
    "Both sides moved since the switch. 3 new turns and 1 file conflicts need your choice — nothing is merged silently.",
  delta_preview:
    "While you were away, this work continued on Personal — 3 new turns.\n\n- You: fix the login bug\n\nThe project's files are as they were left on Personal. Pick up from here.",
  away_only_turns: 3,
  original_only_turns: 0,
  base_turns: 5,
  conflicts: [
    { rel_path: "todos.json", base_sha: "h0", away_sha: "h1", original_sha: "h2" },
  ],
  returns: [{ epoch_id: "epoch-a", account: "Work Max", start: true }],
  return_to: "epoch-a",
};

type Seams = {
  __AIJ_SWITCH_BACK_PLAN__?: SwitchBackPlanData;
  __AIJ_SWITCH_BACK_EXECUTE_FAIL__?: string;
  __AIJ_SWITCH_BACK_EXECUTE_CALLS__?: {
    sessionId: string;
    choices: { rel_path: string; choice: string }[];
    keepAwayRunning: boolean;
    toEpoch?: string;
  }[];
};

beforeEach(() => {
  delete (window as never as Seams).__AIJ_SWITCH_BACK_PLAN__;
  delete (window as never as Seams).__AIJ_SWITCH_BACK_EXECUTE_FAIL__;
  delete (window as never as Seams).__AIJ_SWITCH_BACK_EXECUTE_CALLS__;
});
afterEach(cleanup);

function mount() {
  (window as never as Seams).__AIJ_SWITCH_BACK_PLAN__ = PLAN;
  return render(
    <SwitchBackFlow sessionId="sess-away-1" onClose={() => {}} onSwitched={() => {}} />,
  );
}

describe("AJ-E207 switch-back flow", () => {
  it("executes nothing until the user clicks (suggest-only)", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-back-review")).toBeTruthy());
    // The review is up; the engine has NOT been called.
    expect((window as never as Seams).__AIJ_SWITCH_BACK_EXECUTE_CALLS__).toBeUndefined();
    // S-F8: a conflict without a choice disables the return.
    const confirm = view.getByTestId("switch-back-confirm") as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    expect(view.getByTestId("switch-back-unresolved").textContent).toContain(
      "stays forked",
    );
  });

  it("resolves a conflict keyboard-style, then executes once from the click", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-back-review")).toBeTruthy());
    // Radio trio per conflict — keyboard-completable (label + input).
    fireEvent.click(view.getByLabelText("Take the away version"));
    const confirm = view.getByTestId("switch-back-confirm") as HTMLButtonElement;
    await waitFor(() => expect(confirm.disabled).toBe(false));
    fireEvent.click(confirm);
    await new Promise((resolve) => setTimeout(resolve, 50));
    const callsNow = (window as never as Seams).__AIJ_SWITCH_BACK_EXECUTE_CALLS__;
    expect(callsNow).toHaveLength(1);
    expect(callsNow![0].sessionId).toBe("sess-away-1");
    expect(callsNow![0].choices).toEqual([{ rel_path: "todos.json", choice: "take-away" }]);
    expect(callsNow![0].keepAwayRunning).toBe(false);
    expect(view.getByTestId("switch-back-done")).toBeTruthy();
    expect(view.getByText(/resumed exactly where it rested/)).toBeTruthy();
  });

  it("states both-live out loud when the away instance keeps running", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-back-review")).toBeTruthy());
    fireEvent.click(view.getByLabelText("Keep it running"));
    await waitFor(() => expect(view.getByTestId("switch-back-both-live")).toBeTruthy());
    expect(view.getByTestId("switch-back-both-live").textContent).toContain(
      "Both accounts are now live on this work item",
    );
  });

  it("renders the S-F8 forked copy when the return is refused", async () => {
    (window as never as Seams).__AIJ_SWITCH_BACK_EXECUTE_FAIL__ =
      "files changed on both sides and need your choice — the work stays split, and both sides can still be resumed";
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-back-review")).toBeTruthy());
    fireEvent.click(view.getByLabelText("Keep original"));
    fireEvent.click(view.getByTestId("switch-back-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-back-failed")).toBeTruthy());
    expect(view.getByTestId("switch-back-failed").textContent).toContain(
      "both sides stay resumable",
    );
  });

  it("shows the honest S-F7 state when the original is gone", async () => {
    (window as never as Seams).__AIJ_SWITCH_BACK_PLAN__ = {
      ...PLAN,
      original_alive: false,
      original_native_id: "",
      sf7_fallback: "fresh-f0",
      sf7_copy: "The original conversation on Work Max is gone, so it can't be resumed",
      conflicts: [],
    };
    const view = render(
      <SwitchBackFlow sessionId="sess-away-1" onClose={() => {}} onSwitched={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("switch-back-review")).toBeTruthy());
    expect(view.getByTestId("switch-back-sf7").textContent).toContain("is gone");
    expect(view.getByTestId("switch-back-sf7").textContent).toContain("fresh session");
  });

  it("suggests via the card copy verbatim (the plan's suggestion line)", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-back-suggestion")).toBeTruthy());
    expect(view.getByTestId("switch-back-suggestion").textContent).toBe(PLAN.suggestion);
    expect(view.getByTestId("switch-back-delta").textContent).toContain(
      "While you were away, this work continued on Personal",
    );
    expect(view.queryByText(/we switched/i)).toBeNull();
  });
});

describe("AJ-E1541: where to go back to", () => {
  // A → B → Codex: from Codex the work can go back to B, or to A where it started.
  const HOPS: SwitchBackPlanData = {
    ...PLAN,
    original_account: "Personal",
    away_account: "Codex",
    conflicts: [],
    returns: [
      { epoch_id: "epoch-b", account: "Personal", start: false },
      { epoch_id: "epoch-a", account: "Work Max", start: true },
    ],
    return_to: "epoch-b",
  };

  it("offers each account, nearest first, and returns where the user picks", async () => {
    (window as never as Seams).__AIJ_SWITCH_BACK_PLAN__ = HOPS;
    const view = render(
      <SwitchBackFlow sessionId="sess-codex" onClose={() => {}} onSwitched={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("switch-back-returns")).toBeTruthy());
    expect((view.getByLabelText("Personal") as HTMLInputElement).checked).toBe(true);
    expect(view.getByText("(where it started)")).toBeTruthy();

    (window as never as Seams).__AIJ_SWITCH_BACK_PLAN__ = {
      ...HOPS,
      original_account: "Work Max",
      return_to: "epoch-a",
    };
    fireEvent.click(view.getByLabelText(/Work Max/));
    await waitFor(() =>
      expect(view.getByTestId("switch-back-confirm").textContent).toContain("Resume Work Max"),
    );
    fireEvent.click(view.getByTestId("switch-back-confirm"));
    await waitFor(() => expect(view.getByTestId("switch-back-done")).toBeTruthy());
    const calls = (window as never as Seams).__AIJ_SWITCH_BACK_EXECUTE_CALLS__ ?? [];
    expect(calls.at(-1)?.toEpoch).toBe("epoch-a");
  });

  it("shows no choice when there is only one place to go back to", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-back-review")).toBeTruthy());
    expect(view.queryByTestId("switch-back-returns")).toBeNull();
  });
});
