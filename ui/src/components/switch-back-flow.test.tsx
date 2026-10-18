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
    "While you were away on Personal — 3 new turns (5 shared before the switch).\n\n- [user] fixed the login bug\n\nFull detail: the attached switch pack (not replayed).",
  away_only_turns: 3,
  original_only_turns: 0,
  base_turns: 5,
  conflicts: [
    { rel_path: "todos.json", base_sha: "h0", away_sha: "h1", original_sha: "h2" },
  ],
};

type Seams = {
  __AIJ_SWITCH_BACK_PLAN__?: SwitchBackPlanData;
  __AIJ_SWITCH_BACK_EXECUTE_FAIL__?: string;
  __AIJ_SWITCH_BACK_EXECUTE_CALLS__?: {
    sessionId: string;
    choices: { rel_path: string; choice: string }[];
    keepAwayRunning: boolean;
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
      "S-F8: merge conflict unresolved — work stays forked, both sides resumable";
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
      sf7_copy: "S-F7: the original session is gone — nearest epoch or pack-primed fresh start",
      conflicts: [],
    };
    const view = render(
      <SwitchBackFlow sessionId="sess-away-1" onClose={() => {}} onSwitched={() => {}} />,
    );
    await waitFor(() => expect(view.getByTestId("switch-back-review")).toBeTruthy());
    expect(view.getByTestId("switch-back-sf7").textContent).toContain("S-F7");
    expect(view.getByTestId("switch-back-sf7").textContent).toContain("fresh instance");
  });

  it("suggests via the card copy verbatim (the plan's suggestion line)", async () => {
    const view = mount();
    await waitFor(() => expect(view.getByTestId("switch-back-suggestion")).toBeTruthy());
    expect(view.getByTestId("switch-back-suggestion").textContent).toBe(PLAN.suggestion);
    expect(view.getByTestId("switch-back-delta").textContent).toContain(
      "While you were away on Personal",
    );
    expect(view.queryByText(/we switched/i)).toBeNull();
  });
});
