/**
 * AJ-E207, AJ-E1541: switching back: the suggested returns, the plan for
 * one (which account, what changed, conflicts), and running it. A return
 * can go to any account the work passed through, nearest first, or back
 * to where it started. Moved from bridge.ts (the size budget).
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { LaunchResult } from "../../types/generated/LaunchResult";

/** AJ-E207: one tracked-artifact conflict (three radio choices each). */
export interface SwitchBackConflictData {
  rel_path: string;
  base_sha: string | null;
  away_sha: string | null;
  original_sha: string | null;
}

/** AJ-E207: the switch-back plan (suggest-only; the user clicks). */
export interface SwitchBackPlanData {
  suggestion: string;
  original_account: string;
  away_account: string;
  original_native_id: string;
  original_alive: boolean;
  sf7_fallback: string;
  sf7_copy: string;
  away_live: boolean;
  both_live_copy: string;
  divergence: string;
  delta_preview: string;
  away_only_turns: number;
  original_only_turns: number;
  base_turns: number;
  conflicts: SwitchBackConflictData[];
  /** AJ-E1541: every account this work can go back to, nearest first. */
  returns: SwitchBackReturnData[];
  /** The epoch this plan returns to (one of `returns`). */
  return_to: string;
}

/** AJ-E1541: one account a switch back can return to. */
export interface SwitchBackReturnData {
  /** Pass back as `toEpoch` to plan or execute a return there. */
  epoch_id: string;
  account: string;
  /** Where the work started. */
  start: boolean;
}

/** AJ-E207: one suggested return (the card the user clicks). */
export interface SwitchBackSuggestionData {
  session_id: string;
  original_account: string;
  /** The original account's id (its "has room again" notice offers this return). */
  original_account_id?: string;
  away_account: string;
  copy: string;
}

/** AJ-E207 Req.3: the suggested returns whose original account has
 * quota headroom again (the reset trigger) — suggest-only. */
export async function switchBackSuggestions(): Promise<SwitchBackSuggestionData[]> {
  if (!hasLiveBackend()) {
    return (
      (window as unknown as { __AIJ_SWITCH_BACK_SUGGESTIONS__?: SwitchBackSuggestionData[] })
        .__AIJ_SWITCH_BACK_SUGGESTIONS__ ?? []
    );
  }
  return invoke<SwitchBackSuggestionData[]>("switch_back_suggestions");
}

/** AJ-E207: the plan for one return — verbatim suggestion, S-F7
 * verdict, both-live warning, divergence counts, delta preview,
 * conflict list. Read-only. */
export async function switchBackPlan(
  sessionId: string,
  toEpoch?: string,
): Promise<SwitchBackPlanData> {
  if (!hasLiveBackend()) {
    return (
      (window as unknown as { __AIJ_SWITCH_BACK_PLAN__?: SwitchBackPlanData })
        .__AIJ_SWITCH_BACK_PLAN__ ?? {
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
          "While you were away, this work continued on Personal — 3 new turns.\n\n- You: fix the login bug\n- Personal: Fixed it, and the suite is green (34 passing).\n\nThe project's files are as they were left on Personal. Pick up from here.",
        away_only_turns: 3,
        original_only_turns: 1,
        base_turns: 5,
        conflicts: [
          { rel_path: "todos.json", base_sha: "h0", away_sha: "h1", original_sha: "h2" },
        ],
        returns: [{ epoch_id: "epoch-a", account: "Work Max", start: true }],
        return_to: "epoch-a",
      }
    );
  }
  return invoke<SwitchBackPlanData>("switch_back_plan", { sessionId, toEpoch: toEpoch ?? null });
}

/** AJ-E207: executes the return with the user's choices — resumes
 * the ORIGINAL native session with one delta turn; never a
 * copy-back. */
export async function switchBackExecute(
  sessionId: string,
  choices: { rel_path: string; choice: string }[],
  keepAwayRunning: boolean,
  toEpoch?: string,
): Promise<LaunchResult> {
  if (!hasLiveBackend()) {
    const seams = window as unknown as {
      __AIJ_SWITCH_BACK_EXECUTE_FAIL__?: string;
      __AIJ_SWITCH_BACK_EXECUTE_HANG__?: boolean;
      __AIJ_SWITCH_BACK_EXECUTE_CALLS__?: unknown[];
    };
    // The guardrail seam: executions are observable so tests can
    // prove they fire ONLY from the click.
    seams.__AIJ_SWITCH_BACK_EXECUTE_CALLS__ = [
      ...(seams.__AIJ_SWITCH_BACK_EXECUTE_CALLS__ ?? []),
      { sessionId, choices, keepAwayRunning, toEpoch },
    ];
    if (seams.__AIJ_SWITCH_BACK_EXECUTE_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_SWITCH_BACK_EXECUTE_FAIL__);
    }
    if (seams.__AIJ_SWITCH_BACK_EXECUTE_HANG__) {
      return new Promise<LaunchResult>(() => {});
    }
    return { session_id: `sess-return-${Date.now()}`, pid: 0 };
  }
  return invoke<LaunchResult>("switch_back_execute", {
    sessionId,
    choices,
    keepAwayRunning,
    toEpoch: toEpoch ?? null,
  });
}
