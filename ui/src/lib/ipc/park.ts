/**
 * AJ-E1003/E1004/E1005: park until reset, the morning summary and
 * morning triage.
 */
import { invoke, liveBackend as hasBackend } from "./transport";
import type { OvernightSummaryInfo } from "../../types/generated/OvernightSummaryInfo";
import type { ParkedInfo } from "../../types/generated/ParkedInfo";
import type { ParkedOfferInfo } from "../../types/generated/ParkedOfferInfo";
import { previewHooks } from "../preview-hooks";


type ParkSeams = {
  __AIJ_PARKED_OFFERS__?: ParkedOfferInfo[];
  __AIJ_PARK_CALLS__?: string[];
  __AIJ_PARKED_CHECK__?: ParkedInfo;
};
const parkSeams = () => window as unknown as ParkSeams;
const parkCall = (call: string) => {
  const seams = parkSeams();
  seams.__AIJ_PARK_CALLS__ = [...(seams.__AIJ_PARK_CALLS__ ?? []), call];
};

/** Parks a quota-blocked Instance until its window resets (pack staged). */
export async function instancePark(sessionId: string): Promise<ParkedInfo> {
  if (!hasBackend) {
    parkCall(`park:${sessionId}`);
    return {
      session_id: sessionId,
      account_label: "Work Max",
      parked_at_ms: Date.now(),
      resets_at_ms: Date.now() + 2 * 3_600_000,
      confidence: "measured",
      due: false,
      pack_dir: "",
    };
  }
  return invoke<ParkedInfo>("instance_park", { sessionId });
}

/** The resume offers due now (polled with the alerts — never a timer of
 * its own). */
export async function parkedOffers(): Promise<ParkedOfferInfo[]> {
  if (!hasBackend) return [...(parkSeams().__AIJ_PARKED_OFFERS__ ?? [])];
  return invoke<ParkedOfferInfo[]>("parked_offers");
}

/** "Not now": snoozes once, then stays parked quietly. */
export async function parkedDismiss(sessionId: string): Promise<void> {
  if (!hasBackend) {
    parkCall(`dismiss:${sessionId}`);
    const seams = parkSeams();
    seams.__AIJ_PARKED_OFFERS__ = (seams.__AIJ_PARKED_OFFERS__ ?? []).filter((row) => row.session_id !== sessionId);
    return;
  }
  return invoke<void>("parked_dismiss", { sessionId });
}

/** Re-reads an Estimated reset ("check now"). */
export async function parkedCheckNow(sessionId: string): Promise<ParkedInfo> {
  if (!hasBackend) {
    parkCall(`check:${sessionId}`);
    const seeded = parkSeams().__AIJ_PARKED_CHECK__;
    if (seeded) return seeded;
    throw new Error("still estimated");
  }
  return invoke<ParkedInfo>("parked_check_now", { sessionId });
}

/** Resumes a parked Instance — the original session, on its Account. */
export async function parkedResume(sessionId: string): Promise<{ session_id: string; pid: number } | null> {
  if (!hasBackend) {
    parkCall(`resume:${sessionId}`);
    return null;
  }
  return invoke<{ session_id: string; pid: number }>("parked_resume", { sessionId });
}


/** What happened since the user last looked (severity-ordered). */
export async function overnightSummary(): Promise<OvernightSummaryInfo> {
  if (!hasBackend) {
    const seeded = (window as unknown as { __AIJ_SUMMARY__?: OvernightSummaryInfo }).__AIJ_SUMMARY__;
    return seeded ?? { since_ms: Date.now(), generated_ms: Date.now(), rows: [], counts: {}, leftovers: 0 };
  }
  return invoke<OvernightSummaryInfo>("overnight_summary", { sinceMs: null });
}

/** The user looked: the next summary starts from now. */
export async function overnightSummarySeen(): Promise<void> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_SUMMARY__?: OvernightSummaryInfo; __AIJ_SUMMARY_SEEN__?: number };
    w.__AIJ_SUMMARY_SEEN__ = (w.__AIJ_SUMMARY_SEEN__ ?? 0) + 1;
    delete w.__AIJ_SUMMARY__;
    return;
  }
  return invoke<void>("overnight_summary_seen");
}


/** Marks an ended Instance reviewed: it leaves the triage's finished
 * list, and the step is kept in its history. */
export async function instanceMarkReviewed(sessionId: string): Promise<number> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_TRIAGE_CALLS__?: string[] };
    w.__AIJ_TRIAGE_CALLS__ = [...(w.__AIJ_TRIAGE_CALLS__ ?? []), `reviewed:${sessionId}`];
    const row = previewHooks().fixtures.MOCK_SESSIONS.find((session) => session.id === sessionId);
    const at = Date.now();
    if (row) {
      row.lifecycle = "reviewed";
      row.lifecycle_at_ms = at;
    }
    return at;
  }
  return invoke<number>("instance_mark_reviewed", { sessionId });
}

/** AJ-E1503: undoes Mark reviewed (back on the finished list; both
 * steps stay in its history). */
export async function instanceUnmarkReviewed(sessionId: string): Promise<number> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_TRIAGE_CALLS__?: string[] };
    w.__AIJ_TRIAGE_CALLS__ = [...(w.__AIJ_TRIAGE_CALLS__ ?? []), `unreviewed:${sessionId}`];
    const row = previewHooks().fixtures.MOCK_SESSIONS.find((session) => session.id === sessionId);
    const at = Date.now();
    if (row) {
      row.lifecycle = "unreviewed";
      row.lifecycle_at_ms = at;
    }
    return at;
  }
  return invoke<number>("instance_unmark_reviewed", { sessionId });
}
