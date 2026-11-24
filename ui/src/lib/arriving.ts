/**
 * 0.5.1 F74: a session a switch just made, until the session list holds
 * it. The page opens on it at once (the switch selects it), so for that
 * moment it says what happened and where the conversation is now
 * ("Starting on Personal…"), never "Session · Unavailable". Kept by the
 * new session's id; a later switch to the same id replaces it.
 */
import { useSyncExternalStore } from "react";

export interface Arrival {
  /** Where it continues, as the switch named it ("Personal"). */
  target: string;
  /** The original's title, when the list had it. */
  from: string | null;
  /** When the switch finished (ms). */
  at: number;
}

const arrivals = new Map<string, Arrival>();
const listeners = new Set<() => void>();
const tell = () => listeners.forEach((listener) => listener());

/** A switch made session `id`, now on `target`. */
export function expectArrival(id: string, target: string, from: string | null = null, at: number = Date.now()): void {
  arrivals.set(id, { target, from, at });
  tell();
}

/** The arrival of session `id`, while one is expected. */
function arrivalOf(id: string | null): Arrival | null {
  return id === null ? null : (arrivals.get(id) ?? null);
}

/** Tests: forget every arrival. */
export function resetArrivals(): void {
  arrivals.clear();
  tell();
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => void listeners.delete(listener);
};

/** The arrival of session `id`, re-read when one is expected. */
export function useArrival(id: string | null): Arrival | null {
  return useSyncExternalStore(
    subscribe,
    () => arrivalOf(id),
    () => arrivalOf(id),
  );
}
