/**
 * D-118 — what Home remembers per viewer (this data folder's own storage,
 * every read and write guarded: without storage nothing is remembered and
 * Home still draws): the feed's filters (R7), each section's fold (R3 of
 * AJ-E3701), what has been read (R9), and the feed's first page (R8, the
 * local cache ruling: it paints at once on the next open).
 *
 * Seen and read are apart (R9): *seen* is the away boundary, the
 * backend's `summary_seen_at_ms`, moved by "Mark all seen" or after Home
 * has been in front 10 s with the digest on screen; *read* is per item,
 * set when it is opened or acted on, and clears its unread dot.
 */
import { useCallback, useEffect, useState } from "react";
import { keep, readKept } from "./local-store";
import type { FeedFiltersInfo, FeedItemInfo } from "./ipc/feed";
import type { Page } from "./paged";

const FILTERS = "aij.home.feed.filters";
const READ = "aij.home.feed.read";
const HORIZON = "aij.home.feed.horizon";
const FIRST = "aij.home.feed.first";
const FOLD = "aij.home.fold.";

/** Read ids kept (the newest; older ones fall behind the horizon). */
const READ_KEPT = 500;

/** What `key` keeps, parsed (`null`: nothing, or not JSON). */
function parsed(key: string): unknown {
  const text = readKept(key);
  if (text === null) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function record(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? Object.fromEntries(Object.entries(value)) : null;
}

const text = (value: unknown): string | null => (typeof value === "string" && value !== "" ? value : null);

/** The filters a viewer left the feed on (none: All). */
export function keptFilters(): FeedFiltersInfo {
  const kept = record(parsed(FILTERS));
  if (!kept) return { include_mine: false };
  const folders = Array.isArray(kept.folders) ? kept.folders.filter((one): one is string => typeof one === "string") : null;
  return { category: text(kept.category), project: text(kept.project), pool: text(kept.pool), folders: folders && folders.length > 0 ? folders : null, include_mine: kept.include_mine === true };
}

/** The feeds drawn now, told when the filters are set from elsewhere
 * (Home's counts line: "6 finished today" shows the Finished chip). */
const shown = new Set<(next: FeedFiltersInfo) => void>();

/** The feed's filters, remembered per viewer. */
export function useFeedFilters(): [FeedFiltersInfo, (next: FeedFiltersInfo) => void] {
  const [filters, setFilters] = useState(keptFilters);
  useEffect(() => {
    shown.add(setFilters);
    return () => void shown.delete(setFilters);
  }, []);
  const set = useCallback((next: FeedFiltersInfo) => {
    setFilters(next);
    keep(FILTERS, JSON.stringify(next));
  }, []);
  return [filters, set];
}

/** Shows one chip of the feed (`category`, as the chips name it), its
 * other filters kept, and remembers it as a click on the chip would. */
export function showInFeed(category: string): void {
  const next = { ...keptFilters(), category };
  keep(FILTERS, JSON.stringify(next));
  for (const set of shown) set(next);
}

/** Whether `filters` narrow the feed (the "Filtered: … ×" line). */
export function filtered(filters: FeedFiltersInfo): boolean {
  return Boolean((filters.category && filters.category !== "all") || filters.project || filters.pool || filters.folders);
}

/** A section's fold (open by default), remembered per viewer. */
export function useFold(id: string, initial = true): [boolean, () => void] {
  const [open, setOpen] = useState(() => {
    const kept = readKept(FOLD + id);
    return kept === null ? initial : kept === "open";
  });
  const toggle = useCallback(() => {
    setOpen((was) => {
      keep(FOLD + id, was ? "closed" : "open");
      return !was;
    });
  }, [id]);
  return [open, toggle];
}

/** Items before this time are read (set on the feed's first open, so a
 * first look doesn't dot 30 days of history; "Mark all read" moves it). */
export function readHorizon(now: number): number {
  const kept = Number(readKept(HORIZON));
  if (Number.isFinite(kept) && kept > 0) return kept;
  keep(HORIZON, String(now));
  return now;
}

/** Everything up to `now` is read. */
export function markAllRead(now: number): void {
  keep(HORIZON, String(now));
  keep(READ, "[]");
}

/** The ids read after the horizon. */
export function readIds(): Set<string> {
  const kept = parsed(READ);
  return new Set(Array.isArray(kept) ? kept.filter((one): one is string => typeof one === "string") : []);
}

/** `ids` are read now (opened or acted on). */
export function markRead(ids: readonly string[]): void {
  const now = [...readIds()];
  for (const id of ids) if (!now.includes(id)) now.push(id);
  keep(READ, JSON.stringify(now.slice(-READ_KEPT)));
}

/** Whether an item shows its unread dot. */
export function unread(item: Pick<FeedItemInfo, "id" | "at_ms">, read: ReadonlySet<string>, horizon: number): boolean {
  return item.at_ms > horizon && !read.has(item.id);
}

/** The first page last read for `filters` (R8: it paints at once). */
export function cachedFirst(filters: string): Page<FeedItemInfo> | null {
  const kept = record(parsed(FIRST));
  const page = record(kept?.page);
  if (!kept || kept.filters !== filters || !page || !Array.isArray(page.items)) return null;
  return { items: page.items.filter(isItem), next_cursor: null, total: null };
}

/** A kept item, as far as the feed reads one (it was the backend's). */
function isItem(value: unknown): value is FeedItemInfo {
  const item = record(value);
  return item !== null && typeof item.id === "string" && typeof item.at_ms === "number" && typeof item.kind === "string" && record(item.facts) !== null && record(item.target) !== null && record(item.author) !== null;
}

/** Keeps the first page read for `filters`, and when it was read. */
export function keepFirst(filters: string, page: Page<FeedItemInfo>, at: number = Date.now()): void {
  keep(FIRST, JSON.stringify({ filters, page, at }));
}

/** When the kept first page for `filters` was read (R8's "Showing what
 * was loaded at …"), `null` without one. */
export function cachedAt(filters: string): number | null {
  const kept = record(parsed(FIRST));
  return kept?.filters === filters && typeof kept.at === "number" ? kept.at : null;
}

/** Test seam: forget what the feed and Home's folds keep. */
export function forgetFeedState(): void {
  try {
    for (const key of Object.keys(localStorage)) if (key.startsWith("aij.home.feed.") || key.startsWith(FOLD)) localStorage.removeItem(key);
  } catch {
    // No storage: nothing kept.
  }
}
