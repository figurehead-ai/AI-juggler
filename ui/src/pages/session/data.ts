/**
 * AJ-E2108 — what a session's page reads besides the session: every
 * account's runway and the profiles' names. Read once per half minute at
 * most and shared by every session page (a page that opens, a tab that
 * comes back, a live refresh never asks again while it's fresh).
 */
import { useEffect, useState } from "react";
import { accountsList, quotaDisplayAll, type AccountInfo, type QuotaDisplayData } from "../../lib/bridge";

const FRESH_MS = 30_000;

type Cached<T> = { at: number; value: Promise<T> };

function cachedRead<T>(read: () => Promise<T>, fallback: T) {
  let cache: Cached<T> | null = null;
  return {
    load(now = Date.now()): Promise<T> {
      if (cache && now - cache.at < FRESH_MS) return cache.value;
      const value = read().catch(() => fallback);
      cache = { at: now, value };
      return value;
    },
    /** Tests: forget what was read. */
    reset() {
      cache = null;
    },
  };
}

export const runways = cachedRead<QuotaDisplayData[]>(quotaDisplayAll, []);
export const profiles = cachedRead<AccountInfo[]>(accountsList, []);

function useCached<T>(source: { load: () => Promise<T> }, initial: T): T {
  const [value, setValue] = useState<T>(initial);
  useEffect(() => {
    let alive = true;
    void source.load().then((next) => alive && setValue(next));
    return () => {
      alive = false;
    };
  }, [source]);
  return value;
}

const NO_RUNWAYS: QuotaDisplayData[] = [];
const NO_PROFILES: AccountInfo[] = [];

/** Every account's runway (shared, fresh for half a minute). */
export function useRunways(): QuotaDisplayData[] {
  return useCached(runways, NO_RUNWAYS);
}

/** Every profile (shared, fresh for half a minute). */
export function useProfiles(): AccountInfo[] {
  return useCached(profiles, NO_PROFILES);
}
