/**
 * A long list, a page at a time, drawing only the rows in view (0.1.5
 * review: hundreds of rows drawn at once). Fed by `usePagedList`:
 * - a skeleton in the very first render while nothing is read yet;
 * - only the rows in (and just around) the scrolled view are in the
 *   page, between two spacers that keep the scrollbar honest; heights
 *   start at `rowHeight` and are measured as rows are drawn;
 * - the next page is read when the list's end scrolls near, and a Load
 *   more button does the same by keyboard (or where scrolling can't);
 * - ↑/↓, Home and End move between rows, drawing the one reached.
 * It scrolls with whatever scrolls around it (the page), not in a box of
 * its own, unless `maxHeight` gives it one.
 */
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type JSX, type KeyboardEvent, type ReactNode } from "react";
import type { PagedList as Paged } from "../lib/paged";

/** Rows drawn beyond each edge of the view. */
const OVERSCAN = 6;
/** How near the end (px) the next page is read. */
const AHEAD = 600;

export type PagedListProps<T> = {
  list: Paged<T>;
  /** A row's id, unique in the list. */
  keyOf: (row: T) => string;
  /** One row's content (its `<li>` is the list's). */
  renderRow: (row: T, index: number) => ReactNode;
  /** A row's height before it's measured (px). */
  rowHeight?: number;
  /** What the list is, for screen readers. */
  label: string;
  /** Said when the list has no rows. */
  empty: ReactNode;
  /** Skeleton rows while the first page is read. */
  skeletonRows?: number;
  /** Scroll in a box of this height instead of with the page (px, or a
   * CSS length such as "min(30rem, 50vh)"). */
  maxHeight?: number | string;
  className?: string;
  testId?: string;
};

/** The nearest ancestor that scrolls (or the document's). */
function scrollParent(node: HTMLElement | null): HTMLElement | null {
  for (let at = node?.parentElement ?? null; at; at = at.parentElement) {
    const { overflowY } = getComputedStyle(at);
    if (overflowY === "auto" || overflowY === "scroll") return at;
  }
  return null;
}

