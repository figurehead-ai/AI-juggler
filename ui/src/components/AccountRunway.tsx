/**
 * D-25 — one Account's runway, in words: how much is left in the 5-hour
 * window and this week, when each resets, and (only when it is coming)
 * when this pace runs out. Unmeasured says so; with no account reading
 * yet, the newest session's own quota line is shown and named as such.
 *
 * The one runway: Home's one-line summary (`runwayOf`), the Accounts and
 * Usage cards and the limit dialog (`runwayLines`) all come from here, on
 * `lib/quotaWindow`'s rule. Home kept a copy once, and item A's fix for
 * "100% left" never reached it (ADR-0019: no duplicate logic).
 */
import type { JSX } from "react";
import type { InstanceInfo, QuotaDisplayData, WindowDisplay } from "../lib/bridge";
import { asOfWords, measured, resetClock, resetWords, tokenWords, windowView } from "../lib/quotaWindow";

export type RunwayLine = {
  label: string;
  left: number | null;
  words: string;
  tone: "ok" | "low" | "out" | "none";
};

/** One window, in words (the one rule: `lib/quotaWindow`). A missing
 * percentage is never "% left" or a filled bar (the 0.0.12 report). */
export function windowLine(label: string, window: WindowDisplay | null | undefined, omitted?: string | null): RunwayLine {
  const view = windowView(window, omitted);
  return { label, left: view.left, words: view.words, tone: view.tone };
}

/** Both windows, plus the session fallback when neither is measured. */
export function runwayLines(display: QuotaDisplayData | undefined, sessions: InstanceInfo[] = []): RunwayLine[] {
  const lines = [
    windowLine("5-hour window", display?.five_hour, display?.five_hour_omitted_reason),
    windowLine("This week", display?.weekly, display?.weekly_omitted_reason),
  ];
  if (lines.some((line) => line.left !== null)) return lines;
  const reading = latestSessionReading(sessions);
  if (!reading) return lines;
  const left = Math.max(0, Math.round(100 - reading.used_percentage));
  return [
    {
      label: "From the latest session",
      left,
      words: `${left}% left${reading.resets_at_ms ? ` · resets ${resetClock(reading.resets_at_ms)}` : ""}`,
      tone: left === 0 ? "out" : left <= 25 ? "low" : "ok",
    },
  ];
}

/** The newest session's own quota reading: a real reading when the account
 * has none yet (the newest one wins), never a guess. */
function latestSessionReading(sessions: InstanceInfo[]): NonNullable<InstanceInfo["quota"]> | undefined {
  return sessions
    .filter((s) => s.quota != null)
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms))[0]?.quota ?? undefined;
}

/** Home's one line for an account: % left of the tighter measured window,
 * else the newest session's own reading, else why there is none. */
export function runwayOf(
  display: QuotaDisplayData | undefined,
  sessions: InstanceInfo[] = [],
): { left: number | null; words: string; resets: string | null } {
  const windows: Array<{ label: string; phrase: string; window: WindowDisplay | null | undefined }> = [
    { label: "5-hour window", phrase: "in the 5-hour window", window: display?.five_hour },
    { label: "Week", phrase: "this week", window: display?.weekly },
  ];
  const known = windows.filter((entry): entry is (typeof windows)[number] & { window: WindowDisplay & { percentage: number } } =>
    measured(entry.window),
  );
  if (known.length > 0) {
    const tightest = known.reduce((a, b) => (a.window.percentage >= b.window.percentage ? a : b));
    const left = Math.max(0, Math.round(100 - tightest.window.percentage));
    const words = resetWords(tightest.window);
    return { left, words: `${left}% left ${tightest.phrase}`, resets: words ? `${tightest.label} ${words}` : null };
  }
  const reading = latestSessionReading(sessions);
  if (reading) {
    const left = Math.max(0, Math.round(100 - reading.used_percentage));
    return {
      left,
      words: `${left}% left, from its latest session`,
      resets: reading.resets_at_ms != null ? `Resets ${resetClock(reading.resets_at_ms)}` : null,
    };
  }
  if (windows.some((entry) => entry.window?.state === "learning")) {
    return { left: null, words: "Measuring — shows after a little use", resets: null };
  }
  if (windows.some((entry) => entry.window?.state === "reset")) {
    return { left: null, words: "Not measured since the reset", resets: null };
  }
  const used = windows.reduce((most, entry) => Math.max(most, entry.window?.burned_tokens ?? 0), 0);
  return { left: null, words: `Not measured yet${used > 0 ? ` · ${tokenWords(used)} tokens used` : ""}`, resets: null };
}

/** The account's credits in words, with who reported them and when,
 * when its tool reports them ("Credits: 16.25 · reported by Codex, as of
 * 12 min ago"). */
export function creditsWords(display: QuotaDisplayData | undefined): string | null {
  const credits = display?.credits;
  if (credits == null) return null;
  const amount = credits.unlimited ? "unlimited" : credits.balance;
  if (amount == null) return null;
  return `Credits: ${amount} · reported by ${credits.source}, ${asOfWords(credits.as_of_ms)}`;
}

/** "At this pace it runs out in about 40 min" — only when it will. */
export function paceWords(display: QuotaDisplayData | undefined): string | null {
  const forecast = display?.forecast;
  if (!forecast || forecast.state !== "exhausts" || forecast.tte_min == null) return null;
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
  const credits = creditsWords(display);
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
      {credits && (
        <p className="text-[11px] text-muted" data-testid="account-runway-credits">
          {credits}
        </p>
      )}
      {pace && (
        <p className="text-[11px] text-warning" data-testid="account-runway-pace">
          {pace}
        </p>
      )}
    </div>
  );
}
