/**
 * AJ-E1505 — Settings › Sessions over IPC (ENGINEERING-GUIDE U3: one thin
 * file per feature). "Stuck after" reaches the heuristics at once; a save
 * fires `SESSION_PREFS_EVENT` so open terminals follow.
 */
import { invoke } from "@tauri-apps/api/core";
import { hasLiveBackend } from "../bridge";
import type { SessionPrefsInfo } from "../../types/generated/SessionPrefsInfo";

export type { SessionPrefsInfo };

/** The defaults and ranges the backend holds to (settings_policy.rs). */
export const SESSION_PREFS_DEFAULT: SessionPrefsInfo = {
  stuck_after_minutes: 5,
  terminal_font_size: 12.5,
  terminal_scrollback_lines: 5_000,
};
export const SESSION_PREFS_RANGE = {
  stuck_after_minutes: [1, 120],
  terminal_font_size: [9, 24],
  terminal_scrollback_lines: [500, 100_000],
} as const;

/** Fired on window with the saved prefs: open terminals apply them live. */
export const SESSION_PREFS_EVENT = "aij:session-prefs";

const previewSeams = () => window as unknown as { __AIJ_SESSION_PREFS__?: SessionPrefsInfo };

/** Reads Settings › Sessions. */
export async function sessionPrefsGet(): Promise<SessionPrefsInfo> {
  if (!hasLiveBackend()) return { ...SESSION_PREFS_DEFAULT, ...previewSeams().__AIJ_SESSION_PREFS__ };
  return invoke<SessionPrefsInfo>("session_prefs_get");
}

/** Saves Settings › Sessions; open terminals follow the event. */
export async function sessionPrefsSet(prefs: SessionPrefsInfo): Promise<SessionPrefsInfo> {
  let saved: SessionPrefsInfo;
  if (!hasLiveBackend()) {
    for (const [key, [low, high]] of Object.entries(SESSION_PREFS_RANGE) as [keyof SessionPrefsInfo, readonly [number, number]][]) {
      if (prefs[key] < low || prefs[key] > high) throw `use ${low} to ${high}`;
    }
    previewSeams().__AIJ_SESSION_PREFS__ = { ...prefs };
    saved = { ...prefs };
  } else {
    saved = await invoke<SessionPrefsInfo>("session_prefs_set", { prefs });
  }
  window.dispatchEvent(new CustomEvent<SessionPrefsInfo>(SESSION_PREFS_EVENT, { detail: saved }));
  return saved;
}
