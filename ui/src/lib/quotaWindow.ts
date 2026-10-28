/**
 * One window clock (5-hour or weekly), in words — the single rule every
 * surface uses: Home's runway, the Accounts cards, Usage, the inspector's
 * rings and the limit dialog.
 *
 * The 0.0.12 report: every actively used account read "100% left · resets
 * Wed 03:07 PM". The window had use but no measured sample, and a missing
 * percentage was read as 0% used. Now:
 * - "% left" and a filled bar come only from a percentage the backend
 *   sent (Measured, or Computed from a cap the user confirmed);
 * - without one, it says "Not measured yet", with the tokens it measured
 *   in the window when there are any;
 * - a reset time shows only when it is known: the vendor's, or a known
 *   weekly schedule. The backend sends none otherwise (P12: a time guessed
 *   from local use read "now" under steady use);
 * - a measured percentage says when it was read ("as of 12 min ago"); once
 *   the window it describes has reset with no newer reading, the window
 *   says "Not measured since the reset" — never 0%, never the old number.
 */
import type { QuotaDisplayData, WindowDisplay } from "./bridge";
import { compactCount } from "./compact";

export type WindowTone = "ok" | "low" | "out" | "none";

/** One limit an account shows, with its name on screen. `model` is set for
 * a week on one model only. */
export interface LimitWindow {
  key: string;
  label: string;
  window: WindowDisplay | null | undefined;
  omitted: string | null | undefined;
  model: string | null;
}

/** Every limit of an account, in the order every surface shows them: the
 * 5 hours, the week, then each week on one model only ("This week · Fable
 * only", the maintainer's report on 0.1.6: a team seat's only week read
 * as "no weekly limit"). With no all-models week but a model's, the
 * model's weeks stand alone: its line would only say why it's absent. */
export function limitWindows(display: QuotaDisplayData | undefined): LimitWindow[] {
  const models = display?.model_weeks ?? [];
  const week: LimitWindow = { key: "weekly", label: "This week", window: display?.weekly, omitted: display?.weekly_omitted_reason, model: null };
  return [
    { key: "five_hour", label: "5-hour window", window: display?.five_hour, omitted: display?.five_hour_omitted_reason, model: null },
    ...(display?.weekly == null && models.length > 0 ? [] : [week]),
    ...models.map((entry) => ({ key: `model:${entry.model}`, label: `This week · ${entry.model} only`, window: entry.window, omitted: null, model: entry.model })),
  ];
}

export interface WindowView {
  /** Percent left — ONLY when a percentage was measured or computed. */
  left: number | null;
  /** The line, in words. */
  words: string;
  tone: WindowTone;
  /** The reset, in words ("resets Wed 03:07 PM"); null when none is known. */
  resets: string | null;
}

/** "Wed 03:07 PM" — the clock words every window uses. */
export function resetClock(ms: number): string {
  return new Date(ms).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
}

/** "as of 12 min ago" under an hour, "as of 3:12 PM" under a day, then
 * "as of Oct 19": when a vendor's reading was taken. */
export function asOfWords(ms: number, now: number = Date.now()): string {
  const age = now - ms;
  if (age < 60_000) return "as of just now";
  if (age < 3_600_000) return `as of ${Math.round(age / 60_000)} min ago`;
  if (age < 86_400_000) return `as of ${new Date(ms).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}`;
  return `as of ${new Date(ms).toLocaleDateString([], { month: "short", day: "numeric" })}`;
}

/** "just now", "12 min ago", "13 h ago", "3 days ago": how old a reading is. */
export function agoWords(ms: number, now: number = Date.now()): string {
  const age = Math.max(0, now - ms);
  if (age < 60_000) return "just now";
  if (age < 3_600_000) return `${Math.round(age / 60_000)} min ago`;
  if (age < 86_400_000) return `${Math.round(age / 3_600_000)} h ago`;
  const days = Math.round(age / 86_400_000);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}

/** When a reading was taken, with its source when the backend named one:
 * "from the Claude app, 55 min ago", else "as of 12 min ago". */
export function readingWords(window: WindowDisplay, now: number = Date.now()): string | null {
  if (window.measured_at_ms == null) return null;
  return window.source_words ? `from ${window.source_words}, ${agoWords(window.measured_at_ms, now)}` : asOfWords(window.measured_at_ms, now);
}

/** When an account runs out at this pace, with its basis — or that there
 * isn't enough use to say, never "≈0 min" (QA, P26: a null forecast read
 * as "exhausts ≈0 min · Estimate" on every row). */
