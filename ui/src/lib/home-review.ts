/**
 * Home's "Pick up where you left off" (AJ-E1005, D-25): AI Juggler's
 * sessions that finished or were stopped this week and aren't marked
 * reviewed or archived.
 *
 * The 0.4.6 RC walk K2 (F-K2-7a): it counted only the sessions the app
 * holds (at most 200 ended in 7 days), so "Show all 200" and "Mark all
 * 200 reviewed" said "all" with more to review. Now the count is the
 * backend's (`sessions_counts` over its `to_review` list in the window),
 * Show all pages every one from that list, and Mark all marks every one
 * (`sessions_mark_reviewed`), with Undo. The working set's rows still show
 * at once; the design preview (no backend) has only those.
 */
import { useEffect, useMemo, useState } from "react";
import { hasLiveBackend } from "./bridge";
import { sessionsCounts, sessionsPage, sessionsQuery, type SessionsQuery } from "./ipc/sessions";
import { instancesMarkReviewed, sessionsMarkReviewed, sessionsUnmarkReviewed } from "./ipc/review";
import { refreshPaged, usePagedList, type PageRequest, type PagedList } from "./paged";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const ENDED = ["finished", "stopped", "handed_over"];
/** Ended sessions older than this are left to History (AJ-E1007). */
export const TO_REVIEW_WINDOW_MS = 7 * 24 * 3_600_000;

/** Ended this week (not handed over) and not yet marked reviewed or archived. */
export function toReview(session: InstanceInfo, now: number): boolean {
  if (!ENDED.includes(session.status)) return false;
  // A handed-over session goes on elsewhere; its successor is the one to
  // pick up (as Triage had it).
  if (session.status === "handed_over") return false;
  if (session.lifecycle === "reviewed" || session.lifecycle === "archived") return false;
  return now - (session.last_activity_ms ?? session.started_at_ms) <= TO_REVIEW_WINDOW_MS;
}

/** The backend's list of them, in the window up to now. */
export function reviewQuery(now: number): SessionsQuery {
  return sessionsQuery({ which: "to_review", since_ms: now - TO_REVIEW_WINDOW_MS });
}

/** A short stamp of what the working set says about its sessions: the
 * list and its count are read again when it changes (a session ended,
 * was marked reviewed or archived). */
function stampOf(sessions: readonly InstanceInfo[]): string {
  let hash = 5381;
  for (const session of sessions) {
    for (const ch of `${session.id}:${session.status}:${session.lifecycle ?? ""},`) hash = ((hash << 5) + hash + ch.charCodeAt(0)) | 0;
  }
  return `${sessions.length}:${hash >>> 0}`;
}

/** The paged list's key (one list: the backend's, read again on changes). */
const PAGED_KEY = "home-review";

/** The last count read, by stamp: Home opened again says it at once. */
let remembered: { stamp: string; total: number } | null = null;

/** Forgets the count read (tests). */
export function forgetReviewCount(): void {
  remembered = null;
}

export interface ReviewList {
  /** The working set's, newest first: shown at once. */
  rows: InstanceInfo[];
  /** How many there are: the backend's count once read (the working
   * set's in the preview); `null` until read. */
  total: number | null;
  /** Every one, a page at a time, once asked for (`null` in the preview
   * and until wanted). */
  paged: PagedList<InstanceInfo> | null;
  /** Marks every one reviewed: how many, and the Undo that puts that
   * batch back. */
  markAll: () => Promise<{ count: number; undo: () => Promise<unknown> }>;
}

/** Every page's rows that are sessions (the list holds only AI Juggler's). */
async function sessionPage(request: PageRequest) {
  const page = await sessionsPage(reviewQuery(Date.now()), request);
  return { ...page, items: page.items.flatMap((row) => (row.session ? [row.session] : [])) };
}

/** Home's list to review; `showAll`: every one is wanted, paged. */
export function useReviewList(sessions: InstanceInfo[], now: number, showAll: boolean): ReviewList {
  const live = hasLiveBackend();
  const rows = useMemo(
    () => sessions.filter((s) => toReview(s, now)).sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms)),
    [sessions, now],
  );
  const stamp = useMemo(() => stampOf(sessions), [sessions]);
  const [read, setRead] = useState<{ stamp: string; total: number } | null>(remembered);
  useEffect(() => {
    if (!live) return undefined;
    let alive = true;
    // The sessions changed (`stamp`): the count, and the pages shown, again.
    refreshPaged(PAGED_KEY);
    void sessionsCounts(reviewQuery(Date.now())).then(
      (counts) => {
        remembered = { stamp, total: counts.all };
        if (alive) setRead(remembered);
      },
      () => null,
    );
    return () => {
      alive = false;
    };
  }, [live, stamp]);
  const total = live ? (read?.stamp === stamp ? read.total : null) : rows.length;
  // Paged once every one is wanted, or when the app holds none of them;
  // read again (its rows kept meanwhile) when the sessions change.
  const wanted = live && (showAll || (rows.length === 0 && (total ?? 0) > 0));
  const paged = usePagedList<InstanceInfo>(wanted ? PAGED_KEY : null, sessionPage, { keyOf: (session) => session.id });
  const markAll = async () => {
    if (live) {
      const done = await sessionsMarkReviewed(reviewQuery(Date.now()));
      return { count: done.count, undo: () => sessionsUnmarkReviewed(done.at_ms) };
    }
    const ids = rows.map((session) => session.id);
    await instancesMarkReviewed(ids);
    return { count: ids.length, undo: () => instancesMarkReviewed(ids, true) };
  };
  return { rows, total, paged: wanted ? paged : null, markAll };
}
