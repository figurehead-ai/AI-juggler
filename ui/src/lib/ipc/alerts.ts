/**
 * Passive alerts, the ranking and the reset queue.
 */
import { call, liveBackend as hasBackend } from "./transport";

/** AJ-E611: one ranked (or filtered) account row. */
export interface RankedRowData {
  account: string;
  /** The account's name (rows never show the id). */
  account_label: string;
  score: number | null;
  filtered_reason: string | null;
  top_factors: string[];
  factors: [string, number][];
  headroom_unknown: boolean;
  confidence: string;
  tte_min: number | null;
  suggested: boolean;
  /** AJ-E306 Req.7: the launch admission verdict (SPEC §5.2) —
   * `ok|tight|no`; undefined when headroom or burn is unknown
   * (renders "headroom unknown", never a fabricated number). Warns,
   * never blocks. */
  admission: "ok" | "tight" | "no" | null;
}

/** AJ-E611: the explainable ranking — suggest-only, the user clicks. */
export async function rankingRows(project?: string): Promise<RankedRowData[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_RANKING__?: RankedRowData[] })
      .__AIJ_RANKING__ ?? [];
  }
  return call("ranking_rows", { project });
}

/** AJ-E611: one emitted alert (the notification region's payload).
 * `digest` lines are the hourly batch — visually quieter than window
 * alerts (a renewal item never reads as "switch now"). */
export interface AlertEventData {
  kind: "tray-amber" | "actionable" | "exhausted" | "reset-reached" | "digest";
  account: string;
  text: string;
  at_ms: number;
  /** A quota alert's numbers, as data (the notification model words them). */
  quota?: AlertQuotaData;
}

/** The numbers behind a quota alert (`features::quota::alert_words`). */
export interface AlertQuotaData {
  account_label: string;
  /** `five_hour` | `weekly`: data, never shown. */
  window: string;
  left_pct: number | null;
  minutes_left: number | null;
  resets_at_ms: number | null;
}

/** AJ-E611: one alert evaluation pass (at-most-once by the machines).
 * The webview's wall-clock local hour rides along — the quiet-hours
 * gate for the digest flush (the backend carries no tz database). */
export async function alertsTick(): Promise<AlertEventData[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_ALERTS__?: AlertEventData[] })
      .__AIJ_ALERTS__ ?? [];
  }
  return call("alerts_tick", { localHour: new Date().getHours() });
}

/** AJ-E611: one reset-queue row. */
export interface ResetQueueRow {
  id: string;
  session: string;
  account: string;
  resume_at_ms: number;
  state: string;
}

/** AJ-E611: the queue rows (countdown / ready / stale explained). */
export async function resetQueueList(): Promise<ResetQueueRow[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_RESET_QUEUE__?: ResetQueueRow[] })
      .__AIJ_RESET_QUEUE__ ?? [];
  }
  return call("reset_queue_list");
}

/** AJ-E611: queue a resume at the reset (passive timer; the resume
 * itself is always an explicit click). */
export async function resetQueueQueue(
  session: string,
  account: string,
  resumeAtMs: number,
): Promise<void> {
  if (!hasBackend) return;
  await call("reset_queue_queue", { session, account, resumeAtMs });
}

/** AJ-E611: dismiss a queued resume. */
export async function resetQueueDismiss(id: string): Promise<void> {
  if (!hasBackend) return;
  await call("reset_queue_dismiss", { id });
}
