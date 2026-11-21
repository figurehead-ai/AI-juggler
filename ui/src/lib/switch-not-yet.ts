/**
 * The rc33 M4 walk: a session with no conversation yet showed "Continue
 * this session in another app…" as its out-of-room notice's primary and
 * "Continue on another account…" in its header, both live, while its
 * switch list greyed every app and the dialog refused ("This session
 * hasn't started a conversation yet…"). Why the session itself can't
 * switch yet is the backend's one answer (`switch_targets`' `not_yet`,
 * the same words its rows give); every switch list that asks keeps it
 * here, by the session it asked for, and the page's other doors grey
 * with it, so the page tells one story.
 */
import { useSyncExternalStore } from "react";
import { listeners } from "./listeners";

/** SK-2 (the 0.4.6 soak, A5): why one of AI Juggler's sessions can't
 * switch yet, before any switch list has asked: it has no conversation
 * to carry. The one rule for every door that offers its switch (its menu,
 * ⌘⇧H, an Account's "What this affects", Home's and the bell's "Continue
 * in another app…"), so one never offers what another greys. `null`:
 * it can. */
export function cantSwitchYet(row: { conversation_id: string | null }): string | null {
  return row.conversation_id ? null : "It has no conversation to carry over yet";
}

/** By the switch source key (AI Juggler's session id, or another app's
 * session's key): why it can't switch yet, or null once it can. */
const notYet = new Map<string, string | null>();
const { subscribe, notify } = listeners();

/** Keeps what a switch list was told for `sessionId`. */
export function noteSwitchTargets(sessionId: string, rows: ReadonlyArray<{ not_yet?: string | null }>): void {
  const why = rows.find((row) => row.not_yet)?.not_yet ?? null;
  if (notYet.has(sessionId) && notYet.get(sessionId) === why) return;
  notYet.set(sessionId, why);
  notify();
}

/** Why `sessionId` can't switch yet, as its switch list last heard it;
 * null when it can, or before its list has asked. */
export function useSwitchNotYet(sessionId: string | null | undefined): string | null {
  const read = () => (sessionId ? (notYet.get(sessionId) ?? null) : null);
  return useSyncExternalStore(subscribe, read, read);
}

/** Tests: forget every answer. */
export function resetSwitchNotYet(): void {
  notYet.clear();
  notify();
}
