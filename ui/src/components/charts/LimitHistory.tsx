/**
 * ADR-0021 — a profile's limits over time: the weekly window's percent
 * used as a line (it climbs through the week and drops at its reset), and
 * each 5-hour window as a bar to the highest it was read at. Arrow keys
 * read it day by day.
 */
import { useState, type JSX } from "react";
import { Spoken, Tooltip, useRover, useWidth, type Tip } from "./ChartCard";
import { linear } from "./scale";

export type LimitWindow = { first_ms: number; resets_ms: number | null; peak_pct: number };

const PAD_LEFT = 44;
const PAD_BOTTOM = 22;
const PAD_TOP = 6;
const FIVE_HOURS = 5 * 3_600_000;
/** Readings come about every 15 minutes while the app runs; more than an
 * hour between two is a gap. */
const GAP_MS = 3_600_000;

/** The week's reading at `at`: the last one at or before it. */
function weekAt(points: [number, number][], at: number): number | null {
  let found: number | null = null;
  for (const [ts, pct] of points) {
    if (ts > at) break;
    found = pct;
  }
  return found;
}

export function LimitHistory({
  week,
  windows,
  start,
  end,
  color,
  label,
  dayWords,
  height = 120,
}: {
  /** The weekly window's readings, oldest first. */
  week: [number, number][];
  /** The 5-hour windows, oldest first. */
  windows: LimitWindow[];
  start: number;
  end: number;
  color: string;
  label: string;
  /** Day starts in the user's time zone, and their words ("Mon", "17 Oct"). */
  dayWords: { at: number; words: string; long: string }[];
  height?: number;
}): JSX.Element {
  const [ref, width] = useWidth<HTMLDivElement>();
  const plotW = Math.max(40, width - PAD_LEFT - 8);
  const x = (at: number) => PAD_LEFT + ((at - start) / Math.max(1, end - start)) * plotW;
  const y = linear(100, height - PAD_BOTTOM, PAD_TOP);
  const days = dayWords.filter((day) => day.at < end);
  const facts = (i: number) => {
    const day = days[i]!;
    const next = days[i + 1]?.at ?? end;
    const peak = Math.max(0, ...windows.filter((w) => w.first_ms >= day.at && w.first_ms < next).map((w) => w.peak_pct));
    const weekly = weekAt(week, next - 1);
    return { weekly: weekly === null ? "not read" : `${Math.round(weekly)}% used`, peak: `${Math.round(peak)}%` };
  };
  const describe = (i: number) => {
    const { weekly, peak } = facts(i);
    return `${days[i]!.long}: week ${weekly}; busiest 5-hour window ${peak}`;
  };
  const rover = useRover(days.length, describe);
  const [hover, setHover] = useState<number | null>(null);
  const at = rover.active ?? hover;
  const tip: Tip | null =
    at === null || !days[at]
      ? null
      : {
          x: x(days[at]!.at),
          y: 0,
          title: days[at]!.long,
          rows: [
            { color: "var(--color-ink)", label: "Week", value: facts(at).weekly },
            { color, label: "Busiest 5-hour", value: facts(at).peak },
          ],
        };
  // A gap in the readings (the app was closed) is a gap in the line,
  // never a straight guess across it.
  const line = week
    .map(([ts, pct], i) => `${i === 0 || ts - week[i - 1]![0] > GAP_MS ? "M" : "L"}${x(ts).toFixed(1)},${y(pct).toFixed(1)}`)
    .join("");
  const last = week[week.length - 1];
  return (
    <div
      ref={ref}
      tabIndex={0}
      role="group"
      aria-label={`${label}: the arrow keys read it day by day.`}
      onKeyDown={rover.onKeyDown}
      onBlur={rover.onBlur}
      className="relative rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
      data-testid="limit-history"
    >
      <svg width={width} height={height} aria-hidden="true" className="block">
        {[0, 50, 100].map((pct) => (
          <g key={pct}>
            <line x1={PAD_LEFT} x2={PAD_LEFT + plotW} y1={y(pct)} y2={y(pct)} stroke="var(--color-hairline)" strokeWidth={1} />
            <text x={PAD_LEFT - 8} y={y(pct) + 4} textAnchor="end" className="fill-muted text-label tabular-nums">
              {pct}%
            </text>
          </g>
        ))}
        {windows.map((w) => {
          const from = Math.max(start, w.first_ms);
          const to = Math.min(end, w.resets_ms ?? w.first_ms + FIVE_HOURS);
          if (to <= start || from >= end) return null;
          const left = x(from);
          const barW = Math.max(2, x(to) - left - 1.5);
          return <rect key={w.first_ms} x={left} y={y(w.peak_pct)} width={barW} height={Math.max(0, y(0) - y(w.peak_pct))} rx={1.5} fill={color} opacity={0.5} />;
        })}
        {line && <path d={line} fill="none" stroke="var(--color-ink)" strokeWidth={2} strokeLinejoin="round" />}
        {last && <circle cx={x(last[0])} cy={y(last[1])} r={4} fill="var(--color-ink)" stroke="var(--color-surface)" strokeWidth={2} />}
        {at !== null && days[at] && <line x1={x(days[at]!.at)} x2={x(days[at]!.at)} y1={PAD_TOP} y2={y(0)} stroke="var(--color-ink)" strokeWidth={1} opacity={0.4} />}
        {days.map((day, i) => {
          const next = days[i + 1]?.at ?? end;
          const every = Math.max(1, Math.ceil(days.length / Math.max(2, Math.floor(plotW / 48))));
          return i % every === 0 ? (
            <text key={day.at} x={(x(day.at) + x(next)) / 2} y={height - 5} textAnchor="middle" className="fill-muted text-label">
              {day.words}
            </text>
          ) : null;
        })}
        <rect
          x={PAD_LEFT}
          y={PAD_TOP}
          width={plotW}
          height={height - PAD_TOP - PAD_BOTTOM}
          fill="transparent"
          onPointerMove={(event) => {
            const box = event.currentTarget.getBoundingClientRect();
            const time = start + ((event.clientX - box.left) / box.width) * (end - start);
            const index = days.findIndex((day, k) => time >= day.at && time < (days[k + 1]?.at ?? end));
            setHover(index >= 0 ? index : null);
          }}
          onPointerLeave={() => setHover(null)}
        />
      </svg>
      <Tooltip tip={tip} width={width} />
      <Spoken text={rover.said} />
    </div>
  );
}
