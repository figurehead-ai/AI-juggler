/**
 * AJ-E2504 (§3, §8) — whether AI Juggler's own service answers. The
 * window's refresh says so after each try: a failure (with the app's
 * service live) starts "showing the last state AI Juggler knew", a
 * success ends it. Data is never hidden; actions that would start
 * something say why they wait. The preview's seam `__AIJ_OFFLINE__`
 * (`{ since_ms }`) stands in for a service that stopped answering.
 */
import { useSyncExternalStore } from "react";
import { hasLiveBackend } from "./bridge";

export interface BackendHealth {
  /** Since when it hasn't answered (epoch ms), or null while it does. */
  downSince: number | null;
}

let health: BackendHealth = { downSince: null };
const listeners = new Set<() => void>();

function set(next: BackendHealth): void {
  if (next.downSince === health.downSince) return;
  health = next;
  for (const listener of listeners) listener();
}

/** The refresh answered. */
export function backendAnswered(): void {
  set({ downSince: null });
}

/** The refresh failed: with the app's own service live, it stopped
 * answering (a preview without every mock is not that). */
export function backendFailed(at: number = Date.now()): void {
  if (!hasLiveBackend()) return;
  if (health.downSince === null) set({ downSince: at });
}

let seamCache: BackendHealth | null = null;

/** The preview's stand-in (one object per value: a stable snapshot). */
function seam(): BackendHealth | null {
  const forced = (window as unknown as { __AIJ_OFFLINE__?: { since_ms: number } }).__AIJ_OFFLINE__;
  if (!forced) return null;
  if (seamCache?.downSince !== forced.since_ms) seamCache = { downSince: forced.since_ms };
  return seamCache;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

const read = () => seam() ?? health;

export function useBackendHealth(): BackendHealth {
  return useSyncExternalStore(subscribe, read, read);
}

/** Test seam. */
export function resetBackendHealth(): void {
  health = { downSince: null };
  for (const listener of listeners) listener();
}
