/**
 * 0.1.5 — show the last answer at once, then refresh it
 * (stale-while-revalidate). A page's frame draws on the first render; a
 * summary it has seen before fills it straight from memory, and the fresh
 * one replaces it when it comes. While a new range loads, the old picture
 * stays (dimmed by the caller), never a blank.
 */
import { useCallback, useEffect, useRef, useState } from "react";

type Entry = { at: number; data: unknown };

const memory = new Map<string, Entry>();
/** At most this many answers are kept (each is a few kilobytes). */
const KEEP = 60;

function remember(key: string, data: unknown): void {
  memory.delete(key);
  memory.set(key, { at: Date.now(), data });
  while (memory.size > KEEP) memory.delete(memory.keys().next().value!);
}

/** The answer kept for `key`, if any. */
export function cachedValue<T>(key: string): T | null {
  return (memory.get(key)?.data as T | undefined) ?? null;
}

/** Test seam: forget every kept answer. */
export function clearCached(): void {
  memory.clear();
}

export type Cached<T> = {
  /** The newest answer for this key, or the last key's while this one loads. */
  data: T | null;
  /** `data` is for this key. */
  current: boolean;
  /** A read is under way. */
  loading: boolean;
  /** The last read failed, in words (the old answer stays). */
  error: string | null;
  refresh: () => void;
};

/** Reads `load()` for `key` (null: nothing to read), keeping and showing
 * the last answers. A fresh answer younger than `freshMs` isn't read again. */
export function useCached<T>(key: string | null, load: () => Promise<T>, freshMs = 15_000): Cached<T> {
  const [state, setState] = useState<{ key: string | null; data: T | null; loading: boolean; error: string | null }>(() => ({
    key,
    data: key ? cachedValue<T>(key) : null,
    loading: key !== null,
    error: null,
  }));
  const loader = useRef(load);
  loader.current = load;
  const [nonce, setNonce] = useState(0);
  const forced = useRef(false);

  useEffect(() => {
    if (key === null) return;
    let alive = true;
    const kept = memory.get(key);
    setState((prev) => ({ key, data: (kept?.data as T | undefined) ?? prev.data, loading: true, error: null }));
    if (kept && !forced.current && Date.now() - kept.at < freshMs) {
      setState({ key, data: kept.data as T, loading: false, error: null });
      return;
    }
    forced.current = false;
    loader.current().then(
      (data) => {
        remember(key, data);
        if (alive) setState({ key, data, loading: false, error: null });
      },
      (error: unknown) => {
        if (alive) setState((prev) => ({ ...prev, loading: false, error: error instanceof Error ? error.message : String(error) }));
      },
    );
    return () => {
      alive = false;
    };
  }, [key, nonce, freshMs]);

  const refresh = useCallback(() => {
    forced.current = true;
    setNonce((n) => n + 1);
  }, []);
  const current = state.key === key && (key === null || memory.has(key) || !state.loading);
  return { data: state.data, current, loading: state.loading, error: state.error, refresh };
}
