/**
 * ADR-0021 — how a piece of work travelled: one lane per profile, a
 * capsule wherever it worked (darker on busier days), and an arrow for
 * each move between accounts. Arrow keys read it day by day.
 */
import { useState, type JSX } from "react";
import { Spoken, Tooltip, useRover, useWidth, type Tip } from "./ChartCard";
import { short } from "./scale";

export type Lane = { key: string; label: string; color: string; values: number[] };
export type Move = { at: number; from: string; to: string; words: string };

const LANE_H = 40;
const TOP = 6;
const AXIS = 26;

export function Timeline({
  lanes,
  edges,
  moves,
  label,
  bucketWords,
  tickWords,
}: {
  lanes: Lane[];
  edges: number[];
  moves: Move[];
  label: string;
  bucketWords: (start: number, end: number) => string;
  tickWords: (start: number) => string;
}): JSX.Element {
  const [ref, width] = useWidth<HTMLDivElement>();
  const n = Math.max(1, edges.length - 1);
  const labelW = Math.min(180, Math.max(110, width * 0.16));
  const plotW = Math.max(40, width - labelW - 12);
  const band = plotW / n;
  const x = (i: number) => labelW + i * band;
  const atX = (ms: number) => labelW + ((ms - edges[0]!) / (edges[n]! - edges[0]!)) * plotW;
  const laneY = (index: number) => TOP + index * LANE_H + LANE_H / 2;
  const height = TOP + lanes.length * LANE_H + AXIS;
  const max = Math.max(1, ...lanes.flatMap((lane) => lane.values));
  const describe = (i: number) => {
    const busy = lanes.filter((lane) => (lane.values[i] ?? 0) > 0).map((lane) => `${lane.label} ${short(lane.values[i]!, "tokens")}`);
    return `${bucketWords(edges[i]!, edges[i + 1]!)}: ${busy.length ? busy.join(", ") : "no work"}`;
  };
  const rover = useRover(n, describe);
  const [hover, setHover] = useState<number | null>(null);
  const at = rover.active ?? hover;
  const tip: Tip | null =
    at === null
      ? null
      : {
          x: x(at) + band / 2,
          y: 0,
          title: bucketWords(edges[at]!, edges[at + 1]!),
          rows: lanes.filter((lane) => (lane.values[at] ?? 0) > 0).map((lane) => ({ color: lane.color, label: lane.label, value: short(lane.values[at]!, "tokens") })),
        };
  if (tip && tip.rows.length === 0) tip.rows.push({ label: "No work here", value: "" });
  const every = Math.max(1, Math.ceil(n / Math.max(2, Math.floor(plotW / 100))));
  return (
    <div
      ref={ref}
      tabIndex={0}
      role="group"
      aria-label={`${label}: the arrow keys read it day by day.`}
      onKeyDown={rover.onKeyDown}
      onBlur={rover.onBlur}
      className="relative rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
      data-testid="work-timeline"
    >
      <svg width={width} height={height} aria-hidden="true" className="block">
        {lanes.map((lane, index) => {
          const y = laneY(index);
          // Runs of working days become one capsule each.
          const runs: [number, number][] = [];
          lane.values.forEach((value, i) => {
            if (value <= 0) return;
            const last = runs[runs.length - 1];
            if (last && last[1] === i - 1) last[1] = i;
            else runs.push([i, i]);
          });
          return (
            <g key={lane.key}>
              <line x1={labelW} x2={labelW + plotW} y1={y} y2={y} stroke="var(--color-hairline)" strokeWidth={1} />
              <circle cx={10} cy={y} r={5} fill={lane.color} />
              <text x={24} y={y + 5} className="fill-ink text-meta font-semibold">
                {lane.label.length > 18 ? `${lane.label.slice(0, 17)}…` : lane.label}
              </text>
              {runs.map(([from, to]) => (
                <rect key={from} x={x(from) + 1} y={y - 8} width={Math.max(6, x(to + 1) - x(from) - 2)} height={16} rx={8} fill={lane.color} opacity={0.35} />
              ))}
              {lane.values.map((value, i) =>
                value > 0 ? <rect key={`d-${i}`} x={x(i) + 1} y={y - 8} width={Math.max(4, band - 2)} height={16} rx={Math.min(8, band / 2)} fill={lane.color} opacity={0.35 + 0.65 * (value / max)} /> : null,
              )}
            </g>
          );
        })}
        {moves.map((move) => {
          const from = lanes.findIndex((lane) => lane.key === move.from);
          const to = lanes.findIndex((lane) => lane.key === move.to);
          if (from < 0 || to < 0 || from === to) return null;
          const mx = atX(move.at);
          const down = to > from;
          const y0 = laneY(from) + (down ? 9 : -9);
          const y1 = laneY(to) + (down ? -10 : 10);
          return (
            <g key={`${move.at}-${move.from}`}>
              <line x1={mx} x2={mx} y1={y0} y2={y1} stroke="var(--color-ink)" strokeWidth={1.5} />
              <path d={`M${mx - 4},${y1 + (down ? -5 : 5)}L${mx},${y1}L${mx + 4},${y1 + (down ? -5 : 5)}`} fill="none" stroke="var(--color-ink)" strokeWidth={1.5} />
              <circle cx={mx} cy={laneY(from)} r={4} fill="var(--color-ink)" stroke="var(--color-surface)" strokeWidth={2} />
            </g>
          );
        })}
        {at !== null && <line x1={x(at) + band / 2} x2={x(at) + band / 2} y1={TOP} y2={height - AXIS} stroke="var(--color-ink)" opacity={0.35} />}
        {edges.slice(0, n).map((start, i) =>
          (n - 1 - i) % every === 0 ? (
            <text key={start} x={Math.min(labelW + plotW - 20, Math.max(labelW + 20, x(i) + band / 2))} y={height - 8} textAnchor="middle" className="fill-muted text-label">
              {tickWords(start)}
            </text>
          ) : null,
        )}
        <rect
          x={labelW}
          y={TOP}
          width={plotW}
          height={height - AXIS - TOP}
          fill="transparent"
          onPointerMove={(event) => {
            const box = event.currentTarget.getBoundingClientRect();
            setHover(Math.max(0, Math.min(n - 1, Math.floor(((event.clientX - box.left) / box.width) * n))));
          }}
          onPointerLeave={() => setHover(null)}
        />
      </svg>
      <Tooltip tip={tip} width={width} />
      <Spoken text={rover.said} />
    </div>
  );
}
