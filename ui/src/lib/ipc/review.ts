/**
 * AJ-E2910: Home's Mark all reviewed (every session on Pick up where you
 * left off at once, one call), and its Undo. The preview's seam
 * `__AIJ_TRIAGE_CALLS__` records them as the single Mark reviewed does.
 */
import { hasLiveBackend } from "../bridge";
import { call } from "./transport";

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
