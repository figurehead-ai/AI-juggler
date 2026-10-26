/**
 * ADR-0021 — a share as a donut: segments in the legend's order with a
 * 2 px gap, the total (or the leader) in the middle, and a legend that
 * names each part with its value and percentage, so colour is never the
 * only key. Arrow keys read the parts.
 */
import { useState, type JSX } from "react";
import { Spoken, Tooltip, useRover, type Tip } from "./ChartCard";
import { exact, percentOf, short, type Unit } from "./scale";

export type DonutItem = { key: string; label: string; value: number; color: string; href?: () => void };

const RING = 18;
const GAP = 0.03;

function arc(c: number, r: number, r0: number, a0: number, a1: number): string {
  const large = a1 - a0 > Math.PI ? 1 : 0;
  const p = (radius: number, angle: number) => `${(c + radius * Math.cos(angle)).toFixed(2)},${(c + radius * Math.sin(angle)).toFixed(2)}`;
  return `M${p(r, a0)}A${r},${r} 0 ${large} 1 ${p(r, a1)}L${p(r0, a1)}A${r0},${r0} 0 ${large} 0 ${p(r0, a0)}Z`;
}

export function Donut({
  items,
  unit,
  label,
  centre,
  size = 168,
  onPick,
}: {
  items: DonutItem[];
  unit: Unit;
  /** What it shows ("Tokens by profile"). */
  label: string;
  /** The words in the middle: a value and what it is. */
  centre: { value: string; words: string };
  size?: number;
  /** A part's row was picked (opens its page). */
  onPick?: (key: string) => void;
}): JSX.Element {
  const shown = items.filter((item) => item.value > 0);
  const total = shown.reduce((sum, item) => sum + item.value, 0);
  const [hover, setHover] = useState<number | null>(null);
  const rover = useRover(shown.length, (i) => `${shown[i]!.label}: ${short(shown[i]!.value, unit)}, ${percentOf(shown[i]!.value, total)}`);
  const at = rover.active ?? hover;
  const c = size / 2;
  const r = c - 2;
  let angle = -Math.PI / 2;
  const segments = shown.map((item) => {
    const sweep = total > 0 ? (item.value / total) * Math.PI * 2 : 0;
    const gap = shown.length > 1 ? Math.min(GAP, sweep / 3) : 0;
    const a0 = angle + gap / 2;
    const a1 = angle + sweep - gap / 2;
    angle += sweep;
    // A whole ring can't be one arc: two halves.
    const d = shown.length === 1 ? `${arc(c, r, r - RING, -Math.PI / 2, Math.PI / 2)}${arc(c, r, r - RING, Math.PI / 2, (3 * Math.PI) / 2)}` : arc(c, r, r - RING, a0, a1);
    return { item, d, mid: (a0 + a1) / 2 };
  });
  const tip: Tip | null =
    at === null || !segments[at]
      ? null
      : {
          x: c + (r - RING / 2) * Math.cos(segments[at]!.mid),
          y: c + (r - RING / 2) * Math.sin(segments[at]!.mid),
          title: segments[at]!.item.label,
          rows: [
            { color: segments[at]!.item.color, label: "Used", value: exact(segments[at]!.item.value, unit) },
            { label: "Share", value: percentOf(segments[at]!.item.value, total) },
          ],
        };
  return (
    <div className="flex flex-wrap items-center justify-center gap-x-7 gap-y-4" data-testid="donut">
      <div
        className="relative shrink-0 rounded-full outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
        style={{ width: size, height: size }}
        tabIndex={0}
        role="group"
        aria-label={`${label}: ${shown.length} parts; the arrow keys read them.`}
        onKeyDown={rover.onKeyDown}
        onBlur={rover.onBlur}
      >
        <svg width={size} height={size} aria-hidden="true" className="block">
          {total === 0 && <circle cx={c} cy={c} r={r - RING / 2} fill="none" stroke="var(--color-surface-3)" strokeWidth={RING} />}
          {segments.map((segment, i) => (
            <path
              key={segment.item.key}
              d={segment.d}
              fill={segment.item.color}
              opacity={at !== null && at !== i ? 0.45 : 1}
              onPointerEnter={() => setHover(i)}
              onPointerLeave={() => setHover(null)}
              data-testid="donut-segment"
            />
          ))}
          <text x={c} y={c + 1} textAnchor="middle" className="fill-ink text-[22px] font-semibold tabular-nums">
            {centre.value}
          </text>
          <text x={c} y={c + 21} textAnchor="middle" className="fill-muted text-label">
            {centre.words}
          </text>
        </svg>
        <Tooltip tip={tip} width={size + 120} />
        <Spoken text={rover.said} />
      </div>
      <ul className="flex min-w-0 flex-[1_1_220px] flex-col" aria-label={`${label}, as a list`} data-testid="donut-legend">
        {shown.map((item, i) => {
          const row = (
            <>
              <span aria-hidden="true" className="size-2.5 shrink-0 rounded-[3px]" style={{ background: item.color }} />
              <span className="min-w-0 flex-1 truncate text-left text-ink">{item.label}</span>
              <span className="tabular-nums text-ink" title={exact(item.value, unit)}>
                {short(item.value, unit)}
              </span>
              <span className="w-11 text-right tabular-nums text-muted">{percentOf(item.value, total)}</span>
            </>
          );
          const cls = "flex w-full items-center gap-2.5 rounded-md px-1.5 py-1 text-meta " + (at === i ? "bg-surface-2" : "");
          return (
            <li key={item.key} onPointerEnter={() => setHover(i)} onPointerLeave={() => setHover(null)}>
              {onPick ? (
                <button type="button" className={cls + " outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-accent/60"} onClick={() => onPick(item.key)}>
                  {row}
                </button>
              ) : (
                <span className={cls}>{row}</span>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
