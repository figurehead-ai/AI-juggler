/**
 * AJ-E1520 (0.3) — the Subagents tree table: each group (the message
 * that started them) and, under an open one, its subagents with real
 * tree lines (├ └ │) — who started whom. Columns: Subagent (its task and
 * type), State (an icon and a word), Time, Tokens (with its children's;
 * its own on hover), Model.
 *
 * The Processes tree's patterns (ProcessTree.tsx): a WAI-ARIA treegrid
 * (one row takes Tab; ↑ ↓ move, → opens or goes in, ← closes or goes up,
 * Home, End; Enter opens a subagent's page or folds a group), rows of
 * one height windowed (at most 60 in the page however many there are),
 * the next page read as the end comes near. A group folds through the
 * query (the backend leaves its rows out); a subagent with children
 * folds here.
 */
import { useCallback, useEffect, useMemo, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { Pill } from "../../components/Pill";
import { IconChevronRight } from "../../components/icons-controls";
import type { SubagentRowInfo } from "../../lib/ipc/subagents";
import { treeKey, windowRows, type TreeRowLike } from "../../lib/tree-grid";
import { dayWords, clock } from "../../lib/time-words";
import { kindWords, modelWords, statusOf, timeWords, titleOf, tokenWords, treeLines } from "./words";

export const ROW_HEIGHT = 44;

export const COLUMNS = "grid-cols-[minmax(0,1fr)_7.5rem_4rem_4.5rem_6.5rem]";

type Row = SubagentRowInfo & TreeRowLike;

/** The rows as a tree the keys walk: a group at depth 0, a subagent
 * under its parent (or its group). */
export function treeRows(rows: readonly SubagentRowInfo[], folded: ReadonlySet<string>): Row[] {
  const out: Row[] = [];
  let group: string | null = null;
  let hideBelow: number | null = null;
  rows.forEach((row, index) => {
    if (row.group) {
      group = row.key;
      hideBelow = null;
      out.push({ ...row, depth: 0, parent_key: null, has_children: row.group.count > 0 });
      return;
    }
    const agent = row.agent!;
    const depth = row.guides.length + 1;
    if (hideBelow !== null) {
      if (depth > hideBelow) return;
      hideBelow = null;
    }
    // Only the ones it started that are listed here make it a parent.
    const next = rows[index + 1];
    const parent = !!next?.agent && next.guides.length + 1 > depth;
    out.push({ ...row, depth, parent_key: agent.parent && depth > 1 ? `a:${agent.parent}` : group, has_children: parent });
    if (parent && folded.has(row.key)) hideBelow = depth;
  });
  return out;
}

export function SubagentTree({
  rows,
  label,
  now,
  onOpen,
  onGroup,
  hasMore,
  onNearEnd,
  height,
  keep,
}: {
  rows: readonly SubagentRowInfo[];
  label: string;
  now: number;
  /** A subagent's page. */
  onOpen: (row: SubagentRowInfo) => void;
  /** A group opened or folded. */
  onGroup: (key: string, open: boolean) => void;
  hasMore: boolean;
  onNearEnd: () => void;
  height: string;
  /** Where the list was scrolled (kept across a subagent's page). */
  keep: { current: number };
}): JSX.Element {
  const [folded, setFolded] = useState<Set<string>>(() => new Set());
  const shown = useMemo(() => treeRows(rows, folded), [rows, folded]);
  const scroller = useRef<HTMLDivElement | null>(null);
  const [scroll, setScroll] = useState({ top: keep.current, height: 600 });
  const [focus, setFocus] = useState(0);
  const at = Math.min(focus, Math.max(0, shown.length - 1));
  const [start, end] = windowRows(shown.length, ROW_HEIGHT, scroll.top, scroll.height);

  useEffect(() => {
    const element = scroller.current;
    if (!element) return;
    element.scrollTop = keep.current;
    const measure = () => setScroll({ top: element.scrollTop, height: element.clientHeight || 600 });
    measure();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    observer?.observe(element);
    return () => observer?.disconnect();
  }, [keep]);

  useEffect(() => {
    if (hasMore && end >= shown.length - 20) onNearEnd();
  }, [end, shown.length, hasMore, onNearEnd]);

  const isOpen = useCallback((row: Row) => (row.group ? row.group.open : !folded.has(row.key)), [folded]);
  const toggle = (row: Row, open: boolean) => {
    if (row.group) onGroup(row.group.key, open);
    else
      setFolded((now) => {
        const next = new Set(now);
        if (open) next.delete(row.key);
        else next.add(row.key);
        return next;
      });
  };
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
    const row = shown[at];
    if (!row) return;
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (row.group) toggle(row, !row.group.open);
      else onOpen(row);
      return;
    }
    const move = treeKey(shown, at, event.key, isOpen);
    if (!move) return;
    event.preventDefault();
    if ("to" in move) focusRow(move.to);
    else toggle(shown[move.at]!, move.toggle === "open");
  };

  return (
    <div className="@container flex min-h-0 flex-col overflow-hidden rounded-card border border-hairline bg-surface" data-testid="subagent-tree">
      <div role="treegrid" aria-label={label} aria-rowcount={shown.length + 1} className="flex min-h-0 flex-col" onKeyDown={onKey}>
        <div role="row" aria-rowindex={1} className={`grid ${COLUMNS} items-center gap-3 border-b border-hairline bg-surface-2 px-3 py-2 text-meta font-medium text-muted`}>
          <div role="columnheader">Subagent</div>
          <div role="columnheader">State</div>
          <div role="columnheader" className="text-right">Time</div>
          <div role="columnheader" className="text-right">Tokens</div>
          <div role="columnheader">Model</div>
        </div>
        <div
          ref={scroller}
          className="relative min-h-[12rem] overflow-y-auto"
          style={{ maxHeight: height }}
          onScroll={(event) => {
            keep.current = event.currentTarget.scrollTop;
            setScroll({ top: event.currentTarget.scrollTop, height: event.currentTarget.clientHeight });
          }}
          data-testid="subagent-tree-scroll"
        >
          <div style={{ height: shown.length * ROW_HEIGHT }} className="relative">
            {shown.slice(start, end).map((row, offset) => {
              const index = start + offset;
              const common = {
                role: "row",
                "aria-rowindex": index + 2,
                "aria-level": row.depth + 1,
                "aria-expanded": row.has_children ? isOpen(row) : undefined,
                "aria-selected": index === at,
                tabIndex: index === at ? 0 : -1,
                "data-row-index": index,
                onFocus: () => setFocus(index),
                style: { top: index * ROW_HEIGHT, height: ROW_HEIGHT },
              } as const;
              return row.group ? (
                <GroupRow key={row.key} row={row} now={now} common={common} onToggle={() => toggle(row, !row.group!.open)} />
              ) : (
                <AgentRow key={row.key} row={row} open={isOpen(row)} common={common} onOpen={() => onOpen(row)} onFold={() => toggle(row, !isOpen(row))} />
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}

type Common = Record<string, unknown> & { style: { top: number; height: number } };

const ROW = "absolute inset-x-0 grid items-center gap-3 border-b border-hairline/60 px-3 outline-none focus-visible:bg-accent-soft/40 focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent/60";

function GroupRow({ row, now, common, onToggle }: { row: Row; now: number; common: Common; onToggle: () => void }): JSX.Element {
  const group = row.group!;
  const when = group.at_ms === null ? null : dayWords(group.at_ms, now) === "Today" ? clock(group.at_ms) : dayWords(group.at_ms, now);
  const counts = [`${group.count} ${group.count === 1 ? "subagent" : "subagents"}`, group.running > 0 ? `${group.running} running` : null, group.failed > 0 ? `${group.failed} failed` : null, group.tokens ? tokenWords(group.tokens) : null].filter(Boolean).join(" · ");
  return (
    <div {...common} data-testid="subagent-group" data-open={group.open} onClick={onToggle} className={`${ROW} cursor-pointer grid-cols-[minmax(0,1fr)_auto] bg-surface-2/60 hover:bg-surface-2`}>
      <div role="gridcell" aria-colspan={4} className="flex min-w-0 items-center gap-2">
        <IconChevronRight aria-hidden="true" width={14} height={14} strokeWidth={2.2} className={`shrink-0 text-muted transition-transform motion-reduce:transition-none ${group.open ? "rotate-90" : ""}`} />
        <span className="shrink-0 text-meta font-medium text-muted">{group.asked ? "You asked:" : "Started"}</span>
        <span className="min-w-0 truncate text-body text-ink" title={group.asked ?? undefined}>
          {group.asked ? `“${group.asked}”` : "without a message on record"}
          {group.session?.title && <span className="text-muted"> · in {group.session.title}</span>}
        </span>
        {when && <span className="shrink-0 text-meta text-muted">· {when}</span>}
      </div>
      <div role="gridcell" className="text-meta tabular-nums text-muted" data-testid="subagent-group-counts">
        {counts}
      </div>
    </div>
  );
}

function AgentRow({ row, open, common, onOpen, onFold }: { row: Row; open: boolean; common: Common; onOpen: () => void; onFold: () => void }): JSX.Element {
  const agent = row.agent!;
  const status = statusOf(agent.status);
  const Icon = status.icon;
  const kind = kindWords(agent.kind);
  const own = agent.children > 0 && agent.own_tokens !== null ? `Its own: ${agent.own_tokens.toLocaleString()} · with the ${agent.children} it started: ${agent.tokens?.toLocaleString() ?? "—"}` : agent.tokens?.toLocaleString();
  return (
    <div
      {...common}
      data-testid="subagent-row"
      data-status={agent.status}
      data-depth={row.depth}
      onClick={onOpen}
      onKeyDown={undefined}
      className={`${ROW} ${COLUMNS} cursor-pointer hover:bg-surface-2/50 ${row.context ? "opacity-60" : ""}`}
    >
      <div role="gridcell" className="flex min-w-0 items-center gap-1.5">
        <span aria-hidden="true" className="shrink-0 whitespace-pre font-mono text-meta leading-none text-muted/70">
          {treeLines(row.guides, row.last)}
        </span>
        {row.has_children && (
          <button
            type="button"
            tabIndex={-1}
            aria-label={open ? "Fold the ones it started" : "Show the ones it started"}
            onClick={(event) => {
              event.stopPropagation();
              onFold();
            }}
            className="-ml-1 flex size-5 shrink-0 items-center justify-center rounded text-muted hover:bg-surface-3 hover:text-ink"
          >
            <IconChevronRight aria-hidden="true" width={12} height={12} strokeWidth={2.4} className={open ? "rotate-90" : ""} />
          </button>
        )}
        <span className="min-w-0 truncate text-body font-medium text-ink" title={agent.task ?? undefined} data-testid="subagent-task">
          {titleOf(agent)}
        </span>
        {kind && <span className="shrink-0 rounded-md bg-surface-2 px-1.5 py-px text-label text-muted">{kind}</span>}
        {row.context && <span className="sr-only">(shown for the one it started)</span>}
      </div>
      <div role="gridcell" title={status.hint}>
        <Pill tone={status.tone} testId="subagent-status">
          <Icon aria-hidden="true" width={11} height={11} className="mr-1" />
          {status.word}
        </Pill>
      </div>
      <div role="gridcell" className="text-right text-body tabular-nums text-ink">
        {timeWords(agent)}
      </div>
      <div role="gridcell" className="text-right text-body tabular-nums text-ink" title={own}>
        {tokenWords(agent.tokens)}
      </div>
      <div role="gridcell" className="truncate text-meta text-muted" title={agent.model ?? undefined}>
        {modelWords(agent.model) ?? "—"}
      </div>
    </div>
  );
}