export function forecastWords(tteMin: number | null, confidence: string): string {
  return tteMin != null ? `exhausts ≈${Math.round(tteMin)} min · ${confidence}` : "Not enough use to forecast";
}

/** The reset in words, when one is known. */
export function resetWords(window: WindowDisplay | null | undefined): string | null {
  if (window?.resets_at_ms == null) return null;
  return `resets ${resetClock(window.resets_at_ms)}`;
}

/** 12,345 → "12k", 1,234,567 → "1.2M", 10,329,000,000 → "10.3B" (tokens used in the window). */
export function tokenWords(tokens: number): string {
  // No decimal for thousands, nor from 10M (or 100B) on.
  return compactCount(tokens, (value, suffix) => (suffix === "k" || (suffix === "M" ? value >= 10 : value >= 100) ? 0 : 1));
}

/** A window's tokens as every screen counts them: in and out, like
 * Usage's "Tokens in and out" (the 0.2 walk, F15: 7.7B on a session page
 * against 4.4M on Usage for one account). A reading without that split
 * counts cache too, and says so. `null` when it used none. */
export function usedTokensWords(windows: ReadonlyArray<WindowDisplay | null | undefined>): string | null {
  const io = windows.filter((w): w is WindowDisplay => w != null);
  if (io.length === 0) return null;
  if (io.every((w) => w.burned_io_tokens != null)) {
    const most = Math.max(...io.map((w) => w.burned_io_tokens!));
    return most > 0 ? `${tokenWords(most)} tokens in and out` : null;
  }
  const most = Math.max(...io.map((w) => w.burned_tokens));
  return most > 0 ? `${tokenWords(most)} tokens, cache included` : null;
}

/** True when the window carries a real percentage (never a default). */
export function measured(window: WindowDisplay | null | undefined): window is WindowDisplay & { percentage: number } {
  return (
    window != null &&
    typeof window.percentage === "number" &&
    Number.isFinite(window.percentage) &&
    window.state !== "unknown" &&
    window.state !== "learning" &&
    window.state !== "unmeasured" &&
    window.state !== "reset"
  );
}

/** One window, in words. `omitted` is the backend's reason when this
 * plan has no such window. */
export function windowView(window: WindowDisplay | null | undefined, omitted?: string | null): WindowView {
  if (!window) return { left: null, words: omitted ?? "Not measured yet", tone: "none", resets: null };
  if (window.state === "learning") {
    return { left: null, words: "Measuring — shows after a little use", tone: "none", resets: null };
  }
  if (window.state === "reset") {
    // The last reading stays as history only, never as the window now.
    if (window.last_used_pct != null && window.measured_at_ms != null) {
      const from = window.source_words ? ` (${window.source_words})` : "";
      return {
        left: null,
        words: `Last reading ${agoWords(window.measured_at_ms)}${from}: ${Math.round(window.last_used_pct)}% used; that window has reset since`,
        tone: "none",
        resets: null,
      };
    }
    return { left: null, words: "Not measured since the reset", tone: "none", resets: null };
  }
  const resets = resetWords(window);
  const reading = readingWords(window);
  const asOf = reading ? ` · ${reading}` : "";
  if (window.state === "exhausted") {
    // Claude Code's own record of hitting the limit: when, and the reset.
    if (window.source === "api_retry" && window.measured_at_ms != null) {
      const hit = new Date(window.measured_at_ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      return { left: 0, words: `Hit its limit at ${hit}${resets ? ` · ${resets}` : ""}`, tone: "out", resets };
    }
    const until = window.resets_at_ms != null ? ` until ${resetClock(window.resets_at_ms)}` : "";
    return { left: 0, words: `Used up${until}${asOf}`, tone: "out", resets };
  }
  if (!measured(window)) {
    const tokens = usedTokensWords([window]);
    const used = tokens ? ` · used ${tokens}` : "";
    const when = window.state === "unknown" ? "" : resets ? ` · ${resets}` : "";
    return { left: null, words: `Not measured yet${used}${when}`, tone: "none", resets };
  }
  const left = Math.max(0, Math.round(100 - window.percentage));
  if (left === 0) {
    return { left: 0, words: `Used up${window.resets_at_ms != null ? ` until ${resetClock(window.resets_at_ms)}` : ""}${asOf}`, tone: "out", resets };
  }
  // The "as of" says how old a reading is; without one, stale says so.
  const stale = !asOf && window.state === "stale" ? " · last known" : "";
  return {
    left,
    words: `${left}% left${resets ? ` · ${resets}` : ""}${asOf}${stale}`,
    tone: left <= 25 ? "low" : "ok",
    resets,
  };
}
