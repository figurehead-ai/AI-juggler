/**
 * AJ-E2910: Home's Mark all reviewed (every session on Pick up where you
 * left off at once, one call), and its Undo. The preview's seam
 * `__AIJ_TRIAGE_CALLS__` records them as the single Mark reviewed does.
 */
import { hasLiveBackend } from "../bridge";
import { call } from "./transport";
import type { ReviewedAllInfo } from "../../types/generated/ReviewedAllInfo";
import type { SessionsQuery } from "../../types/generated/SessionsQuery";

/** Marks `sessionIds` reviewed, or (`back`) puts them back on the list. */
export async function instancesMarkReviewed(sessionIds: string[], back = false): Promise<number> {
  if (!hasLiveBackend()) {
    const w = window as unknown as { __AIJ_TRIAGE_CALLS__?: string[] };
    w.__AIJ_TRIAGE_CALLS__ = [...(w.__AIJ_TRIAGE_CALLS__ ?? []), `${back ? "unreviewed" : "reviewed"}-all:${sessionIds.join(",")}`];
    return Date.now();
  }
  if (back) return call("instances_unmark_reviewed", { sessionIds });
  return call("instances_mark_reviewed", { sessionIds });
}

/** The 0.4.6 RC walk K2 (F-K2-7a): marks every session `query`'s list
 * holds reviewed (read as `to_review`), not only those the app holds;
 * how many, and the batch its Undo puts back. */
export async function sessionsMarkReviewed(query: SessionsQuery): Promise<ReviewedAllInfo> {
  return call("sessions_mark_reviewed", { query });
}

/** Undoes one Mark all reviewed (batch `atMs`): how many came back. */
export async function sessionsUnmarkReviewed(atMs: number): Promise<number> {
  return call("sessions_unmark_reviewed", { atMs });
}
