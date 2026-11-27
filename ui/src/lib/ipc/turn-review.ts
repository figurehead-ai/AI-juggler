/**
 * AJ-E3820 — a turn's review over IPC: the code-only review at once
 * (`review_turn`), the model's Beta lines after it when the review job
 * runs (`review_turn_ai`, never blocking the page), and "This is wrong"
 * about those lines (`review_wrong`, kept on this Mac).
 *
 * The preview and tests set `window.__AIJ_REVIEWS__` (the code-only
 * reviews) and `window.__AIJ_REVIEWS_AI__` (with the model's lines), each
 * by the turn's session or conversation id; with none, a turn has no
 * review ("none"), as for a transcript that isn't on this Mac.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { ReviewAsk } from "../../types/generated/ReviewAsk";
import type { TurnReviewInfo } from "../../types/generated/TurnReviewInfo";

export type { ReviewAsk, TurnReviewInfo };

declare global {
  interface Window {
    __AIJ_REVIEWS__?: Record<string, TurnReviewInfo>;
    __AIJ_REVIEWS_AI__?: Record<string, TurnReviewInfo>;
    __AIJ_REVIEW_WRONG__?: string[];
    /** How long the preview's model takes over its lines (0.5.2 G28's
     * cue); never answered when `Infinity`. */
    __AIJ_REVIEWS_AI_MS__?: number;
  }
}

/** No review: what a turn with no transcript here answers. */
const NO_REVIEW: TurnReviewInfo = {
  state: "none", turn: "", files: 0, changed: [], reverted: [], undone_unclear: false, reverted_words: null, failures: [], question: null,
  tests: null, tests_words: null, risk: "low", risk_why: [], ai: null,
};

function seeded(map: Record<string, TurnReviewInfo> | undefined, ask: ReviewAsk): TurnReviewInfo {
  const key = ask.conversation_id ?? ask.session_id ?? "";
  return map?.[key] ?? NO_REVIEW;
}

/** The code-only review of the turn `ask` names. */
export async function reviewTurn(ask: ReviewAsk): Promise<TurnReviewInfo> {
  if (!hasLiveBackend()) return seeded(window.__AIJ_REVIEWS__, ask);
  return call("review_turn", { ask });
}

/** The review with the model's lines, when its Beta runs (else the
 * code-only review, unchanged). */
export async function reviewTurnAi(ask: ReviewAsk): Promise<TurnReviewInfo> {
  if (!hasLiveBackend()) {
    const got = seeded(window.__AIJ_REVIEWS_AI__ ?? window.__AIJ_REVIEWS__, ask);
    const ms = window.__AIJ_REVIEWS_AI_MS__;
    if (!ms) return got;
    return new Promise((resolve) => {
      if (Number.isFinite(ms)) setTimeout(() => resolve(got), ms);
    });
  }
  return call("review_turn_ai", { ask });
}

/** "This is wrong" about a review's model lines (`wrong` false: its
 * Undo); `lines` are the lines shown, kept on this Mac for the next
 * round. */
export async function reviewWrong(turn: string, lines: string, wrong = true): Promise<void> {
  if (!hasLiveBackend()) {
    window.__AIJ_REVIEW_WRONG__ = [...(window.__AIJ_REVIEW_WRONG__ ?? []).filter((one) => one !== turn), ...(wrong ? [turn] : [])];
    return;
  }
  return call("review_wrong", { turn, lines, wrong });
}
