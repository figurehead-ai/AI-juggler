/**
 * AJ-E2107 — the seen mark AI Juggler keeps: when the user last opened or
 * went to each session another app runs, so "Done, review it" clears once
 * they've looked. Kept in AI Juggler's own settings (one small record, the
 * newest marks only), never in the app's files.
 */
import { useSyncExternalStore } from "react";
import { appSettingGet, appSettingSet } from "./bridge";

const KEY = "conversation_seen";
/** The most marks kept (the oldest go first). */
const KEEP = 300;

let marks: Record<string, number> = {};
let loaded = false;
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of listeners) listener();
}

/** A stored record, checked: `{key: epoch ms}` only. */
export function parseSeen(raw: string | null): Record<string, number> {
  if (!raw) return {};
  try {
    const value = JSON.parse(raw) as unknown;
    if (typeof value !== "object" || value === null || Array.isArray(value)) return {};
    const out: Record<string, number> = {};
    for (const [key, at] of Object.entries(value as Record<string, unknown>)) {
      if (typeof at === "number" && Number.isFinite(at) && key.length < 400) out[key] = at;
    }
    return out;
  } catch {
    return {};
  }
}

function load(): void {
  if (loaded) return;
  loaded = true;
  void appSettingGet(KEY).then(
    (raw) => {
      marks = { ...parseSeen(raw), ...marks };
      emit();
    },
    () => undefined,
  );
}

/** The marks: `c:<account>/<conversation>` → when it was last looked at. */
export function useSeen(): Record<string, number> {
  load();
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => marks,
    () => marks,
  );
}

/** The user opened or went to it now. */
export function markSeen(accountId: string, conversationId: string, at = Date.now()): void {
  load();
  const next = { ...marks, [`c:${accountId}/${conversationId}`]: at };
  const keys = Object.keys(next);
  if (keys.length > KEEP) {
    for (const key of keys.sort((a, b) => next[a]! - next[b]!).slice(0, keys.length - KEEP)) delete next[key];
  }
  marks = next;
  emit();
  void appSettingSet(KEY, JSON.stringify(marks)).catch(() => undefined);
}

/** Tests: start empty. */
export function resetSeen(): void {
  marks = {};
  loaded = false;
  emit();
}
