/**
 * AJ-E1552: why the last read of the pools failed (`lib/pools` keeps the
 * last answer on a failure), so a Pools screen never read says so with
 * Retry instead of a skeleton for ever. A new read clears it.
 */
import { useSyncExternalStore } from "react";

let failure: unknown = undefined;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The pools' read failed (`cause`), or a new one began (`undefined`). */
export function setPoolsFailure(cause: unknown): void {
  if (cause === failure) return;
  failure = cause;
  for (const listener of listeners) listener();
}

/** Why the last read of the pools failed, or `undefined`. */
export function usePoolsFailure(): unknown {
  return useSyncExternalStore(subscribe, () => failure);
}
