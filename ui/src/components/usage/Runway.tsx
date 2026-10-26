/**
 * 0.1.5 — an account's limits at a glance: a gauge per window (what's
 * left, when it resets, how old the reading is), the windows over the last
 * week, and its usage in tokens either way. Every state is said in words:
 * a fresh reading, one whose window has reset since, a limit hit, being
 * measured, and never measured (which still shows the tokens used).
 */
import type { JSX } from "react";
import { asOfWords, measured, tokenWords } from "../../lib/quotaWindow";
import { useCached } from "../../lib/usage/cached";
import { daysBetween } from "../../lib/usage/range";
import { usageLimits } from "../../lib/ipc/usageHistory";
import type { QuotaDisplayData, WindowDisplay } from "../../lib/bridge";
import { Gauge } from "../charts/Gauge";
import { LimitHistory } from "../charts/LimitHistory";
import { Sparkline } from "../charts/StatTile";

/** How long until `at`, in words ("in 40 min", "in 2 h 10 min", "in 3 days"). */
export function inWords(at: number, now: number): string {
  const minutes = Math.round((at - now) / 60_000);
  if (minutes <= 0) return "now";
  if (minutes < 60) return `in ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `in ${hours} h${minutes % 60 ? ` ${minutes % 60} min` : ""}`;
  const days = Math.round(hours / 24);
  return `in ${days} ${days === 1 ? "day" : "days"}`;
}

/** A reset, short: the time today ("4:20 PM"), else the day and time ("Mon 10:40 AM"). */
export function resetShort(at: number, now: number): string {
  const time = new Date(at).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  return new Date(at).toDateString() === new Date(now).toDateString() ? time : `${new Date(at).toLocaleDateString([], { weekday: "short" })} ${time}`;
}

/** When the reading was taken, in words ("12 min ago", "13 h ago"). */
function readWords(at: number, now: number): string {
  const minutes = Math.max(0, Math.round((now - at) / 60_000));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  return hours < 48 ? `${hours} h ago` : asOfWords(at, now).replace(/^as of /, "on ");
}

/** One window's gauge, from the engine's display of it. */
export function WindowGauge({ name, window, omitted, now }: { name: string; window: WindowDisplay | null | undefined; omitted?: string | null; now: number }): JSX.Element {
  if (!window) return <Gauge name={name} left={null} state="unmeasured" note={omitted ?? null} testId="window-gauge" />;
  if (window.state === "learning") return <Gauge name={name} left={null} state="measuring" testId="window-gauge" />;
  if (window.state === "reset") {
    const when = window.measured_at_ms != null ? `, ${readWords(window.measured_at_ms, now)}` : "";
    const used = window.last_used_pct ?? window.percentage;
    const last = used != null ? `Last read ${Math.round(used)}% used${when}; it has reset since` : "It has reset since the last reading";
    return <Gauge name={name} left={null} state="stale" note={last} testId="window-gauge" />;
  }
  const resets = window.resets_at_ms != null ? resetShort(window.resets_at_ms, now) : null;
  const until = window.resets_at_ms != null ? inWords(window.resets_at_ms, now) : null;
  if (window.state === "exhausted" || (measured(window) && window.percentage >= 100)) {
    return <Gauge name={name} left={0} resets={resets} resetsIn={until ? `Limit hit · back ${until}` : "Limit hit"} testId="window-gauge" />;
  }
  if (!measured(window)) {
    const used = window.burned_tokens > 0 ? `${tokenWords(window.burned_tokens)} tokens used in it` : null;
    return <Gauge name={name} left={null} state="unmeasured" note={used} testId="window-gauge" />;
  }
  return <Gauge name={name} left={100 - window.percentage} resets={resets} resetsIn={until} note={window.state === "stale" ? "last known" : null} testId="window-gauge" />;
}

/** Whether a window has no reading at all (never read, not being read). */
function unread(window: WindowDisplay | null | undefined): boolean {
  if (!window) return true;
  if (window.state === "learning" || window.state === "reset" || window.state === "exhausted") return false;
  return !measured(window);
}

/** Neither window read: one gauge for both, with the tokens used, so
 * "Not measured yet" is said once (the review of 0.1.5), not per window. */
export function BothUnread({ display }: { display: QuotaDisplayData | undefined }): JSX.Element {
  const week = display?.weekly?.burned_tokens ?? 0;
  const five = display?.five_hour?.burned_tokens ?? 0;
  const omitted = [display?.five_hour_omitted_reason, display?.weekly_omitted_reason].filter(Boolean);
  const note =
    week > 0
      ? `${tokenWords(week)} tokens used this week`
      : five > 0
        ? `${tokenWords(five)} tokens used in this 5-hour window`
        : omitted.length > 0 && omitted.every((reason) => reason === omitted[0])
          ? omitted[0]!
          : null;
  return <Gauge name="5-hour window and this week" left={null} state="unmeasured" note={note} testId="window-gauge" />;
}

/** The newest reading's age, for the line under both gauges. */
export function readingLine(display: QuotaDisplayData | undefined, now: number): string | null {
  const windows = [display?.five_hour, display?.weekly].filter((window): window is WindowDisplay => window != null && window.measured_at_ms != null);
  const newest = windows.sort((a, b) => (b.measured_at_ms ?? 0) - (a.measured_at_ms ?? 0))[0];
  if (!newest) return null;
  const from = newest.source_words ? ` from ${newest.source_words}` : "";
  return `Limits read${from} ${readWords(newest.measured_at_ms!, now)}`;
}

/** The limits and usage of one account (or one profile). */
export function RunwayPanel({
  display,
  limitsProfile,
  usage,
  usageTotal,
  color,
  name,
  now,
  compact = false,
}: {
  display: QuotaDisplayData | undefined;
  /** The profile whose readings draw the week (the account's measured one). */
  limitsProfile: string | null;
  /** Tokens per bucket over the last 7 days. */
  usage: number[] | null;
  usageTotal: number | null;
  color: string;
  name: string;
  now: number;
  compact?: boolean;
}): JSX.Element {
  const start = now - 7 * 86_400_000;
  const hour = Math.floor(now / 3_600_000);
  const limits = useCached(limitsProfile ? `limits:${limitsProfile}:${hour}` : null, () => usageLimits(limitsProfile!, start, now), 120_000);
  const week = limits.data?.weekly.points ?? [];
  const five = limits.data?.five_hour.windows ?? [];
  const hasHistory = week.length > 1 || five.length > 0;
  if (display?.unavailable_reason) {
    return (
      <div className="flex flex-col gap-3">
        <p className="text-meta text-muted">{display.unavailable_reason}</p>
        <UsageLine usage={usage} usageTotal={usageTotal} name={name} color={color} />
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-3.5" data-testid="account-runway">
      {unread(display?.five_hour) && unread(display?.weekly) ? (
        <BothUnread display={display} />
      ) : (
        <div className="grid grid-cols-1 gap-3 @min-[500px]:grid-cols-2">
          <WindowGauge name="5-hour window" window={display?.five_hour} omitted={display?.five_hour_omitted_reason} now={now} />
          <WindowGauge name="This week" window={display?.weekly} omitted={display?.weekly_omitted_reason} now={now} />
        </div>
      )}
      {readingLine(display, now) && <p className="-mt-1 text-label text-muted">{readingLine(display, now)}</p>}
      {hasHistory && !compact ? (
        <div className="flex flex-col gap-1.5">
          <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-label text-muted" aria-hidden="true">
            <span className="flex items-center gap-1.5">
              <span className="h-0.5 w-3 rounded-full bg-ink" />
              Week used
            </span>
            <span className="flex items-center gap-1.5">
              <span className="size-2.5 rounded-[3px] opacity-60" style={{ background: color }} />
              Each 5-hour window, at its peak
            </span>
          </p>
          <LimitHistory week={week} windows={five} start={start} end={now} color={color} label={`${name}: the week's limit used, and each 5-hour window, over the last 7 days`} dayWords={daysBetween(start, now)} />
        </div>
      ) : null}
      <UsageLine usage={usage} usageTotal={usageTotal} name={name} color={color} />
    </div>
  );
}

/** The account's tokens over the last 7 days: always shown, measured
 * limits or not. */
function UsageLine({ usage, usageTotal, name, color }: { usage: number[] | null; usageTotal: number | null; name: string; color: string }): JSX.Element {
  return (
    <div className="flex items-end gap-3" data-testid="runway-usage">
      <span className="flex shrink-0 flex-col">
        <span className="text-label text-muted">Last 7 days</span>
        <span className="text-section tabular-nums text-ink" title={usageTotal == null ? undefined : `${Math.round(usageTotal).toLocaleString("en-US")} tokens`}>
          {usageTotal == null ? "—" : `${tokenWords(Math.round(usageTotal))} tokens`}
        </span>
      </span>
      <span className="min-w-0 flex-1">
        <Sparkline values={usage ?? []} label={`${name}: tokens over the last 7 days`} color={color} height={34} />
      </span>
    </div>
  );
}

