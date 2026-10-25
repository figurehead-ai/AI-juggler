/**
 * ADR-0021 — the frame every chart sits in: a card with its title row
 * (an icon, the title in ink, a muted line, its controls), a legend, the
 * chart or its table ("Show as table"), and an honest empty state that
 * keeps the chart's height. The tooltip and the keyboard reading of a
 * chart live here too, so every chart reads the same way.
 */
import { useEffect, useId, useLayoutEffect, useRef, useState, type JSX, type ReactNode } from "react";

/** One line of a tooltip: a key stroke in the series' colour, the name,
 * the value (strong: the reader has the series and wants the number). */
export type TipRow = { color?: string; label: string; value: string };
export type Tip = { x: number; y: number; title: string; rows: TipRow[]; total?: TipRow };

/** A chart's table view: its columns and rows, numbers right-aligned. */
export type ChartTable = { columns: { label: string; numeric?: boolean }[]; rows: (string | number)[][] };

/** The width a chart has to draw in (ResizeObserver; 640 where there is none). */
export function useWidth<T extends HTMLElement>(): [React.RefObject<T | null>, number] {
  const ref = useRef<T | null>(null);
  const [width, setWidth] = useState(640);
  useLayoutEffect(() => {
    const node = ref.current;
    if (!node) return;
    const measure = () => {
      const next = Math.floor(node.getBoundingClientRect().width);
      if (next > 0) setWidth(next);
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  return [ref, width];
}

/** A one-dimensional keyboard reading: arrows move along the marks, Home
 * and End jump, Escape lets go. `describe` says the active mark in words
 * for the live region. */
export function useRover(count: number, describe: (index: number) => string) {
  const [active, setActive] = useState<number | null>(null);
  const [said, setSaid] = useState("");
  const move = (next: number | null) => {
    setActive(next);
    setSaid(next === null ? "" : describe(next));
  };
  const onKeyDown = (event: React.KeyboardEvent) => {
    if (count === 0) return;
    const at = active ?? -1;
    const keys: Record<string, number | null> = {
      ArrowRight: Math.min(count - 1, at + 1),
      ArrowDown: Math.min(count - 1, at + 1),
      ArrowLeft: Math.max(0, at - 1),
      ArrowUp: Math.max(0, at - 1),
      Home: 0,
      End: count - 1,
      Escape: null,
    };
    if (!(event.key in keys)) return;
    event.preventDefault();
    move(keys[event.key]!);
  };
  return { active, setActive: move, onKeyDown, said, onBlur: () => move(null) };
}

/** The live region a chart's keyboard reading speaks through. */
export function Spoken({ text }: { text: string }): JSX.Element {
  return (
    <div aria-live="polite" className="sr-only">
      {text}
    </div>
  );
}

export function Tooltip({ tip, width }: { tip: Tip | null; width: number }): JSX.Element | null {
  if (!tip) return null;
  const left = Math.max(0, Math.min(tip.x + 14, width - 236));
  return (
    <div
      role="presentation"
      className="pointer-events-none absolute z-20 w-[220px] rounded-xl border border-hairline bg-surface-3/95 px-3 py-2.5 text-meta shadow-card backdrop-blur"
      style={{ left, top: Math.max(0, tip.y) }}
      data-testid="chart-tooltip"
    >
      <p className="mb-1.5 text-label text-muted">{tip.title}</p>
      <ul className="flex flex-col gap-1">
        {tip.rows.map((row) => (
          <li key={row.label} className="grid grid-cols-[12px_minmax(0,1fr)_auto] items-center gap-2">
            <span aria-hidden="true" className="h-0.5 w-3 rounded-full" style={{ background: row.color ?? "transparent" }} />
            <span className="truncate text-muted">{row.label}</span>
            <span className="font-semibold tabular-nums text-ink">{row.value}</span>
          </li>
        ))}
      </ul>
      {tip.total && (
        <p className="mt-1.5 flex justify-between border-t border-hairline pt-1.5 font-semibold">
          <span>{tip.total.label}</span>
          <span className="tabular-nums">{tip.total.value}</span>
        </p>
      )}
    </div>
  );
}

/** A legend entry: a swatch shaped like the mark (a square for areas and
 * bars, a line for lines), the name in ink, a value in muted. */
export type LegendItem = { key: string; label: string; color: string; value?: string; line?: boolean };

export function Legend({ items, label }: { items: LegendItem[]; label: string }): JSX.Element | null {
  if (items.length < 2) return null;
  return (
    <ul aria-label={label} className="flex flex-wrap gap-x-4 gap-y-1.5 text-meta" data-testid="chart-legend">
      {items.map((item) => (
        <li key={item.key} className="flex min-w-0 items-center gap-1.5">
          <span aria-hidden="true" className={item.line ? "h-0.5 w-3 rounded-full" : "size-2.5 rounded-[3px]"} style={{ background: item.color }} />
          <span className="truncate text-ink">{item.label}</span>
          {item.value && <span className="tabular-nums text-muted">{item.value}</span>}
        </li>
      ))}
    </ul>
  );
}

export function TableView({ table, caption }: { table: ChartTable; caption: string }): JSX.Element {
  return (
    <div className="max-h-80 overflow-auto rounded-lg border border-hairline" data-testid="chart-table">
      <table className="w-full border-collapse text-body">
        <caption className="sr-only">{caption}</caption>
        <thead>
          <tr>
            {table.columns.map((column) => (
              <th key={column.label} scope="col" className={"sticky top-0 bg-surface-2 px-3 py-2 text-meta font-semibold text-muted " + (column.numeric ? "text-right" : "text-left")}>
                {column.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {table.rows.map((row, index) => (
            <tr key={index} className="border-t border-hairline/60 hover:bg-surface-2">
              {row.map((cell, at) => (
                <td key={at} className={"px-3 py-1.5 " + (table.columns[at]?.numeric ? "text-right tabular-nums" : "")}>
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** The empty state, at the chart's own height: faint grid lines and what
 * will show there, so the page doesn't jump when data comes. */
export function EmptyPlot({ height, words }: { height: number; words: string }): JSX.Element {
  return (
    <div className="relative flex items-center justify-center rounded-lg" style={{ height }} data-testid="chart-empty">
      <div aria-hidden="true" className="absolute inset-0 flex flex-col justify-between py-2">
        {[0, 1, 2, 3].map((line) => (
          <span key={line} className="block h-px bg-hairline/60" />
        ))}
      </div>
      <p className="relative rounded-lg bg-surface px-3 py-1.5 text-meta text-muted">{words}</p>
    </div>
  );
}

export function TableToggle({ on, onToggle }: { on: boolean; onToggle: () => void }): JSX.Element {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onToggle}
      className="inline-flex h-8 items-center gap-1.5 rounded-lg border border-hairline bg-surface-2 px-2.5 text-label font-medium text-ink outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60"
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
        <path d="M3 5h18v14H3zM3 10h18M9 10v9" />
      </svg>
      {on ? "Show as chart" : "Show as table"}
    </button>
  );
}

/** A section card at the readability scale: 20 px padding, a 15 px title
 * in ink with its icon, a 13 px muted line beside it, controls on the right. */
export function Section({
  title,
  icon,
  sub,
  actions,
  children,
  className = "",
  testId,
  busy = false,
}: {
  title: string;
  icon?: ReactNode;
  sub?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  testId?: string;
  busy?: boolean;
}): JSX.Element {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      aria-busy={busy || undefined}
      className={"sheen flex min-w-0 flex-col gap-3.5 rounded-card border border-hairline bg-surface p-card shadow-card transition-opacity " + (busy ? "opacity-70 " : "") + className}
      data-testid={testId}
    >
      <div className="flex flex-wrap items-center gap-x-2.5 gap-y-2">
        {icon && <span aria-hidden="true" className="text-muted">{icon}</span>}
        <h3 id={id} className="text-section text-ink">
          {title}
        </h3>
        {sub && <span className="text-meta text-muted">{sub}</span>}
        {actions && <div className="ml-auto flex flex-wrap items-center gap-2">{actions}</div>}
      </div>
      {children}
    </section>
  );
}

/** A chart's body: the chart, or its table, or its empty state. */
export function ChartBody({
  empty,
  emptyWords,
  height,
  table,
  showTable,
  caption,
  children,
}: {
  empty: boolean;
  emptyWords: string;
  height: number;
  table: ChartTable;
  showTable: boolean;
  caption: string;
  children: ReactNode;
}): JSX.Element {
  if (showTable && !empty) return <TableView table={table} caption={caption} />;
  if (empty) return <EmptyPlot height={height} words={emptyWords} />;
  return <>{children}</>;
}

/** Hides the tooltip when the pointer leaves the window (a tooltip
 * never sticks after the mouse has gone). */
export function useTipReset(clear: () => void): void {
  useEffect(() => {
    window.addEventListener("blur", clear);
    return () => window.removeEventListener("blur", clear);
  }, [clear]);
}
