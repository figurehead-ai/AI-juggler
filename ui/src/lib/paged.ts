/**
 * Lists that can grow, read a page at a time (0.1.5 review: Usage and
 * History loaded hundreds of rows at once and stayed blank for seconds).
 *
 * `usePagedList(key, fetchPage)` reads the first page, and the next one
 * when asked (`loadMore`: the list's end scrolled into view, or its Load
 * more button). The backend's pages are keyset ones (`paging.rs`): a row
 * added meanwhile never shifts or repeats what's shown.
 *
 * What was read is kept in memory by key: a screen opened again shows its
 * rows at once and reads them again behind them (stale while it
 * revalidates). `prefetchPaged` fills that memory after first paint, so a
 * main screen's first open is instant too; `refreshPaged` reads every
 * shown list under a key prefix again (a backend event said it changed).
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { Page } from "../types/generated/Page";
import type { PageRequest } from "../types/generated/PageRequest";

export type { Page, PageRequest };

/** Rows a page asks for unless told. */
export const PAGE_SIZE = 50;
/** The backend's ceiling (`paging::MAX_LIMIT`). */
export const MAX_PAGE = 200;

export type FetchPage<T> = (request: PageRequest) => Promise<Page<T>>;

type Entry<T> = {
  items: T[];
  nextCursor: string | null;
  total: number | null;
  /** When it was read, ms. */
  at: number;
};

/** Read lists, by key (in memory, this window only). */
const memory = new Map<string, Entry<unknown>>();
/** Shown lists, by key: each one's "read again". */
const shown = new Map<string, Set<() => void>>();
/** Keeps the memory small: the lists opened most recently. */
const KEPT = 40;

function remember<T>(key: string, entry: Entry<T>): void {
  memory.delete(key);
  memory.set(key, entry as Entry<unknown>);
  while (memory.size > KEPT) memory.delete(memory.keys().next().value as string);
}

/** What's remembered for `key` (tests, and screens that peek). */
export function rememberedPage<T>(key: string): Entry<T> | undefined {
  return memory.get(key) as Entry<T> | undefined;
}

/** Forgets every remembered list (tests; a sign-out). */
export function forgetPaged(): void {
  memory.clear();
}

/** Reads `key`'s first page into memory now, if it isn't there: call it
 * after first paint for the screens opened most. */
export async function prefetchPaged<T>(key: string, fetchPage: FetchPage<T>, pageSize = PAGE_SIZE): Promise<void> {
  if (memory.has(key)) return;
  try {
    const page = await fetchPage({ cursor: null, limit: pageSize });
    if (!memory.has(key)) remember(key, { items: page.items, nextCursor: page.next_cursor, total: page.total, at: Date.now() });
  } catch {
    // A prefetch is a nicety: the screen reads it itself when opened.
  }
}

/** Reads every shown list whose key starts with `prefix` again (and
 * marks the remembered ones stale). */
export function refreshPaged(prefix: string): void {
  for (const key of [...memory.keys()]) if (key.startsWith(prefix)) (memory.get(key) as Entry<unknown>).at = 0;
  for (const [key, listeners] of shown) if (key.startsWith(prefix)) listeners.forEach((again) => again());
}

export type PagedList<T> = {
  /** Every row read so far, in order. */
  items: T[];
  /** The whole list's size, when the backend knows it. */
  total: number | null;
  /** Nothing to show yet: the first read is under way (show a skeleton). */
  loading: boolean;
  /** The rows shown are remembered ones, being read again. */
  stale: boolean;
  /** The next page is being read. */
  loadingMore: boolean;
  /** Another page follows. */
  hasMore: boolean;
  /** The last read failed (the rows shown, if any, stay). */
  error: unknown;
  loadMore: () => void;
  refresh: () => void;
};

/**
 * One list, a page at a time. `key` names the list and everything that
 * shapes it (its scope, its search): a new key starts again from its
 * remembered rows, or a skeleton. `null` reads nothing.
 */
