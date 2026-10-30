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
import { signedIn, type AccountInfo, type InstanceInfo, type QuotaDisplayData, type WindowDisplay } from "../lib/bridge";
import { READING_LIMITS } from "../lib/history-reading";
import { asOfWords, limitWindows, measured, resetWords, usedTokensWords, windowView } from "../lib/quotaWindow";
import { resetsLead, resetsWords } from "../lib/time-words";
import { LOW_LEFT, rankByRoom } from "../lib/room";
import { usingCredits, usingCreditsWords, type CreditsCover } from "../lib/credits";

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

/** Every limit (the 5 hours, the week, each model's own week), plus the
 * session fallback when none is measured. */
export function runwayLines(display: QuotaDisplayData | undefined, sessions: InstanceInfo[] = []): RunwayLine[] {
  // 0.3.2: a used-up window its credits carry on past isn't a stop: amber,
  // not red (the credits line says why).
  const credits = usingCredits(display) !== null;
  const lines = limitWindows(display)
    .map((entry) => windowLine(entry.label, entry.window, entry.omitted))
    .map((line) => (credits && line.tone === "out" ? { ...line, tone: "low" as const } : line));
  if (lines.some((line) => line.left !== null)) return lines;
  const reading = sessionFallback(display, sessions);
  if (!reading) return lines;
  const left = Math.max(0, Math.round(100 - reading.used_percentage));
  return [
    {
      label: "Last read in a session",
      left,
      words: `${left}% left${reading.resets_at_ms ? ` · ${resetsWords(reading.resets_at_ms)}` : ""}`,
      tone: left === 0 ? "out" : left <= 25 ? "low" : "ok",
    },
  ];
}

/** The newest session's own quota reading, when the account has none: a
 * real reading (the newest one wins), never a guess. None once a window
 * has reset since the vendor's last reading, or when the session's own
 * reading has passed its reset: an old window's "% left" must not come
 * back as the new one's (review of #120). */
function sessionFallback(
  display: QuotaDisplayData | undefined,
  sessions: InstanceInfo[],
  now: number = Date.now(),
): NonNullable<InstanceInfo["quota"]> | undefined {
  if (display?.five_hour?.state === "reset" || display?.weekly?.state === "reset") return undefined;
  const reading = sessions
    .filter((s) => s.quota != null)
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms))[0]?.quota;
  if (reading == null || (reading.resets_at_ms != null && reading.resets_at_ms <= now)) return undefined;
  return reading;
}

/** Home's one line for an account: % left of the tighter measured window,
 * else the newest session's own reading, else why there is none. A window
 * used up while its credits carry the work on says so ("Using credits ·
 * 14,150 left", 0.3.2), never only "0% left". */
export function runwayOf(
  display: QuotaDisplayData | undefined,
  sessions: InstanceInfo[] = [],
): { left: number | null; words: string; resets: string | null; resetsAtMs?: number | null; credits?: CreditsCover | null } {
  // A tool that doesn't report its limits, or has no source: its reason,
  // never a window's numbers.
  if (display?.unavailable_reason) return { left: null, words: display.unavailable_reason, resets: null };
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
    const credits = usingCredits(display);
    return {
      left,
      words: credits ? usingCreditsWords(credits) : `${left}% left ${tightest.phrase}${modelWeekNote(display, left)}`,
      resets: words ? `${tightest.label} ${words}` : null,
      resetsAtMs: tightest.window.resets_at_ms ?? null,
      credits,
    };
  }
  const reading = sessionFallback(display, sessions);
  if (reading) {
    const left = Math.max(0, Math.round(100 - reading.used_percentage));
    return {
      left,
      words: `${left}% left, from its latest session`,
      resets: reading.resets_at_ms != null ? resetsLead(reading.resets_at_ms) : null,
      resetsAtMs: reading.resets_at_ms ?? null,
    };
  }
  if (windows.some((entry) => entry.window?.state === "learning")) {
    return { left: null, words: "Measuring — shows after a little use", resets: null };
  }
  const reset = windows.find((entry) => entry.window?.state === "reset");
  if (reset) {
    // The last reading, as history ("Last reading 13 h ago: 40% used; …").
    return { left: null, words: windowView(reset.window).words, resets: null };
  }
  // The 0.2 walk (F6): while the first read runs, not measured may only
  // be not read yet.
  if (display?.history_reading) return { left: null, words: READING_LIMITS, resets: null };
  const used = usedTokensWords(windows.map((entry) => entry.window));
  return { left: null, words: `Not measured yet${used ? ` · used ${used}` : ""}`, resets: null };
}

/** A week on one model only limits that model alone, so it never sets an
 * account's room (a session on another model doesn't touch it, and which
 * model new work will use isn't known). Only when it's tighter than the
 * account's own figure and low does the line say so ("· Fable: 10% left
 * this week"), so a pick for new work is never blind to it. */
