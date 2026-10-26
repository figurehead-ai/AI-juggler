/**
 * AJ-E2404 — the Processes tree table: App → Session → process →
 * subprocess, one row each, under a header that stays put.
 *
 * - A treegrid (WAI-ARIA): one row takes Tab, the arrows move, → opens or
 *   goes in, ← closes or goes up, Home and End, Enter runs the row's
 *   first action; each row says its level and whether it is open.
 * - Windowed: rows are one height, and only those in view (at most 60)
 *   are in the page, however many processes there are; the next page is
 *   read as the end comes near.
 * - Folding: a session folds through the query (the backend leaves its
 *   processes out); a process with children folds here.
 */
import { useCallback, useEffect, useMemo, useRef, useState, type JSX, type KeyboardEvent, type ReactNode } from "react";
import type { ProcessGraphRowInfo } from "../../lib/ipc/processes";
import { treeKey, windowRows } from "../../lib/tree-grid";
import { COLUMNS, NameCell, NumberCells, StateChip } from "./cells";

/** Every row's height, px (two lines of text). */
export const ROW_HEIGHT = 48;

/** The rows with the processes under a folded process taken out. */
export function unfolded(rows: readonly ProcessGraphRowInfo[], folded: ReadonlySet<string>): ProcessGraphRowInfo[] {
  const out: ProcessGraphRowInfo[] = [];
  let hideBelow: number | null = null;
  for (const row of rows) {
    if (hideBelow !== null) {
      if (row.kind === "process" && row.depth > hideBelow) continue;
      hideBelow = null;
    }
    out.push(row);
    if (row.kind === "process" && row.has_children && folded.has(row.key)) hideBelow = row.depth;
  }
  return out;
}