export function usePagedList<T>(
  key: string | null,
  fetchPage: FetchPage<T>,
  options: {
    pageSize?: number;
    keyOf?: (row: T) => string;
    /** The first page when the rows are already in memory (a list the UI
     * holds): shown in the first render, then read as usual. */
    seed?: (pageSize: number) => Page<T> | null;
  } = {},
): PagedList<T> {
  const pageSize = Math.min(Math.max(options.pageSize ?? PAGE_SIZE, 1), MAX_PAGE);
  const seeded = (): Entry<T> | undefined => {
    const page = key === null ? null : (options.seed?.(pageSize) ?? null);
    return page ? { items: page.items, nextCursor: page.next_cursor, total: page.total, at: 0 } : undefined;
  };
  const remembered = (): Entry<T> | undefined => (key === null ? undefined : ((memory.get(key) as Entry<T> | undefined) ?? seeded()));
  const fresh = () => {
    const entry = remembered();
    return { key, entry: entry ?? null, stale: entry !== undefined, loadingMore: false, error: null as unknown };
  };
  const [state, setState] = useState<{ key: string | null; entry: Entry<T> | null; stale: boolean; loadingMore: boolean; error: unknown }>(fresh);
  // A new key shows its own remembered rows in this very render.
  let current = state;
  if (state.key !== key) {
    current = fresh();
    setState(current);
  }
  const fetchRef = useRef(fetchPage);
  fetchRef.current = fetchPage;
  const keyOfRef = useRef(options.keyOf);
  keyOfRef.current = options.keyOf;
  // Each read carries the generation it was started in; an answer for
  // an older one (the key changed, a refresh started) is dropped.
  const generation = useRef(0);
  const entryRef = useRef<Entry<T> | null>(current.entry);
  entryRef.current = current.entry;
  const busyMore = useRef(false);

  const readFirst = useCallback(() => {
    if (key === null) return;
    const mine = ++generation.current;
    busyMore.current = false;
    // Read again as many rows as are shown (within the backend's ceiling),
    // so a refresh doesn't shrink a list scrolled far.
    const shownRows = entryRef.current?.items.length ?? 0;
    const limit = Math.min(Math.max(pageSize, shownRows), MAX_PAGE);
    setState((now) => (now.key === key ? { ...now, stale: now.entry !== null, error: null } : now));
    fetchRef.current({ cursor: null, limit }).then(
      (page) => {
        if (generation.current !== mine) return;
        const entry: Entry<T> = { items: page.items, nextCursor: page.next_cursor, total: page.total, at: Date.now() };
        remember(key, entry);
        setState({ key, entry, stale: false, loadingMore: false, error: null });
      },
      (error: unknown) => {
        if (generation.current !== mine) return;
        setState((now) => (now.key === key ? { ...now, stale: false, error } : now));
      },
    );
  }, [key, pageSize]);

  const loadMore = useCallback(() => {
    const entry = entryRef.current;
    if (key === null || !entry || entry.nextCursor === null || busyMore.current) return;
    busyMore.current = true;
    const mine = generation.current;
    const cursor = entry.nextCursor;
    setState((now) => (now.key === key ? { ...now, loadingMore: true } : now));
    fetchRef.current({ cursor, limit: pageSize }).then(
      (page) => {
        busyMore.current = false;
        if (generation.current !== mine) return;
        const before = entryRef.current ?? entry;
        const keyOf = keyOfRef.current;
        const seen = keyOf ? new Set(before.items.map(keyOf)) : null;
        const added = seen && keyOf ? page.items.filter((row) => !seen.has(keyOf(row))) : page.items;
        const next: Entry<T> = { items: [...before.items, ...added], nextCursor: page.next_cursor, total: page.total ?? before.total, at: before.at };
        remember(key, next);
        setState((now) => (now.key === key ? { ...now, entry: next, loadingMore: false, error: null } : now));
      },
      (error: unknown) => {
        busyMore.current = false;
        if (generation.current !== mine) return;
        setState((now) => (now.key === key ? { ...now, loadingMore: false, error } : now));
      },
    );
  }, [key, pageSize]);

  // Read (again) when shown, and whenever someone says it changed.
  useEffect(() => {
    if (key === null) return;
    readFirst();
    const listeners = shown.get(key) ?? new Set<() => void>();
    listeners.add(readFirst);
    shown.set(key, listeners);
    return () => {
      generation.current++;
      listeners.delete(readFirst);
      if (listeners.size === 0) shown.delete(key);
    };
  }, [key, readFirst]);

  const entry = current.entry;
  return {
    items: entry?.items ?? [],
    total: entry?.total ?? null,
    loading: key !== null && entry === null && current.error === null,
    stale: current.stale,
    loadingMore: current.loadingMore,
    hasMore: entry !== null && entry.nextCursor !== null,
    error: current.error,
    loadMore,
    refresh: readFirst,
  };
}

/** A page of a list held whole in the UI (a preview, a test): the same
 * shape as the backend's, offset-cursored. */
export function pageOf<T>(rows: T[], request: PageRequest): Page<T> {
  const start = request.cursor ? Number(request.cursor) : 0;
  const limit = Math.min(Math.max(request.limit ?? PAGE_SIZE, 1), MAX_PAGE);
  const items = rows.slice(start, start + limit);
  const end = start + items.length;
  return { items, next_cursor: end < rows.length ? String(end) : null, total: rows.length };
}
