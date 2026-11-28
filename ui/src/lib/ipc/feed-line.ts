/**
 * AJ-E3831 (D-154) — a finished turn's AI summary for its feed post over
 * IPC (`feed_line`, its opt-in Beta: never blocking the post), and "This
 * is wrong" about it (`feed_line_wrong`, kept on this Mac).
 *
 * The preview and tests set `window.__AIJ_FEED_LINES__` by the turn's
 * session or conversation id; with none, a turn has no summary ("none").
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { FeedLineInfo } from "../../types/generated/FeedLineInfo";
import type { ReviewAsk } from "../../types/generated/ReviewAsk";

export type { FeedLineInfo };

declare global {
  interface Window {
    __AIJ_FEED_LINES__?: Record<string, FeedLineInfo>;
    __AIJ_FEED_LINE_WRONG__?: string[];
  }
}

const NONE: FeedLineInfo = { state: "none", turn: "", line: null, excerpt: null, model: null };

/** The summary of the turn `ask` names (the agent's own words always; the
 * model's line when the Beta runs and a draft passed). */
export async function feedLine(ask: ReviewAsk): Promise<FeedLineInfo> {
  if (!hasLiveBackend()) return window.__AIJ_FEED_LINES__?.[ask.conversation_id ?? ask.session_id ?? ""] ?? NONE;
  return call("feed_line", { ask });
}

/** "This is wrong" about a summary (`wrong` false: its Undo). */
export async function feedLineWrong(turn: string, line: string, wrong = true): Promise<void> {
  if (!hasLiveBackend()) {
    window.__AIJ_FEED_LINE_WRONG__ = [...(window.__AIJ_FEED_LINE_WRONG__ ?? []).filter((one) => one !== turn), ...(wrong ? [turn] : [])];
    return;
  }
  return call("feed_line_wrong", { turn, line, wrong });
}
