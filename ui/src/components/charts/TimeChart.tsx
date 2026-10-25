/**
 * ADR-0020 — usage over time: stacked bars (up to 60 buckets) or a
 * stacked area (more), one series per profile, tool, model or project.
 * A crosshair snaps to the nearest bucket and the tooltip lists every
 * series there; arrow keys read the buckets one by one.
 */
import { useMemo, useState, type JSX } from "react";
import { Spoken, Tooltip, useRover, useWidth, type Tip } from "./ChartCard";
import { exact, linear, niceAxis, short, stackOf, totalsOf, type Series, type Unit } from "./scale";

export type TimeMarker = { at: number; label: string };

export type TimeChartProps = {
  /** The buckets' edges, epoch ms (one more than each series' values). */
  edges: number[];
  series: Series[];
  unit: Unit;
  height?: number;
  /** What the chart shows, for screen readers ("Tokens per day by profile"). */
  label: string;
  /** A bucket in words, for the tooltip ("Thu 17 Oct"). */
  bucketWords: (start: number, end: number) => string;
  /** A tick's short label ("17 Oct", "14:00"). */
  tickWords: (start: number) => string;
  /** Buckets drawn on a faint band (weekends). */
  shade?: (start: number) => boolean;
  /** Moments marked on the axis (moves between accounts). */
  markers?: TimeMarker[];
  mode?: "bars" | "area";
};

const PAD_TOP = 10;
const PAD_BOTTOM = 28;
const GAP = 2;
const AXIS_CHAR = 7.2;

function roundTop(x: number, y: number, w: number, h: number, r: number): string {
  const radius = Math.max(0, Math.min(r, h, w / 2));
  return `M${x},${y + h}V${y + radius}Q${x},${y} ${x + radius},${y}H${x + w - radius}Q${x + w},${y} ${x + w},${y + radius}V${y + h}Z`;
}

