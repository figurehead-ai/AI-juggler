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
import { AGE_CELL, COLUMNS, NameCell, NumberCells, OWNER_CELL, StateChip } from "./cells";

/** Every row's height, px (two lines of text). */
export const ROW_HEIGHT = 48;

/** The rows under a folded row taken out: an app's sessions and
 * programs, a session's processes, a process's children (0.3.2: the app
 * and session rows folded only through the backend, and an app's never). */
export function unfolded(rows: readonly ProcessGraphRowInfo[], folded: ReadonlySet<string>): ProcessGraphRowInfo[] {
  const out: ProcessGraphRowInfo[] = [];
  let hideBelow: number | null = null;
  for (const row of rows) {
    if (hideBelow !== null) {
      if (row.depth > hideBelow) continue;
      hideBelow = null;
    }
    out.push(row);
    if (folded.has(row.key)) hideBelow = row.depth;
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
  ended = false,
  testId = "process-tree",
  onSubagent,
  select = null,
  onSelected,
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
  /** The Ended view: Age reads as how long each ran. */
  ended?: boolean;
  testId?: string;
  /** AJ-E1520 (0.3): opens the subagent that probably started a row. */
  onSubagent?: (id: string) => void;
  /** A row to select and scroll to once it is shown (a subagent's
   * process), and when it was. */
  select?: string | null;
  onSelected?: () => void;
}): JSX.Element {
  // Open or folded by a click here, at once, whatever the backend says
  // next (a session's choice is also kept by the view, `onToggle`).
  const [chosen, setChosen] = useState<ReadonlyMap<string, boolean>>(() => new Map());
  const open = useCallback(
    (row: ProcessGraphRowInfo) => chosen.get(row.key) ?? (row.kind === "session" ? isOpen(row) : true),
    [chosen, isOpen],
  );
  const folded = useMemo(() => new Set(rows.filter((row) => !open(row)).map((row) => row.key)), [rows, open]);
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

  // A row asked for (a subagent's process): selected once it's here.
  useEffect(() => {
    if (!select) return;
    const index = shown.findIndex((row) => row.key === select);
    if (index < 0) return;
    setFocus(index);
    const element = scroller.current;
    if (element) element.scrollTop = Math.max(0, index * ROW_HEIGHT - element.clientHeight / 2);
    onSelected?.();
  }, [select, shown, onSelected]);

  const toggle = useCallback(
    (row: ProcessGraphRowInfo, next: boolean) => {
      setChosen((now) => new Map(now).set(row.key, next));
      if (row.kind === "session") onToggle(row, next);
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
    <div className="@container flex min-h-0 flex-col overflow-hidden rounded-card border border-hairline bg-surface" data-testid={testId}>
      <div role="treegrid" aria-label={label} aria-rowcount={total + 1} className="flex min-h-0 flex-col" onKeyDown={onKey}>
        <div role="row" aria-rowindex={1} className={`grid ${COLUMNS} items-center border-b border-hairline bg-surface-2 px-3 py-2 text-meta font-medium text-muted`}>
          <div role="columnheader">Process</div>
          <div role="columnheader">State</div>
          <div role="columnheader" className="pl-2">CPU</div>
          <div role="columnheader" className="text-right">{ended ? "Peak" : "Memory"}</div>
          <div role="columnheader" className="pl-2">Ports</div>
          <div role="columnheader" className={`${AGE_CELL} text-right`}>{ended ? "Ran for" : "Age"}</div>
          <div role="columnheader" className={OWNER_CELL}>
            Owner
          </div>
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
                    className={`absolute inset-x-0 grid ${COLUMNS} items-center border-b border-hairline/60 px-3 outline-none focus-visible:bg-accent-soft/40 focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent/60 ${
                      row.kind === "app" ? "bg-surface-2/60" : "hover:bg-surface-2/50"
                    }`}
                    style={{ top: index * ROW_HEIGHT, height: ROW_HEIGHT }}
                  >
                    <NameCell row={row} open={rowOpen} onToggle={() => toggle(row, !rowOpen)} onSubagent={onSubagent} />
                    {row.kind !== "app" && (
                      <div role="gridcell" className="min-w-0">
                        {row.kind !== "program" && <StateChip word={row.state} session={row.kind === "session"} />}
                      </div>
                    )}
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
