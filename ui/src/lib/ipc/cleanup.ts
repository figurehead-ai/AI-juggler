/**
 * Cleanup: status, tiers, pause, recovery summary, alerts and the audit
 * export.
 */
import { invoke, liveBackend as hasBackend } from "./transport";

/** AJ-E509: the audit export (JSONL text; first line = redaction note). */
export async function cleanupAuditExport(): Promise<string> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_AUDIT_EXPORT__?: string }).__AIJ_AUDIT_EXPORT__
      ?? JSON.stringify({ note: "cleanup audit export — command lines are hashed and truncated; full cmdlines are never stored" });
  }
  return invoke<string>("cleanup_audit_export");
}


/** AJ-E512: one passive Cleanup Centre alert (hog / idle nudge). */
export interface CleanupAlertData {
  kind: "hog" | "idle" | string;
  text: string;
  pid: number;
  session: string;
}

/** AJ-E512: the passive alert set (hog alerts + idle-orphan nudges). */
export interface CleanupAlertsData {
  alerts: CleanupAlertData[];
}

export async function cleanupAlerts(): Promise<CleanupAlertsData> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_CLEANUP_ALERTS__?: CleanupAlertsData })
        .__AIJ_CLEANUP_ALERTS__ ?? { alerts: [] }
    );
  }
  return invoke<CleanupAlertsData>("cleanup_alerts");
}

/** AJ-E508: the automation status (banners + pause + Tier-2 notices). */
export interface CleanupStatusData {
  paused: boolean;
  suspended: boolean;
  suspended_until_ms: number | null;
  tier2_pending: {
    pid: number;
    session: string;
    cmdline_trunc: string;
    deadline_ms: number;
  }[];
}

/** AJ-E508: per-Account tier opt-ins (two separate toggles). */
export interface CleanupTierSettingsData {
  tier1_opt_in: boolean;
  tier2_opt_in: boolean;
}

/** AJ-E510: the one-time recovery summary. */
export interface RecoverySummaryData {
  summary: string;
  adopted: number;
  left_behind: number;
  unverifiable: number;
}

/** Reads the unconsumed recovery summary, if pending (shown once). */
export async function recoverySummaryGet(): Promise<RecoverySummaryData | null> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_RECOVERY__?: RecoverySummaryData }).__AIJ_RECOVERY__
      ?? null;
  }
  return invoke<RecoverySummaryData | null>("recovery_summary_get");
}

/** Dismisses the recovery summary (the only way it goes away). */
export async function recoverySummaryDismiss(): Promise<void> {
  if (!hasBackend) {
    return Promise.resolve();
  }
  return invoke("recovery_summary_dismiss");
}

/** Reads the automation status (Tier 0's fallback — never signals). */
export async function cleanupStatus(): Promise<CleanupStatusData> {
  if (!hasBackend) {
    // Test seam: the status read fails (Settings shows the error and a retry).
    const failure = (window as unknown as { __AIJ_CLEANUP_STATUS_ERROR__?: unknown }).__AIJ_CLEANUP_STATUS_ERROR__;
    if (failure !== undefined) throw failure;
    return (
      (window as unknown as { __AIJ_CLEANUP_STATUS__?: CleanupStatusData }).__AIJ_CLEANUP_STATUS__
      ?? { paused: false, suspended: false, suspended_until_ms: null, tier2_pending: [] }
    );
  }
  return invoke<CleanupStatusData>("cleanup_status");
}

/** Sets the global `Pause automatic cleanup` kill-switch. */
export async function cleanupPauseSet(paused: boolean): Promise<CleanupStatusData> {
  if (!hasBackend) {
    return {
      paused,
      suspended: false,
      suspended_until_ms: null,
      tier2_pending: [],
    };
  }
  return invoke<CleanupStatusData>("cleanup_pause_set", { paused });
}

/** The Tier-2 notice decision: `keep` = Undo / Keep running. */
export async function cleanupTier2Decide(
  pid: number,
  keep: boolean,
): Promise<CleanupStatusData> {
  if (!hasBackend) {
    return { paused: false, suspended: false, suspended_until_ms: null, tier2_pending: [] };
  }
  return invoke<CleanupStatusData>("cleanup_tier2_decide", { pid, keep });
}

/** Re-confirms automation after the 24 h crash suspension. */
export async function cleanupResumeAfterCrash(): Promise<CleanupStatusData> {
  if (!hasBackend) {
    // Test seam: Re-confirm fails (the banner stays and says why).
    const failure = (window as unknown as { __AIJ_CLEANUP_RESUME_ERROR__?: unknown }).__AIJ_CLEANUP_RESUME_ERROR__;
    if (failure !== undefined) throw failure;
    return { paused: false, suspended: false, suspended_until_ms: null, tier2_pending: [] };
  }
  return invoke<CleanupStatusData>("cleanup_resume_after_crash");
}

/** Reads one Account's two tier opt-ins (absent rows = both off). */
export async function cleanupTiersGet(accountId: string): Promise<CleanupTierSettingsData> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_CLEANUP_TIERS__?: CleanupTierSettingsData })
        .__AIJ_CLEANUP_TIERS__ ?? { tier1_opt_in: false, tier2_opt_in: false }
    );
  }
  return invoke<CleanupTierSettingsData>("cleanup_tiers_get", { accountId });
}

/** Sets one Account's two SEPARATE toggles. */
export async function cleanupTiersSet(
  accountId: string,
  tier1OptIn: boolean,
  tier2OptIn: boolean,
): Promise<CleanupTierSettingsData> {
  if (!hasBackend) {
    return { tier1_opt_in: tier1OptIn, tier2_opt_in: tier2OptIn };
  }
  return invoke<CleanupTierSettingsData>("cleanup_tiers_set", {
    accountId,
    tier1OptIn,
    tier2OptIn,
  });
}
