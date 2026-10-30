/**
 * ADR-0023 — the local cache: what AI Juggler keeps on this computer to be
 * quick (subagent lists, app icons, the search index), its size and
 * limit, the switch and Clear cache. The preview keeps its state in
 * `__AIJ_CACHE__` (a sample until a test seeds one) and records each
 * call in `__AIJ_CACHE_CALLS__`.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { CacheInfo } from "../../types/generated/CacheInfo";
import type { CacheRowInfo } from "../../types/generated/CacheRowInfo";

export type { CacheInfo, CacheRowInfo };

/** The fixed limits Settings offers, in megabytes (Automatic is the default). */
export const CACHE_LIMITS_MB = [250, 1000, 4000] as const;

type Call = { on?: boolean; limit_mb?: number | null; clear?: true };
type Preview = { __AIJ_CACHE__?: CacheInfo; __AIJ_CACHE_CALLS__?: Call[] };
const preview = () => window as unknown as Preview;

/** Decimal, as Finder and the backend count them. */
const MB = 1_000_000;

/** The preview's cache: a busy Mac with room, before anything is seeded. */
export const SAMPLE_CACHE: CacheInfo = {
  on: true,
  automatic: true,
  limit_mb: null,
  budget_bytes: 1_300 * MB,
  used_bytes: 47 * MB,
  low_disk: false,
  free_bytes: 65_500 * MB,
  caches: [
    {
      key: "subagents",
      name: "Subagent lists",
      speeds_up: "A session's Subagents tab opens at once, even with hundreds of subagents.",
      slower: "Each session's Subagents tab reads its subagents' files again the first time it opens.",
      bytes: 45 * MB,
      entries: 126,
      whole_only: false,
      on: true,
      own_switch: null,
      own_switch_pane: null,
      own_switch_section: null,
    },
    {
      key: "icons",
      name: "App icons",
      speeds_up: "The apps' icons show at once, without asking the system for them again.",
      slower: "Each app's icon is read from the app again the first time it shows.",
      bytes: 2 * MB,
      entries: 38,
      whole_only: false,
      on: true,
      own_switch: null,
      own_switch_pane: null,
      own_switch_section: null,
    },
    {
      key: "search-index",
      name: "Search index",
      speeds_up: "Search the words of every session answers at once.",
      slower: "The index is built again in the background; until it's done, search finds less.",
      bytes: 0,
      entries: 0,
      whole_only: true,
      on: false,
      own_switch: "Search the words of every session",
      own_switch_pane: "Privacy",
      own_switch_section: "search",
    },
  ],
};

function current(): CacheInfo {
  return preview().__AIJ_CACHE__ ?? SAMPLE_CACHE;
}

function settle(next: CacheInfo, call: Call): CacheInfo {
  (preview().__AIJ_CACHE_CALLS__ ??= []).push(call);
  const used = next.caches.reduce((sum, row) => sum + row.bytes, 0);
  const settled = { ...next, used_bytes: used };
  preview().__AIJ_CACHE__ = settled;
  return settled;
}

/** Where the cache stands. */
export async function cacheStatus(): Promise<CacheInfo> {
  if (!hasLiveBackend()) return current();
  return invoke<CacheInfo>("cache_status");
}

/** Turns it on, or off (its files go; the search index keeps its own switch). */
export async function cacheSetOn(on: boolean): Promise<CacheInfo> {
  if (!hasLiveBackend()) {
    const now = current();
    const caches = now.caches.map((row) => (row.own_switch ? row : on ? { ...row, on: true } : { ...row, on: false, bytes: 0, entries: 0 }));
    return settle({ ...now, on, caches }, { on });
  }
  return invoke<CacheInfo>("cache_set_on", { on });
}

/** Sets the limit: `null` for automatic, else megabytes. */
export async function cacheSetLimit(limitMb: number | null): Promise<CacheInfo> {
  if (!hasLiveBackend()) {
    const now = current();
    const budget = limitMb === null ? SAMPLE_CACHE.budget_bytes : limitMb * MB;
    return settle(
      {
        ...now,
        automatic: limitMb === null,
        limit_mb: limitMb,
        budget_bytes: now.low_disk ? now.budget_bytes : budget,
      },
      { limit_mb: limitMb },
    );
  }
  return invoke<CacheInfo>("cache_set_limit", { limitMb });
}

/** Clears every cache now; each fills again as it's used. */
export async function cacheClear(): Promise<CacheInfo> {
  if (!hasLiveBackend()) {
    const now = current();
    return settle(
      {
        ...now,
        caches: now.caches.map((row) => ({ ...row, bytes: 0, entries: 0 })),
      },
      { clear: true },
    );
  }
  return invoke<CacheInfo>("cache_clear");
}
