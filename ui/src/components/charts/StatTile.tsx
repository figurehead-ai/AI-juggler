/**
 * ADR-0021 — the figures a page leads with: a stat tile (a label, a big
 * number, its change against the period before, a sparkline), the
 * sparkline itself, and a share bar (one item's part of a whole, split by
 * who did it).
 */
import type { JSX, ReactNode } from "react";
import { change, exact, short, type Unit } from "./scale";

/** A tiny trend line: an area wash, a 2 px line, the newest point marked. */
export function Sparkline({ values, label, color = "var(--color-accent)", height = 36 }: { values: number[]; label: string; color?: string; height?: number }): JSX.Element {
  const w = 300;
  const max = Math.max(...values, 0);
  if (values.length < 2 || max <= 0) {
    return (
      <svg width="100%" height={height} viewBox={`0 0 ${w} ${height}`} preserveAspectRatio="none" role="img" aria-label={`${label}: nothing yet`}>
        <line x1={0} x2={w} y1={height - 2} y2={height - 2} stroke="var(--color-hairline)" strokeWidth={1} vectorEffect="non-scaling-stroke" />
      </svg>
    );
  }
  const dx = w / (values.length - 1);
  const points = values.map((value, i) => [i * dx, height - 3 - (Math.max(0, value) / max) * (height - 8)] as const);
  const line = points.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)}`).join("");
  return (
    <svg width="100%" height={height} viewBox={`0 0 ${w} ${height}`} preserveAspectRatio="none" role="img" aria-label={label} data-testid="sparkline">
      <path d={`${line}L${w},${height}L0,${height}Z`} fill={color} opacity={0.14} />
      <path d={line} fill="none" stroke={color} strokeWidth={2} strokeLinejoin="round" strokeLinecap="round" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

/** Whether more is good news for this figure (cost: no; sessions: neutral). */
export type Better = "up" | "down" | "neither";

export function Delta({ now, before, better, period }: { now: number; before: number; better: Better; period: string }): JSX.Element {
  const { dir, words } = change(now, before);
  const good = better === "neither" || dir === "same" || dir === "new" ? null : (dir === "up") === (better === "up");
  const tone = good === null ? "bg-surface-2 text-muted" : good ? "bg-success-soft text-success" : "bg-warning-soft text-warning";
  const arrow = dir === "up" ? "▲" : dir === "down" ? "▼" : "•";
  const said = dir === "same" ? `the same as ${period}` : dir === "new" ? `none ${period}` : `${dir} ${words} on ${period}`;
  return (
    <span className="flex flex-wrap items-center gap-x-2 gap-y-1" data-testid="stat-delta" data-dir={dir}>
      <span className={"inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-label font-semibold " + tone}>
        <span aria-hidden="true">{arrow}</span>
        <span className="sr-only">{said}</span>
        <span aria-hidden="true">{dir === "new" ? "New" : dir === "same" ? "Same" : words}</span>
      </span>
      <span className="text-label text-muted" aria-hidden="true">
        vs {period}
      </span>
    </span>
  );
}

export function StatTile({
  label,
  icon,
  value,
  unit,
  suffix,
  full = false,
  before,
  better = "neither",
  period,
  trend,
  note,
  words,
  atLeast = false,
  estimate = false,
  holdSpace = false,
  testId,
}: {
  label: string;
  icon?: ReactNode;
  value: number | null;
  unit: Unit;
  /** Words after the number ("h", "min ago"). */
  suffix?: string;
  /** The value in full ("$198.63"), where cents matter (prices). */
  full?: boolean;
  /** The period before's value, for the change. */
  before?: number | null;
  better?: Better;
  /** The period before, in words ("the 30 days before"). */
  period?: string;
  /** A value per bucket, for the sparkline. */
  trend?: number[];
  /** A line instead of (or under) the change. */
  note?: ReactNode;
  /** Said instead of a number that can't be known ("No list price"):
   * never a made-up 0 (§4.11). */
  words?: string;
  /** The number is a floor (some of it couldn't be counted): "at least
   * ~$12", in words (the 0.2 walk, F15: "≥ at least $53.2k"). */
  atLeast?: boolean;
  /** The number is an estimate (a cost at API prices): "~$12" (§10). */
  estimate?: boolean;
  /** Its first figures are on their way: the change line and the
   * sparkline keep their space meanwhile (0.3.4: the tiles grew a read
   * later, pushing the page down). */
  holdSpace?: boolean;
  testId?: string;
}): JSX.Element {
  return (
    <div className="sheen flex min-w-0 flex-col gap-1.5 rounded-card border border-hairline bg-surface px-5 pb-3.5 pt-4 shadow-card" data-testid={testId ?? "stat-tile"}>
      <span className="flex items-center gap-1.5 text-meta text-muted">
        {icon && <span aria-hidden="true">{icon}</span>}
        {label}
      </span>
      <span className={words !== undefined && value === null ? "text-section font-semibold text-ink" : "text-stat text-ink"} title={value === null ? undefined : exact(value, unit)} data-testid="stat-value">
        {atLeast && value !== null && <span className="text-section font-medium text-muted">at least </span>}
        {estimate && value !== null ? "~" : ""}
        {value === null ? (words ?? "—") : full ? exact(value, unit) : short(value, unit)}
        {suffix && value !== null && <span className="ml-1 text-section font-medium text-muted">{suffix}</span>}
      </span>
      {before != null && period && value !== null && <Delta now={value} before={before} better={better} period={period} />}
      {holdSpace && value === null && <span aria-hidden="true" className="w-16 rounded-md bg-skeleton px-1.5 py-0.5 text-label motion-safe:animate-pulse">&nbsp;</span>}
      {note && <span className="text-label text-muted">{note}</span>}
      {trend && trend.length > 1 ? (
        <div className="mt-auto pt-1">
          <Sparkline values={trend} label={`${label}, over the range`} />
        </div>
      ) : holdSpace && value === null ? (
        <div className="mt-auto pt-1" aria-hidden="true">
          <Sparkline values={[]} label={label} />
        </div>
      ) : null}
    </div>
  );
}

/** One part of a share bar. */
export type SharePart = { key: string; label: string; value: number; color: string };

/** An item's share of the whole, split by who did it: the bar's length is
 * the item against the list's biggest (`scale`, the total when absent),
 * each part in its colour; the percentage beside is of the total. */
export function ShareBar({ parts, total, scale, unit, label }: { parts: SharePart[]; total: number; scale?: number; unit: Unit; label: string }): JSX.Element {
  const sum = parts.reduce((acc, part) => acc + Math.max(0, part.value), 0);
  const share = total > 0 ? sum / total : 0;
  const pct = share * 100;
  return (
    <span className="flex min-w-[72px] items-center gap-2" data-testid="share-bar">
      <span
        className="flex h-2.5 flex-1 gap-[2px] overflow-hidden rounded-full bg-surface-3"
        role="img"
        aria-label={`${label}: ${pct < 1 && pct > 0 ? "under 1" : Math.round(pct)}% of the total, ${parts.map((part) => `${part.label} ${short(part.value, unit)}`).join(", ")}`}
      >
        {parts
          .filter((part) => part.value > 0)
          .map((part) => (
            <span key={part.key} className="h-full" style={{ width: `${(part.value / Math.max(1, scale ?? total)) * 100}%`, background: part.color }} title={`${part.label}: ${exact(part.value, unit)}`} />
          ))}
      </span>
      <span className="w-10 text-right text-meta tabular-nums text-muted" aria-hidden="true">
        {pct > 0 && pct < 1 ? "<1%" : `${Math.round(pct)}%`}
      </span>
    </span>
  );
}