/** The first index whose row ends below `y` (offsets are prefix sums). */
function indexAt(offsets: number[], y: number): number {
  let lo = 0;
  let hi = offsets.length - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (offsets[mid + 1]! <= y) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

export function PagedList<T>({
  list,
  keyOf,
  renderRow,
  rowHeight = 44,
  label,
  empty,
  skeletonRows = 6,
  maxHeight,
  className = "",
  testId = "paged-list",
}: PagedListProps<T>): JSX.Element {
  const { items } = list;
  const box = useRef<HTMLDivElement | null>(null);
  const listRef = useRef<HTMLUListElement | null>(null);
  const heights = useRef(new Map<string, number>());
  const [view, setView] = useState({ top: 0, height: typeof window === "undefined" ? 800 : window.innerHeight });
  const [measured, setMeasured] = useState(0);
  const focusNext = useRef<number | null>(null);

  // Each row's top, from measured heights (estimated until drawn).
  const offsets = useMemo(() => {
    const out = new Array<number>(items.length + 1);
    out[0] = 0;
    for (let i = 0; i < items.length; i++) out[i + 1] = out[i]! + (heights.current.get(keyOf(items[i]!)) ?? rowHeight);
    return out;
    // `measured` bumps when a height changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items, rowHeight, keyOf, measured]);
  const totalHeight = offsets[items.length] ?? 0;

  const hasRows = items.length > 0;
  // Where the view is over the list: the list's top relative to its
  // scroller's view, and that view's height.
  const look = useCallback(() => {
    const ul = listRef.current;
    if (!ul) return;
    const own = maxHeight !== undefined ? box.current : null;
    const scroller = own ?? scrollParent(ul);
    const top = ul.getBoundingClientRect().top - (scroller ? scroller.getBoundingClientRect().top : 0);
    const height = (scroller ? scroller.clientHeight : window.innerHeight) || window.innerHeight;
    setView((now) => (now.top === top && now.height === height ? now : { top, height }));
  }, [maxHeight]);

  useLayoutEffect(() => {
    look();
    const ul = listRef.current;
    const scroller = (maxHeight !== undefined ? box.current : null) ?? scrollParent(ul);
    const target: HTMLElement | Window = scroller ?? window;
    target.addEventListener("scroll", look, { passive: true });
    window.addEventListener("resize", look);
    return () => {
      target.removeEventListener("scroll", look);
      window.removeEventListener("resize", look);
    };
  }, [look, maxHeight, hasRows]);

  // The rows in view, and a few around them.
  const from = Math.max(0, indexAt(offsets, Math.max(0, -view.top)) - OVERSCAN);
  const to = Math.min(items.length, indexAt(offsets, Math.max(0, -view.top + view.height)) + 1 + OVERSCAN);

  // Measure what was drawn; a changed height redraws the spacers.
  useLayoutEffect(() => {
    const ul = listRef.current;
    if (!ul) return;
    let changed = false;
    ul.querySelectorAll<HTMLLIElement>("li[data-row-key]").forEach((li) => {
      const key = li.dataset.rowKey!;
      const height = li.offsetHeight;
      if (height > 0 && heights.current.get(key) !== height) {
        heights.current.set(key, height);
        changed = true;
      }
    });
    if (changed) setMeasured((n) => n + 1);
    if (focusNext.current !== null) {
      ul.querySelector<HTMLElement>(`li[data-index="${focusNext.current}"] button, li[data-index="${focusNext.current}"] a, li[data-index="${focusNext.current}"] [tabindex]`)?.focus();
      focusNext.current = null;
    }
  });

  // The next page, when the end of what's read comes near.
  const { hasMore, loadingMore, loadMore } = list;
  useEffect(() => {
    if (hasMore && !loadingMore && totalHeight + view.top - view.height < AHEAD) loadMore();
  }, [hasMore, loadingMore, loadMore, totalHeight, view.top, view.height]);

  // ↑/↓/Home/End: to the next row, drawn first if it isn't.
  const onKeyDown = (event: KeyboardEvent<HTMLUListElement>) => {
    const li = (event.target as HTMLElement).closest<HTMLLIElement>("li[data-index]");
    if (!li) return;
    const at = Number(li.dataset.index);
    const next = event.key === "ArrowDown" ? at + 1 : event.key === "ArrowUp" ? at - 1 : event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : null;
    if (next === null || next < 0 || next >= items.length || next === at) return;
    event.preventDefault();
    const ul = listRef.current;
    const scroller = (maxHeight !== undefined ? box.current : null) ?? scrollParent(ul);
    if (next < from || next >= to) {
      // Bring it into view; it's focused once drawn.
      const rowTop = offsets[next]!;
      if (scroller) scroller.scrollTop += rowTop + view.top - view.height / 3;
      else window.scrollBy(0, rowTop + view.top - view.height / 3);
      setView((now) => ({ ...now, top: now.height / 3 - rowTop }));
    }
    focusNext.current = next;
    setMeasured((n) => n + 1);
  };

  if (list.loading) {
    return (
      <ul aria-label={label} aria-busy="true" className={"overflow-hidden rounded-xl border border-hairline bg-surface " + className} data-testid={`${testId}-skeleton`}>
        {Array.from({ length: skeletonRows }, (_, i) => (
          <li key={i} aria-hidden="true" className="flex items-center gap-3 border-t border-hairline/60 px-4 first:border-t-0" style={{ height: rowHeight }}>
            <span className="size-2 shrink-0 animate-pulse rounded-full bg-surface-2" />
            <span className="h-3 flex-1 animate-pulse rounded bg-surface-2" style={{ maxWidth: `${60 + ((i * 17) % 30)}%` }} />
            <span className="h-3 w-14 shrink-0 animate-pulse rounded bg-surface-2" />
          </li>
        ))}
        <li className="sr-only" role="status">
          Loading…
        </li>
      </ul>
    );
  }
  if (items.length === 0) {
    return (
      <div className={className} data-testid={`${testId}-empty`}>
        {list.error ? <p className="text-xs text-warning" role="alert">This list couldn't be read. </p> : empty}
      </div>
    );
  }
  const shownRows = items.slice(from, to);
  const total = list.total ?? items.length;
  return (
    <div ref={box} className={className} style={maxHeight !== undefined ? { maxHeight, overflowY: "auto" } : undefined} data-testid={testId} data-stale={list.stale ? "true" : undefined}>
      <ul ref={listRef} aria-label={label} aria-busy={list.stale || loadingMore ? "true" : undefined} className="overflow-hidden rounded-xl border border-hairline bg-surface" onKeyDown={onKeyDown} style={{ paddingTop: offsets[from], paddingBottom: totalHeight - offsets[to]! }}>
        {shownRows.map((row, i) => {
          const index = from + i;
          const key = keyOf(row);
          return (
            <li key={key} data-row-key={key} data-index={index} aria-setsize={total} aria-posinset={index + 1} className="border-t border-hairline/60 first:border-t-0">
              {renderRow(row, index)}
            </li>
          );
        })}
      </ul>
      {hasMore && (
        <button type="button" onClick={loadMore} disabled={loadingMore} className="mt-2 self-start text-xs text-accent-hi hover:underline disabled:text-muted" data-testid={`${testId}-more`}>
          {loadingMore ? "Loading more…" : list.total !== null ? `Load more (${list.total - items.length} more)` : "Load more"}
        </button>
      )}
      {list.error !== null && items.length > 0 && (
        <p className="mt-1 text-xs text-warning" role="alert">
          The rest couldn't be read. <button type="button" className="underline" onClick={list.refresh}>Try again</button>
        </p>
      )}
    </div>
  );
}