function modelWeekNote(display: QuotaDisplayData | undefined, accountLeft: number): string {
  const low = (display?.model_weeks ?? [])
    .filter((entry): entry is typeof entry & { window: WindowDisplay & { percentage: number } } => measured(entry.window))
    .map((entry) => ({ model: entry.model, left: Math.max(0, Math.round(100 - entry.window.percentage)) }))
    .filter((entry) => entry.left <= LOW_LEFT && entry.left < accountLeft)
    .sort((a, b) => a.left - b.left)[0];
  return low ? ` · ${low.model}: ${low.left}% left this week` : "";
}

/** Where to start new work, in words, from the one runway (QA batch 8:
 * Home recommended an account with 6% left because the others weren't
 * measured). `best` is an account with room, never a low one; `words`
 * says so, or why it can't tell. Signed-out accounts can't take work. */
export function newWorkAdvice(
  accounts: AccountInfo[],
  displays: QuotaDisplayData[],
  sessions: InstanceInfo[],
): { best: { account: AccountInfo; left: number } | null; words: string | null } {
  const rows = accounts
    .filter((account) => signedIn(account.login_state))
    .map((account) => {
      const run = runwayOf(displays.find((d) => d.account === account.id), sessions.filter((s) => s.account_id === account.id));
      return { account, left: run.left, credits: run.credits ?? null };
    });
  // The one order by room (`lib/room`): the switch and New session rank
  // and suggest by the same rule.
  const ranked = rankByRoom(rows, (row) => row.left, undefined, (row) => row.credits !== null);
  const first = ranked.find((row) => row.left !== null);
  if (!first) return { best: null, words: null };
  const top = { account: first.item.account, left: first.left! };
  const unmeasured = ranked.filter((row) => row.left === null).map((row) => row.item.account.label);
  // Who isn't measured, by name when it's one ("Personal isn't measured yet").
  const unknown =
    unmeasured.length === 1 ? `${unmeasured[0]} isn't measured yet` : `${unmeasured.length} others aren't measured yet`;
  const name = top.account.label;
  if (first.suggested) {
    return {
      best: top,
      words: unmeasured.length === 0 ? `Best for new work: ${name}, ${top.left}% left.` : `${name} has ${top.left}% left; ${unknown}.`,
    };
  }
  // 0.3.2: none has room in its window; the first goes on with credits.
  if (first.kind === "credits" && unmeasured.length === 0) {
    return { best: null, words: `No account has room in its window: ${name} is using its credits.` };
  }
  return {
    best: null,
    words:
      unmeasured.length === 0
        ? `Every account is low: ${name} has the most, ${top.left}% left.`
        : `${name} has ${top.left}% left, and ${unknown}, so AI Juggler can't tell which has room.`,
  };
}

/** The account's credits in words, with who reported them and when,
 * when its tool reports them ("Credits: 16.25 · reported by Codex, as of
 * 12 min ago"); while they carry a used-up window on, it says so ("Using
 * credits · 14,150 left · reported by …", 0.3.2). */
export function creditsWords(display: QuotaDisplayData | undefined): string | null {
  const credits = display?.credits;
  if (credits == null) return null;
  const by = `reported by ${credits.source}, ${asOfWords(credits.as_of_ms)}`;
  const using = usingCredits(display);
  if (using) return `${usingCreditsWords(using)} · ${by}`;
  const number = credits.balance != null && credits.balance.trim() !== "" ? Number(credits.balance) : NaN;
  const amount = credits.unlimited ? "unlimited" : Number.isFinite(number) ? number.toLocaleString("en-US", { maximumFractionDigits: 2 }) : credits.balance;
  if (amount == null) return null;
  return `Credits: ${amount} · ${by}`;
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
  none: "text-muted",
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
            <span className="text-label font-medium text-ink">{line.label}</span>
            <span
              aria-hidden="true"
              className={"h-1.5 overflow-hidden rounded-full " + (line.left === null ? "border border-dashed border-hairline" : "bg-surface-2")}
            >
              <span className={"block h-full rounded-full " + BAR[line.tone]} style={{ width: `${line.left ?? 0}%` }} />
            </span>
            <span className={"text-label " + WORDS[line.tone]}>{line.words}</span>
          </div>
        ))}
      </div>
      {credits && (
        <p className={"text-label " + (usingCredits(display) ? "text-ink" : "text-muted")} data-testid="account-runway-credits">
          {credits}
        </p>
      )}
      {pace && (
        <p className="text-label text-warning" data-testid="account-runway-pace">
          {pace}
        </p>
      )}
    </div>
  );
}

/** The account's room in the one runway's words (`runwayOf`): the same
 * sentence Home shows, never a bare or defaulted number. */
export function RunwayWords({ display, sessions = [], loading = false }: { display?: QuotaDisplayData; sessions?: InstanceInfo[]; loading?: boolean }): JSX.Element {
  // The same words as Home for the same account: its sessions' newest
  // reading when the account has none, else "Not measured yet" (AJ-E1535);
  // never "Not measured yet" before the readings have answered.
  return (
    <span className="text-label text-muted" data-testid="launch-runway">
      {loading ? "Checking its room…" : runwayOf(display, sessions).words}
    </span>
  );
}
