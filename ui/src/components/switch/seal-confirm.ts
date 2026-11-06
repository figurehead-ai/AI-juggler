/**
 * D-62 (AJ-E3009): a switch that enters or leaves a sealed pool waits for
 * the person's tick. The pool lines (ConfirmBody's SwitchPoolLines) say
 * what's at stake and hold the tick; the switch button reads whether one
 * is still owed, through this small store, keyed by the switch.
 */
import { useSyncExternalStore } from "react";

const owed = new Map<string, boolean>();
const listeners = new Set<() => void>();

const keyOf = (sessionId: string, targetProfileId: string) => `${sessionId}>${targetProfileId}`;

/** Sets whether `sessionId`'s switch to `targetProfileId` still waits on
 * the person's tick. */
export function setSealOwed(sessionId: string, targetProfileId: string, waits: boolean): void {
  const key = keyOf(sessionId, targetProfileId);
  if ((owed.get(key) ?? false) === waits) return;
  owed.set(key, waits);
  for (const listener of listeners) listener();
}

/** Whether the switch still waits on the tick. */
export function useSealOwed(sessionId: string, targetProfileId: string): boolean {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => owed.get(keyOf(sessionId, targetProfileId)) ?? false,
  );
}
