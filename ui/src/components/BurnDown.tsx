/**
 * AJ-E1512 — Usage's runway in the units people plan with: per window,
 * about how many sessions like this account's usual one are left, and a
 * burn-down — what's been used over the window, the pace's projection to
 * running out (only when that's before the reset), and the reset. The
 * figures are worked out from the vendor's own readings and carry their
 * confidence word; with too little to go on, it says so.
 */
import type { JSX } from "react";
import { quotaTierTag } from "./primitives";
import { resetClock } from "../lib/quotaWindow";
import type { QuotaDisplayData, RunwayDisplay } from "../lib/ipc/quota";

const W = 300;
const H = 72;
const PAD = 4;

/** "About 4 sessions left", or why there's no number yet. */
export function sessionsLeftWords(runway: RunwayDisplay): string {
  if (runway.sessions_left === null || runway.session_pct === null) {
    return runway.sessions_counted < 3
      ? "Sessions left show after a few sessions on this account"
      : "Sessions left show once this window has moved a little";
  }
  const n = runway.sessions_left;
  if (n === 0) return "Not enough left for a usual session";
  if (n >= 20) return "20+ usual sessions left";
  return `About ${n} usual session${n === 1 ? "" : "s"} left (each about ${Math.max(1, Math.round(runway.session_pct))}%)`;
}

/** When it runs out at this pace, or that it lasts past the reset. */
export function paceWords(runway: RunwayDisplay): string {
  if (runway.exhausts_at_ms !== null) return `At this pace it runs out about ${resetClock(runway.exhausts_at_ms)}, before the reset at ${resetClock(runway.resets_at_ms)}`;
  if (runway.pct_per_hour === null) return `Resets ${resetClock(runway.resets_at_ms)}`;
  return `At this pace it lasts past the reset at ${resetClock(runway.resets_at_ms)}`;
}

/** The chart: used over the window (solid), the projection (dashed), now
 * and the reset (the right edge). Points beyond the window are clipped. */
export function BurnDownChart({ runway, label }: { runway: RunwayDisplay; label: string }): JSX.Element {
  const span = Math.max(1, runway.resets_at_ms - runway.starts_ms);
  const x = (at: number) => PAD + ((Math.min(Math.max(at, runway.starts_ms), runway.resets_at_ms) - runway.starts_ms) / span) * (W - 2 * PAD);
  const y = (pct: number) => H - PAD - (Math.min(Math.max(pct, 0), 100) / 100) * (H - 2 * PAD);
  const used = runway.series.map(([at, pct]) => `${x(at).toFixed(1)},${y(pct).toFixed(1)}`).join(" ");
  const last = runway.series.at(-1);
  let projection: string | null = null;
  if (last && runway.pct_per_hour !== null && runway.pct_per_hour > 0) {
    const end = runway.exhausts_at_ms ?? runway.resets_at_ms;
    const endPct = runway.exhausts_at_ms !== null ? 100 : last[1] + (runway.pct_per_hour * (end - last[0])) / 3_600_000;
    projection = `${x(last[0]).toFixed(1)},${y(last[1]).toFixed(1)} ${x(end).toFixed(1)},${y(endPct).toFixed(1)}`;
  }
  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      preserveAspectRatio="none"
      className="h-[72px] w-full"
      role="img"
      aria-label={label}
      focusable="false"
      data-testid="burn-down-chart"
    >
      <line x1={PAD} x2={W - PAD} y1={y(100)} y2={y(100)} stroke="var(--color-hairline)" strokeDasharray="2 3" vectorEffect="non-scaling-stroke" />
      <line x1={PAD} x2={W - PAD} y1={y(0)} y2={y(0)} stroke="var(--color-hairline)" vectorEffect="non-scaling-stroke" />
      <line x1={W - PAD} x2={W - PAD} y1={y(100)} y2={y(0)} stroke="var(--color-muted)" strokeWidth={1} vectorEffect="non-scaling-stroke" data-testid="burn-down-reset" />
      {used && <polyline points={used} fill="none" stroke="var(--color-accent-hi)" strokeWidth={1.8} strokeLinejoin="round" vectorEffect="non-scaling-stroke" data-testid="burn-down-used" />}
      {projection && (
        <polyline points={projection} fill="none" stroke={runway.exhausts_at_ms !== null ? "var(--color-warning)" : "var(--color-muted)"} strokeWidth={1.4} strokeDasharray="4 3" vectorEffect="non-scaling-stroke" data-testid="burn-down-projection" />
      )}
      {runway.exhausts_at_ms !== null && (
        <line
          x1={x(runway.exhausts_at_ms)}
          x2={x(runway.exhausts_at_ms)}
          y1={y(100)}
          y2={y(0)}
          stroke="var(--color-warning)"
          strokeDasharray="2 2"
          vectorEffect="non-scaling-stroke"
          data-testid="burn-down-runs-out"
        />
      )}
    </svg>
  );
}

function WindowRunway({ name, runway }: { name: string; runway: RunwayDisplay }): JSX.Element {
  const label = `${name}: ${Math.round(runway.used_pct)}% used. ${sessionsLeftWords(runway)}. ${paceWords(runway)}.`;
  return (
    <div className="flex flex-col gap-1" data-testid="window-runway">
      <span className="flex flex-wrap items-baseline gap-x-1.5 text-xs">
        <span className="font-medium text-ink">{name}</span>
        <span className="text-ink" data-testid="sessions-left">
          {sessionsLeftWords(runway)}
        </span>
        {runway.sessions_left !== null && (
          <span className="text-label text-muted" data-testid="sessions-left-tier">
            {quotaTierTag(runway.badge)}
          </span>
        )}
      </span>
      <BurnDownChart runway={runway} label={label} />
      <span className="text-label text-muted" data-testid="burn-down-pace">
        {paceWords(runway)}
      </span>
    </div>
  );
}

/** Each window that has a runway, 5-hour first. Nothing when neither does. */
export function BurnDowns({ display }: { display: QuotaDisplayData | undefined }): JSX.Element | null {
  const windows = [
    ["5-hour window", display?.five_hour?.runway],
    ["This week", display?.weekly?.runway],
  ].filter((entry): entry is [string, RunwayDisplay] => entry[1] != null);
  if (windows.length === 0) return null;
  return (
    <div className="flex flex-col gap-3 border-t border-hairline/60 pt-3" data-testid="burn-downs">
      {windows.map(([name, runway]) => (
        <WindowRunway key={name} name={name} runway={runway} />
      ))}
    </div>
  );
}