export function ProcessTree({
  rows,
  total,
  label,
  isOpen,
  onToggle,
  actions,
  onActivate,
  hasMore,
  onNearEnd,
  height = "calc(100vh - 19rem)",
  empty,
  testId = "process-tree",
}: {
  rows: readonly ProcessGraphRowInfo[];
  /** Every row the view holds (for the reader: "row 12 of 340"). */
  total: number;
  /** What the table is ("Processes that need you"). */
  label: string;
  /** Whether a session row is open (its processes listed). */
  isOpen: (row: ProcessGraphRowInfo) => boolean;
  onToggle: (row: ProcessGraphRowInfo, open: boolean) => void;
  /** A row's buttons and menu. */
  actions: (row: ProcessGraphRowInfo) => ReactNode;
  /** Enter on a row. */
  onActivate?: (row: ProcessGraphRowInfo) => void;
  hasMore: boolean;
  onNearEnd: () => void;
  height?: string;
  empty?: ReactNode;
  testId?: string;
}): JSX.Element {
  const [folded, setFolded] = useState<Set<string>>(() => new Set());
  const shown = useMemo(() => unfolded(rows, folded), [rows, folded]);
  const scroller = useRef<HTMLDivElement | null>(null);
  const [scroll, setScroll] = useState({ top: 0, height: 800 });
  const [focus, setFocus] = useState(0);
  const at = Math.min(focus, Math.max(0, shown.length - 1));
  const [start, end] = windowRows(shown.length, ROW_HEIGHT, scroll.top, scroll.height);

  useEffect(() => {
    const element = scroller.current;
    if (!element) return;
    const measure = () => setScroll({ top: element.scrollTop, height: element.clientHeight || 800 });
    measure();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    observer?.observe(element);
    return () => observer?.disconnect();
  }, []);

  useEffect(() => {
    if (hasMore && end >= shown.length - 20) onNearEnd();
  }, [end, shown.length, hasMore, onNearEnd]);

  const open = useCallback(
    (row: ProcessGraphRowInfo) => (row.kind === "process" ? !folded.has(row.key) : row.kind === "session" ? isOpen(row) : true),
    [folded, isOpen],
  );
  const toggle = useCallback(
    (row: ProcessGraphRowInfo, next: boolean) => {
      if (row.kind === "process") {
        setFolded((now) => {
          const copy = new Set(now);
          if (next) copy.delete(row.key);
          else copy.add(row.key);
          return copy;
        });
      } else onToggle(row, next);
    },
    [onToggle],
  );

  const focusRow = (index: number) => {
    setFocus(index);
    const element = scroller.current;
    if (!element) return;
    const top = index * ROW_HEIGHT;
    if (top < element.scrollTop) element.scrollTop = top;
    else if (top + ROW_HEIGHT > element.scrollTop + element.clientHeight) element.scrollTop = top + ROW_HEIGHT - element.clientHeight;
    requestAnimationFrame(() => element.querySelector<HTMLElement>(`[data-row-index="${index}"]`)?.focus());
  };

  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target instanceof HTMLElement && event.target.closest("button, a, input") && event.target.getAttribute("role") !== "row") return;
    if (event.key === "Enter") {
      const row = shown[at];
      if (row && onActivate) {
        event.preventDefault();
        onActivate(row);
      }
      return;
    }
    const move = treeKey(shown, at, event.key, open);
    if (!move) return;
    event.preventDefault();
    if ("to" in move) focusRow(move.to);
    else toggle(shown[move.at]!, move.toggle === "open");
  };

  return (
    <div className="flex min-h-0 flex-col overflow-hidden rounded-card border border-hairline bg-surface" data-testid={testId}>
      <div role="treegrid" aria-label={label} aria-rowcount={total + 1} className="flex min-h-0 flex-col" onKeyDown={onKey}>
        <div role="row" aria-rowindex={1} className={`grid ${COLUMNS} items-center gap-3 border-b border-hairline bg-surface-2 px-3 py-2 text-meta font-medium text-muted`}>
          <div role="columnheader">Process</div>
          <div role="columnheader">State</div>
          <div role="columnheader" className="pl-2">CPU</div>
          <div role="columnheader" className="text-right">Memory</div>
          <div role="columnheader">Ports</div>
          <div role="columnheader" className="text-right">Age</div>
          <div role="columnheader" className="hidden min-[1440px]:block">Owner</div>
          <div role="columnheader">
            <span className="sr-only">Actions</span>
          </div>
        </div>
        <div
          ref={scroller}
          className="relative min-h-[12rem] overflow-y-auto"
          style={{ maxHeight: height }}
          onScroll={(event) => setScroll({ top: event.currentTarget.scrollTop, height: event.currentTarget.clientHeight })}
          data-testid={`${testId}-scroll`}
        >
          {shown.length === 0 ? (
            <div className="px-4 py-10 text-center text-body text-muted">{empty}</div>
          ) : (
            <div style={{ height: shown.length * ROW_HEIGHT }} className="relative">
              {shown.slice(start, end).map((row, offset) => {
                const index = start + offset;
                const rowOpen = open(row);
                return (
                  <div
                    key={row.key}
                    role="row"
                    aria-rowindex={index + 2}
                    aria-level={row.depth + 1}
                    aria-expanded={row.has_children ? rowOpen : undefined}
                    aria-selected={index === at}
                    tabIndex={index === at ? 0 : -1}
                    data-row-index={index}
                    data-testid={`process-row-${row.key}`}
                    data-kind={row.kind}
                    onFocus={() => setFocus(index)}
                    onClick={() => setFocus(index)}
                    className={`absolute inset-x-0 grid ${COLUMNS} items-center gap-3 border-b border-hairline/60 px-3 outline-none focus-visible:bg-accent-soft/40 focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent/60 ${
                      row.kind === "app" ? "bg-surface-2/60" : "hover:bg-surface-2/50"
                    }`}
                    style={{ top: index * ROW_HEIGHT, height: ROW_HEIGHT }}
                  >
                    <NameCell row={row} open={rowOpen} onToggle={() => toggle(row, !rowOpen)} />
                    <div role="gridcell" className="min-w-0">
                      {row.kind !== "app" && <StateChip word={row.state} session={row.kind === "session"} />}
                    </div>
                    <NumberCells row={row} />
                    <div role="gridcell" className="flex items-center justify-end gap-1">
                      {actions(row)}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
