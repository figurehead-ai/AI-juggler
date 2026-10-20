/**
 * D-25 — one Account's runway, in words: how much is left in the 5-hour
 * window and this week, when each resets, and (only when it is coming)
 * when this pace runs out. Unmeasured says so; with no account reading
 * yet, the newest session's own quota line is shown and named as such.
 */
import type { JSX } from "react";
import type { InstanceInfo, QuotaDisplayData, WindowDisplay } from "../lib/bridge";

export type RunwayLine = {
  label: string;
  left: number | null;
  words: string;
  tone: "ok" | "low" | "out" | "none";
};

function when(ms: number): string {
  return new Date(ms).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
}

/** One window, in words. */
export function windowLine(label: string, window: WindowDisplay | undefined, omitted?: string): RunwayLine {
  if (!window || window.state === "unknown") {
    return { label, left: null, words: omitted ?? "Not measured yet", tone: "none" };
  }
  if (window.state === "learning") {
    return { label, left: null, words: "Measuring — shows after a little use", tone: "none" };
  }
  const left = Math.max(0, Math.round(100 - (window.percentage ?? 0)));
  const resets = window.resets_at_ms ? ` · resets ${when(window.resets_at_ms)}` : "";
  if (window.state === "exhausted" || left === 0) {
    return { label, left: 0, words: `Used up${window.resets_at_ms ? ` until ${when(window.resets_at_ms)}` : ""}`, tone: "out" };
  }
  const stale = window.state === "stale" ? " · last known" : "";
  return { label, left, words: `${left}% left${resets}${stale}`, tone: left <= 25 ? "low" : "ok" };
}

/** Both windows, plus the session fallback when neither is measured. */
export function runwayLines(display: QuotaDisplayData | undefined, sessions: InstanceInfo[] = []): RunwayLine[] {
  const lines = [
    windowLine("5-hour window", display?.five_hour, display?.five_hour_omitted_reason),
    windowLine("This week", display?.weekly, display?.weekly_omitted_reason),
  ];
  if (lines.some((line) => line.left !== null)) return lines;
  const reading = sessions
    .filter((s) => s.quota !== null)
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms))[0]?.quota;
  if (!reading) return lines;
  const left = Math.max(0, Math.round(100 - reading.used_percentage));
  return [
    {
      label: "From the latest session",
      left,
      words: `${left}% left${reading.resets_at_ms ? ` · resets ${when(reading.resets_at_ms)}` : ""}`,
      tone: left === 0 ? "out" : left <= 25 ? "low" : "ok",
    },
  ];
}

/** "At this pace it runs out in about 40 min" — only when it will. */
export function paceWords(display: QuotaDisplayData | undefined): string | null {
  const forecast = display?.forecast;
  if (!forecast || forecast.state !== "exhausts" || forecast.tte_min === undefined) return null;
  const minutes = Math.round(forecast.tte_min);
  const span = minutes >= 90 ? `${Math.round(minutes / 60)} h` : `${minutes} min`;
  return `At this pace it runs out in about ${span}`;
}

const BAR: Record<RunwayLine["tone"], string> = {
  ok: "bg-accent",
  low: "bg-warning",
  out: "bg-danger",
  none: "bg-transparent",
};
const WORDS: Record<RunwayLine["tone"], string> = {
  ok: "text-muted",
  low: "text-warning",
  out: "text-danger",
  none: "text-faint",
};

export function AccountRunway({
  display,
  sessions = [],
}: {
  display: QuotaDisplayData | undefined;
  sessions?: InstanceInfo[];
}): JSX.Element {
  if (display?.unavailable_reason) {
    return (
      <p className="text-xs text-muted" data-testid="account-runway">
        {display.unavailable_reason}
      </p>
    );
  }
  const lines = runwayLines(display, sessions);
  const pace = paceWords(display);
  return (
    <div className="flex flex-col gap-2.5" data-testid="account-runway">
      <div className="grid gap-x-6 gap-y-2.5 sm:grid-cols-2">
        {lines.map((line) => (
          <div key={line.label} className="flex min-w-0 flex-col gap-1">
            <span className="text-[11px] font-medium text-ink">{line.label}</span>
            <span
              aria-hidden="true"
              className={"h-1.5 overflow-hidden rounded-full " + (line.left === null ? "border border-dashed border-hairline" : "bg-surface-2")}
            >
              <span className={"block h-full rounded-full " + BAR[line.tone]} style={{ width: `${line.left ?? 0}%` }} />
            </span>
            <span className={"text-[11px] " + WORDS[line.tone]}>{line.words}</span>
          </div>
        ))}
      </div>
      {pace && (
        <p className="text-[11px] text-warning" data-testid="account-runway-pace">
          {pace}
        </p>
      )}
    </div>
  );
}
