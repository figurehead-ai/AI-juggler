/**
 * D-118 (AJ-E3702 R6, R8) — the feed's stream as Home shows it: pages
 * read a page at a time (`usePagedList`, keyed by the filters), the first
 * page painted at once from the last visit's (the local cache ruling),
 * and the reader's place kept:
 *
 * - **New items never move the reader.** Scrolled down, new items wait
 *   behind an "N new updates" pill (X's pattern); at the top they show at
 *   once, each fading in once over 600 ms (none under Reduce Motion).
 * - **One polite announcement** for what arrived while Home is open, at
 *   most one per 30 s ("3 new updates"); never one per item.
 * - **What was loaded stays** when a read fails: the error says when it
 *   was loaded, with Retry.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { usePagedList, type PagedList } from "./paged";
import { cachedAt, cachedFirst, keepFirst } from "./feed-state";
import { FEED_PAGE, feedPage, type FeedFiltersInfo, type FeedItemInfo } from "./ipc/feed";
import type { PageRequest } from "./paged";

/** At most one announcement of new items per this long. */
const ANNOUNCE_EVERY_MS = 30_000;

export interface FeedStream {
  list: PagedList<FeedItemInfo>;
  /** The items shown: the new ones held back while scrolled down. */
  shown: FeedItemInfo[];
  /** New items waiting behind the pill. */
  waiting: number;
  /** Shows them, and goes to the top. */
  showNew: () => void;
  /** Whether an item arrived while Home was open (it fades in once). */
  fresh: (item: FeedItemInfo) => boolean;
  /** The polite announcement, when there is one. */
  announced: string;
  /** When what's shown was read (the error line says it). */
  loadedAt: number | null;
  /** Pages read so far. */
  pages: number;
}

/** The items to show and how many wait: from the item that was on top
 * when the reader scrolled down (`heldTop`), or everything at the top. Pure. */
export function heldBack(items: readonly FeedItemInfo[], heldTop: string | null): { shown: FeedItemInfo[]; waiting: number } {
  if (heldTop === null) return { shown: [...items], waiting: 0 };
  const at = items.findIndex((item) => item.id === heldTop);
  return at <= 0 ? { shown: [...items], waiting: 0 } : { shown: items.slice(at), waiting: at };
}

/** The announcement for `count` new items. */
export function newWords(count: number): string {
  return count === 0 ? "" : `${count} new ${count === 1 ? "update" : "updates"}`;
}

/** `scroller` is the box the feed scrolls in (its column, or Home's). */
export function useFeedStream(filters: FeedFiltersInfo, scroller: () => HTMLElement | null): FeedStream {
  const key = `home-feed:${JSON.stringify(filters)}`;
  const fetch = useCallback((request: PageRequest) => feedPage(request, filters), [filters]);
  const list = usePagedList(key, fetch, { pageSize: FEED_PAGE, keyOf: (item) => item.id, seed: () => cachedFirst(key) });
  const { items } = list;
  // The top item when the reader scrolled down; `null` at the top.
  const [heldTop, setHeldTop] = useState<string | null>(null);
  const [opened] = useState(() => Date.now());
  const [announced, setAnnounced] = useState("");
  const top = useRef<string | null>(null);
  const lastSaid = useRef(0);
  useEffect(() => {
    top.current = items[0]?.id ?? null;
  }, [items]);

  // Where the reader is: at the top (everything shows), or scrolled down
  // (what was on top then holds the place).
  useEffect(() => {
    const box = scroller();
    if (!box) return undefined;
    const onScroll = () => setHeldTop((held) => (box.scrollTop <= 4 ? null : (held ?? top.current)));
    box.addEventListener("scroll", onScroll, { passive: true });
    return () => box.removeEventListener("scroll", onScroll);
  }, [scroller]);

  const { shown, waiting } = useMemo(() => heldBack(items, heldTop), [items, heldTop]);
  const arrived = items.filter((item) => item.at_ms > opened).length;

  // Said once, batched: at most one announcement per 30 s.
  useEffect(() => {
    if (arrived === 0) return undefined;
    const wait = Math.max(0, lastSaid.current + ANNOUNCE_EVERY_MS - Date.now());
    const timer = window.setTimeout(() => {
      lastSaid.current = Date.now();
      setAnnounced(newWords(arrived));
    }, wait);
    return () => window.clearTimeout(timer);
  }, [arrived]);

  // The first page, kept for the next visit's first paint (its cursor
  // isn't kept: the live read that replaces it pages on).
  useEffect(() => {
    if (list.stale || list.error !== null || list.loading) return;
    keepFirst(key, { items: items.slice(0, FEED_PAGE), next_cursor: null, total: null });
  }, [items, list.stale, list.error, list.loading, key]);

  const showNew = useCallback(() => {
    setHeldTop(null);
    scroller()?.scrollTo?.({ top: 0 });
  }, [scroller]);
  const fresh = useCallback((item: FeedItemInfo) => item.at_ms > opened, [opened]);
  return { list, shown, waiting, showNew, fresh, announced, loadedAt: cachedAt(key), pages: Math.max(1, Math.ceil(items.length / FEED_PAGE)) };
}
