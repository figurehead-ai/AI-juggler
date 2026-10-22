/**
 * The OS access cards' logic — pure, so first run's Computer access step
 * (and release/0.1's OsAccessRows) only wire it up.
 *
 * The 0.0.12 report: Screen Recording allowed in System Settings, the card
 * never turned to Allowed. macOS reports that permission to a process only
 * once; AI Juggler sees a new answer after it reopens. So:
 * - not asked → Allow (raises the system prompt);
 * - asked, back without a grant → the note "Allowed it in System
 *   Settings? macOS needs AI Juggler to reopen to see it", Reopen AI
 *   Juggler, and System Settings beside it;
 * - granted → Allowed — only when the OS says so;
 * - the step checks again whenever the window is back (focus, visible),
 *   not only on its timer.
 */
import { appRelaunch, type OsAccessInfo, type ReopenBlockInfo } from "./bridge";

export type AccessAction = "request" | "settings" | "reopen";

export interface AccessCard {
  /** `allowed` only when the OS reports it. */
  state: "allowed" | "not_asked" | "not_allowed" | "reopen";
  /** The words under the card, and whether they lead (the reopen note). */
  note: string | null;
  noteLeads: boolean;
  /** The main button; null once allowed. */
  primary: { label: string; action: AccessAction } | null;
  /** System Settings beside Reopen (in case it wasn't allowed). */
  secondary: { label: string; action: AccessAction } | null;
}

/** The card for one entry, from what the backend reports. */
export function accessCard(entry: OsAccessInfo): AccessCard {
  if (entry.status === "allowed") {
    return { state: "allowed", note: null, noteLeads: false, primary: null, secondary: null };
  }
  if (entry.action === "reopen") {
    return {
      state: "reopen",
      note: entry.note,
      noteLeads: true,
      primary: { label: "Reopen AI Juggler", action: "reopen" },
      secondary: { label: "Open System Settings", action: "settings" },
    };
  }
  const settings = entry.action === "settings";
  return {
    state: settings ? "not_allowed" : "not_asked",
    note: entry.note,
    noteLeads: false,
    primary: { label: settings ? "Open System Settings" : "Allow", action: settings ? "settings" : "request" },
    secondary: null,
  };
}

/** Checks again every `everyMs` and whenever the window is back (the
 * user returns from System Settings). Returns the cleanup. */
export function watchAccess(check: () => void, everyMs = 1500): () => void {
  const timer = window.setInterval(check, everyMs);
  const onVisible = () => {
    if (document.visibilityState === "visible") check();
  };
  window.addEventListener("focus", check);
  document.addEventListener("visibilitychange", onVisible);
  return () => {
    window.clearInterval(timer);
    window.removeEventListener("focus", check);
    document.removeEventListener("visibilitychange", onVisible);
  };
}

/** After Reopen AI Juggler, first run comes back at its access step (a
 * one-time marker, read once at start). */
export const RESUME_ACCESS_KEY = "aij.firstRun.resumeAccess";

/** True once if the last run left first run to reopen the app. */
export function takeResumeAccess(): boolean {
  try {
    const resume = localStorage.getItem(RESUME_ACCESS_KEY) === "1";
    localStorage.removeItem(RESUME_ACCESS_KEY);
    return resume;
  } catch {
    return false;
  }
}

function markResumeAccess(on: boolean): void {
  try {
    if (on) localStorage.setItem(RESUME_ACCESS_KEY, "1");
    else localStorage.removeItem(RESUME_ACCESS_KEY);
  } catch {
    // Without storage the reopen still works; first run starts over.
  }
}

/** Reopen AI Juggler. Resolves with why it has to be confirmed (live
 * sessions: `can_reopen_anyway`) or wait; when nothing stands in the way
 * the app relaunches, and first run will resume at the access step. */
export async function reopenApp(anyway: boolean): Promise<ReopenBlockInfo | null> {
  markResumeAccess(true);
  try {
    const block = await appRelaunch(anyway);
    if (block) markResumeAccess(false);
    return block;
  } catch (cause) {
    markResumeAccess(false);
    throw cause;
  }
}
