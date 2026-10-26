/**
 * AJ-E2404 — the rows a Processes view shows, read a page at a time and
 * kept live:
 * - a view opened again shows its last rows at once (kept in memory, this
 *   window only) and reads them again behind them;
 * - the next page is read when the list's end comes near (`loadMore`);
 * - when the backend says the graph changed (`processes://changed`), the
 *   pages already shown are read again, one refresh at a time, so a busy
 *   Mac never queues them up.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import {
  onProcessesChanged,
  processesPage,
  type ProcessCountsInfo,
  type ProcessGraphRowInfo,
  type ProcessQueryInput,
} from "../../lib/ipc/processes";

/** Rows a page asks for (the backend's most). */
export const PAGE_ROWS = 200;

type Kept = { rows: ProcessGraphRowInfo[]; total: number; counts: ProcessCountsInfo | null; next: string | null };

/** The last rows of each view, by its query (in memory only). */
const kept = new Map<string, Kept>();
const KEPT_VIEWS = 12;

/** Tests: forget what was kept. */
export function forgetProcessRows(): void {
  kept.clear();
}

function keyOf(query: ProcessQueryInput): string {
  return JSON.stringify({ ...query, cursor: null, limit: null });
}

export interface ProcessRows {
  rows: ProcessGraphRowInfo[];
  total: number;
  counts: ProcessCountsInfo | null;
  /** The first read hasn't answered yet (and nothing was kept). */
  loading: boolean;
  error: string | null;
  hasMore: boolean;
  loadMore: () => void;
  refresh: () => void;
}

export function useProcessRows(query: ProcessQueryInput): ProcessRows {
  const key = keyOf(query);
  const [state, setState] = useState<Kept | null>(() => kept.get(key) ?? null);
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);
  const again = useRef(false);
  const latest = useRef({ key, query, state });
  latest.current = { key, query, state };

  /** Reads pages from the start until `want` rows (at least one page). */
  const read = useCallback(async (want: number) => {
    if (busy.current) {
      again.current = true;
      return;
    }
    busy.current = true;
    const { key: readKey, query: readQuery } = latest.current;
    try {
      const rows: ProcessGraphRowInfo[] = [];
      let cursor: string | null = null;
      let total = 0;
      let counts: ProcessCountsInfo | null = null;
      do {
        const page = await processesPage({ ...readQuery, cursor, limit: PAGE_ROWS });
        rows.push(...page.items);
        cursor = page.next_cursor;
        total = page.total;
        counts = page.counts;
      } while (cursor !== null && rows.length < want);
      const next: Kept = { rows, total, counts, next: cursor };
      kept.delete(readKey);
      kept.set(readKey, next);
      while (kept.size > KEPT_VIEWS) kept.delete(kept.keys().next().value as string);
      if (latest.current.key === readKey) {
        setState(next);
        setError(null);
      }
    } catch (failure) {
      if (latest.current.key === readKey) setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      busy.current = false;
      if (again.current) {
        again.current = false;
        void read(Math.max(PAGE_ROWS, latest.current.state?.rows.length ?? 0));
      }
    }
  }, []);

  useEffect(() => {
    setState(kept.get(key) ?? null);
    void read(PAGE_ROWS);
  }, [key, read]);

  useEffect(
    () => onProcessesChanged(() => void read(Math.max(PAGE_ROWS, latest.current.state?.rows.length ?? 0))),
    [read],
  );

  const loadMore = useCallback(() => {
    const current = latest.current.state;
    if (!current?.next) return;
    void read(current.rows.length + PAGE_ROWS);
  }, [read]);

  return {
    rows: state?.rows ?? [],
    total: state?.total ?? 0,
    counts: state?.counts ?? null,
    loading: state === null && error === null,
    error,
    hasMore: Boolean(state?.next),
    loadMore,
    refresh: () => void read(Math.max(PAGE_ROWS, latest.current.state?.rows.length ?? 0)),
  };
}