export function TimeChart(props: TimeChartProps): JSX.Element {
  const { edges, series, unit, height = 260, label, bucketWords, tickWords, shade, markers = [] } = props;
  const [ref, width] = useWidth<HTMLDivElement>();
  const n = Math.max(0, edges.length - 1);
  const mode = props.mode ?? (n > 60 ? "area" : "bars");
  const totals = useMemo(() => totalsOf(series, n), [series, n]);
  const axis = niceAxis(Math.max(...totals, 0) * 1.04);
  const tickText = axis.ticks.map((tick) => (tick === 0 ? "0" : short(tick, unit)));
  const padLeft = Math.ceil(Math.max(...tickText.map((text) => text.length)) * AXIS_CHAR) + 14;
  const plotW = Math.max(40, width - padLeft - 6);
  const plotH = height - PAD_TOP - PAD_BOTTOM;
  const y = linear(axis.top, PAD_TOP + plotH, PAD_TOP);
  const band = n > 0 ? plotW / n : plotW;
  const barW = Math.max(2, Math.min(24, band * 0.66));
  const centre = (i: number) => padLeft + i * band + band / 2;
  const [hover, setHover] = useState<number | null>(null);
  const describe = (i: number) => {
    const parts = series.filter((entry) => (entry.values[i] ?? 0) > 0).map((entry) => `${entry.label} ${short(entry.values[i]!, unit)}`);
    return `${bucketWords(edges[i]!, edges[i + 1]!)}: ${short(totals[i] ?? 0, unit)} in all${parts.length > 1 ? `; ${parts.join(", ")}` : ""}`;
  };
  const rover = useRover(n, describe);
  const at = rover.active ?? hover;

  const tip: Tip | null =
    at === null
      ? null
      : {
          x: centre(at),
          y: 4,
          title: bucketWords(edges[at]!, edges[at + 1]!),
          rows: series
            .filter((entry) => (entry.values[at] ?? 0) > 0)
            .map((entry) => ({ color: entry.color, label: entry.label, value: short(entry.values[at]!, unit) })),
          total: series.length > 1 ? { label: "All", value: short(totals[at] ?? 0, unit) } : undefined,
        };
  if (tip && tip.rows.length === 0) tip.rows.push({ label: "Nothing used", value: short(0, unit) });

  // About six ticks, the newest bucket always labelled.
  const every = Math.max(1, Math.ceil(n / Math.max(2, Math.floor(plotW / 110))));
  const ticks: number[] = [];
  for (let i = n - 1; i >= 0; i -= every) ticks.unshift(i);

  const onMove = (event: React.PointerEvent<SVGRectElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    const index = Math.floor(((event.clientX - box.left) / box.width) * n);
    setHover(Math.max(0, Math.min(n - 1, index)));
  };

  const below = new Array<number>(n).fill(0);
  const areas =
    mode !== "area" || n === 0
      ? []
      : series.map((entry) => {
          const top = entry.values.map((value, i) => below[i]! + Math.max(0, value ?? 0));
          const upper = top.map((value, i) => `${i ? "L" : "M"}${centre(i).toFixed(1)},${y(value).toFixed(1)}`).join("");
          const lower = below
            .map((value, i) => [centre(i), y(value)] as const)
            .reverse()
            .map(([px, py]) => `L${px.toFixed(1)},${py.toFixed(1)}`)
            .join("");
          below.splice(0, n, ...top);
          return { key: entry.key, color: entry.color, d: `${upper}${lower}Z`, line: upper };
        });

  return (
    <div
      ref={ref}
      tabIndex={0}
      role="group"
      aria-label={`${label}. ${n} ${n === 1 ? "bucket" : "buckets"}: the arrow keys read them one by one.`}
      onKeyDown={rover.onKeyDown}
      onBlur={rover.onBlur}
      className="relative rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
      data-testid="time-chart"
      data-mode={mode}
    >
      <svg width={width} height={height} aria-hidden="true" className="block overflow-visible">
        {shade &&
          edges.slice(0, n).map((start, i) =>
            shade(start) ? <rect key={`shade-${start}`} x={padLeft + i * band} y={PAD_TOP} width={band} height={plotH} fill="var(--color-ink)" opacity={0.03} /> : null,
          )}
        {axis.ticks.map((tick, i) => (
          <g key={`y-${tick}`}>
            <line x1={padLeft} x2={padLeft + plotW} y1={y(tick)} y2={y(tick)} stroke="var(--color-hairline)" strokeWidth={1} />
            <text x={padLeft - 10} y={y(tick) + 4} textAnchor="end" className="fill-muted text-label tabular-nums">
              {tickText[i]}
            </text>
          </g>
        ))}
        {mode === "bars"
          ? edges.slice(0, n).map((start, i) => {
              const stack = stackOf(series, i);
              const x = centre(i) - barW / 2;
              const dim = at !== null && at !== i;
              return (
                <g key={`bar-${start}`} opacity={dim ? 0.55 : 1} data-testid="time-bar">
                  {stack.map((segment, s) => {
                    const top = y(segment.to);
                    const h = Math.max(0, y(segment.from) - top - (s > 0 ? GAP : 0));
                    const color = series.find((entry) => entry.key === segment.key)!.color;
                    return s === stack.length - 1 ? (
                      <path key={segment.key} d={roundTop(x, top, barW, h, 4)} fill={color} />
                    ) : (
                      <rect key={segment.key} x={x} y={top} width={barW} height={h} fill={color} />
                    );
                  })}
                </g>
              );
            })
          : areas.map((area) => (
              <g key={area.key}>
                <path d={area.d} fill={area.color} opacity={0.82} />
                <path d={area.line} fill="none" stroke="var(--color-surface)" strokeWidth={1.5} strokeLinejoin="round" />
              </g>
            ))}
        <line x1={padLeft} x2={padLeft + plotW} y1={y(0)} y2={y(0)} stroke="var(--color-hairline)" strokeWidth={1} />
        {markers.map((marker) => {
          const i = edges.findIndex((edge, k) => k < n && marker.at >= edge && marker.at < edges[k + 1]!);
          if (i < 0) return null;
          const x = padLeft + ((marker.at - edges[0]!) / (edges[n]! - edges[0]!)) * plotW;
          return (
            <g key={`m-${marker.at}`}>
              <line x1={x} x2={x} y1={PAD_TOP} y2={y(0)} stroke="var(--color-ink)" strokeWidth={1} opacity={0.5} />
              <circle cx={x} cy={PAD_TOP + 3} r={4} fill="var(--color-ink)" stroke="var(--color-surface)" strokeWidth={2}>
                <title>{marker.label}</title>
              </circle>
            </g>
          );
        })}
        {at !== null && mode === "area" && (
          <line x1={centre(at)} x2={centre(at)} y1={PAD_TOP} y2={y(0)} stroke="var(--color-ink)" strokeWidth={1} opacity={0.6} />
        )}
        {ticks.map((i) => {
          const x = centre(i);
          const anchor = x - padLeft < 30 ? "start" : padLeft + plotW - x < 30 ? "end" : "middle";
          return (
            <text key={`x-${i}`} x={anchor === "start" ? x - band / 2 : anchor === "end" ? x + band / 2 : x} y={height - 8} textAnchor={anchor} className="fill-muted text-label tabular-nums">
              {tickWords(edges[i]!)}
            </text>
          );
        })}
        <rect
          x={padLeft}
          y={PAD_TOP}
          width={plotW}
          height={plotH}
          fill="transparent"
          className="cursor-crosshair"
          onPointerMove={onMove}
          onPointerLeave={() => setHover(null)}
          data-testid="time-chart-plot"
        />
      </svg>
      <Tooltip tip={tip} width={width} />
      <Spoken text={rover.said} />
    </div>
  );
}

/** The table a time chart shows instead: a row per bucket, newest first. */
export function timeTable(edges: number[], series: Series[], unit: Unit, bucketWords: (start: number, end: number) => string) {
  const n = Math.max(0, edges.length - 1);
  const totals = totalsOf(series, n);
  const rows: (string | number)[][] = [];
  for (let i = n - 1; i >= 0; i -= 1) {
    rows.push([bucketWords(edges[i]!, edges[i + 1]!), ...series.map((entry) => exact(entry.values[i] ?? 0, unit)), exact(totals[i] ?? 0, unit)]);
  }
  return {
    columns: [{ label: "When" }, ...series.map((entry) => ({ label: entry.label, numeric: true })), { label: "All", numeric: true }],
    rows,
  };
}
