/**
 * What every backend call shares (AJ-E2018): the event names, whether a
 * live backend answers, and how a rejection reads. The calls themselves live
 * in lib/ipc/<feature>.ts, one module per feature.
 */
import { liveBackend as hasBackend } from "./ipc/transport";

/** Event names mirrored from juggler_app constants (single source: Rust). */
export const EVENTS = {
  appReady: "app://ready",
  sessionsUpdated: "session://updated",
  /** 5 s resource tick (I-07/I-08 deltas; AJ-E401). */
  sessionsResources: "session://resources",
  /** 30 s quota tick (I-05 deltas; AJ-E401). */
  sessionsQuota: "session://quota",
  /** Global "Go to instance…" hotkey (AJ-E414). */
  goTo: "app://go-to",
  /** Live-preview frame (AJ-E415). */
  previewFrame: "preview://frame",
  /** Live-preview phase change (AJ-E415). */
  previewState: "preview://state",
  /** Cross-machine switch push progress (AJ-E706). */
  fleetSwitchProgress: "fleet://switch-progress",
  /** Backup export progress (AJ-E1009): at least every 1 %. */
  backupProgress: "backup://progress",
  /** Update status changes (AJ-E1010). */
  updateStatus: "update://status",
} as const;

export function hasLiveBackend(): boolean {
  return hasBackend;
}

/** Preview-mode refusal, worded like the backend's (code `refused`). */
export function refused(user_message: string): { code: string; retryable: boolean; user_message: string } {
  return { code: "refused", retryable: false, user_message };
}

/** AJ-E301: extracts the CURATED message from any command rejection —
 * coded errors carry `user_message`; JS-side causes fall back to
 * their string. The UI never renders a raw OS string. */
export function errorText(cause: unknown): string {
  if (cause !== null && typeof cause === "object" && "user_message" in cause) {
    const wire = cause as { user_message?: unknown };
    return typeof wire.user_message === "string" && wire.user_message.length > 0
      ? wire.user_message
      : "Something went wrong — try again.";
  }
  // A thrown Error says its message, never "Error: …" (D-25).
  if (cause instanceof Error) return cause.message.length > 0 ? cause.message : "Something went wrong — try again.";
  const text = String(cause);
  return text === "[object Object]" ? "Something went wrong — try again." : text;
}

/**
 * AJ-E308: the ready-check across login-state generations — the
 * taxonomy's `signed_in` plus the legacy `logged_in` string (scan
 * candidates still carry it). Everything else (partial / expired /
 * sso_pending / api_key / logged_out / unknown) is not ready.
 */
export function signedIn(state: string | null | undefined): boolean {
  return state === "signed_in" || state === "logged_in";
}

