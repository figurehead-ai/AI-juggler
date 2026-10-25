/**
 * ADR-0021 — when the work happens: the hours with usage folded into the
 * user's weekdays and hours ("hours"), or each day of the range in
 * calendar weeks ("days"). One hue, light to dark; a cell's count is in
 * its tooltip and the arrow keys read the cells.
 */
import { useState, type JSX } from "react";
import { Spoken, Tooltip, useWidth, type Tip } from "./ChartCard";

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] as const;
const LABEL_W = 40;
const TOP = 22;
const GAP = 3;

/** Monday-first weekday of a local date. */
function weekday(date: Date): number {
  return (date.getDay() + 6) % 7;
}

/** The ramp's colour for a share of the busiest cell (0: the empty cell). */
export function rampColor(share: number): string {
  if (share <= 0) return "var(--color-surface-2)";
  return `color-mix(in oklab, var(--color-accent) ${Math.round(22 + share * 78)}%, var(--color-surface-2))`;
}

type Cell = { row: number; col: number; count: number; words: string };

/** Hours with usage → a 7 × 24 grid of how many such hours fell there. */
export function hourCells(hours: number[]): Cell[] {
  const counts = new Array<number>(7 * 24).fill(0);
  for (const at of hours) {
    const date = new Date(at);
    counts[weekday(date) * 24 + date.getHours()]! += 1;
  }
  return counts.map((count, i) => {
    const row = Math.floor(i / 24);
    const col = i % 24;
    const day = WEEKDAYS[row]!;
    return { row, col, count, words: `${day} ${String(col).padStart(2, "0")}:00: ${count === 0 ? "no usage" : `used in ${count} ${count === 1 ? "week" : "weeks"}`}` };
  });
}

/** Hours with usage → one cell per day from `start` to `end`, by week. */
export function dayCells(hours: number[], start: number, end: number): { cells: Cell[]; weeks: number; firstMonday: number } {
  const first = new Date(start);
  first.setHours(0, 0, 0, 0);
  first.setDate(first.getDate() - weekday(first));
  const perDay = new Map<string, number>();
  const key = (date: Date) => `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
  for (const at of hours) {
    const date = new Date(at);
    perDay.set(key(date), (perDay.get(key(date)) ?? 0) + 1);
  }
  const cells: Cell[] = [];
  const day = new Date(first);
  let col = 0;
  while (day.getTime() < end) {
    const row = weekday(day);
    if (day.getTime() >= start - 86_400_000) {
      const count = perDay.get(key(day)) ?? 0;
      const words = day.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
      cells.push({ row, col, count, words: `${words}: ${count === 0 ? "no usage" : `${count} ${count === 1 ? "hour" : "hours"} with usage`}` });
    }
    day.setDate(day.getDate() + 1);
    if (weekday(day) === 0) col += 1;
  }
  return { cells, weeks: col + 1, firstMonday: first.getTime() };
}

export function Heatmap({ cells, columns, label, columnLabels }: { cells: Cell[]; columns: number; label: string; columnLabels: { col: number; words: string }[] }): JSX.Element {
  const [ref, width] = useWidth<HTMLDivElement>();
  const cw = Math.max(6, (width - LABEL_W - 4) / columns - GAP);
  const ch = Math.min(32, Math.max(16, cw * 0.9));
  const height = TOP + 7 * (ch + GAP);
  const max = Math.max(1, ...cells.map((cell) => cell.count));
  const [active, setActive] = useState<number | null>(null);
  const [hover, setHover] = useState<number | null>(null);
  const [said, setSaid] = useState("");
  const at = active ?? hover;
  const go = (next: number | null) => {
    setActive(next);
    setSaid(next === null ? "" : cells[next]!.words);
  };
  const find = (row: number, col: number) => cells.findIndex((cell) => cell.row === row && cell.col === col);
  const onKeyDown = (event: React.KeyboardEvent) => {
    const cell = active === null ? null : cells[active]!;
    const moves: Record<string, [number, number]> = { ArrowRight: [0, 1], ArrowLeft: [0, -1], ArrowDown: [1, 0], ArrowUp: [-1, 0] };
    if (event.key === "Escape") return go(null);
    if (event.key === "Home") return (event.preventDefault(), go(0));
    if (event.key === "End") return (event.preventDefault(), go(cells.length - 1));
    const move = moves[event.key];
    if (!move) return;
    event.preventDefault();
    if (!cell) return go(0);
    const next = find(cell.row + move[0], cell.col + move[1]);
    if (next >= 0) go(next);
  };
  const x = (col: number) => LABEL_W + col * (cw + GAP);
  const y = (row: number) => TOP + row * (ch + GAP);
  const tip: Tip | null =
    at === null ? null : { x: x(cells[at]!.col) + cw / 2, y: y(cells[at]!.row) + ch + 6, title: cells[at]!.words.split(":")[0]!, rows: [{ label: cells[at]!.words.split(": ").slice(1).join(": "), value: "" }] };
  return (
    <div
      ref={ref}
      className="relative rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
      tabIndex={0}
      role="group"
      aria-label={`${label}: the arrow keys read each cell.`}
      onKeyDown={onKeyDown}
      onBlur={() => go(null)}
      data-testid="heatmap"
    >
      <svg width={width} height={height} aria-hidden="true" className="block">
        {columnLabels.map((entry) => (
          <text key={`c-${entry.col}`} x={x(entry.col) + cw / 2} y={14} textAnchor="middle" className="fill-muted text-label tabular-nums">
            {entry.words}
          </text>
        ))}
        {WEEKDAYS.map((day, row) => (
          <text key={day} x={0} y={y(row) + ch / 2 + 4} className="fill-muted text-label">
            {day}
          </text>
        ))}
        {cells.map((cell, i) => (
          <rect
            key={`${cell.row}-${cell.col}`}
            x={x(cell.col)}
            y={y(cell.row)}
            width={cw}
            height={ch}
            rx={3}
            fill={rampColor(cell.count / max)}
            stroke={at === i ? "var(--color-ink)" : "none"}
            strokeWidth={1.5}
            onPointerEnter={() => setHover(i)}
            onPointerLeave={() => setHover(null)}
            data-testid="heatmap-cell"
            data-count={cell.count}
          />
        ))}
      </svg>
      <Tooltip tip={tip} width={width} />
      <Spoken text={said} />
    </div>
  );
}

/** The ramp's key: "Less" to "More" in five steps. */
export function RampKey({ less, more }: { less: string; more: string }): JSX.Element {
  return (
    <p className="flex items-center gap-1.5 text-label text-muted" aria-hidden="true">
      {less}
      {[0.1, 0.3, 0.55, 0.8, 1].map((share) => (
        <span key={share} className="size-3 rounded-[3px]" style={{ background: rampColor(share) }} />
      ))}
      {more}
    </p>
  );
}

export { WEEKDAYS };
