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
import type { WindowDisplay } from "./bridge";

export type WindowTone = "ok" | "low" | "out" | "none";

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
  if (tokens >= 1_000_000_000) return `${(tokens / 1_000_000_000).toFixed(tokens >= 100_000_000_000 ? 0 : 1)}B`;
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(tokens >= 10_000_000 ? 0 : 1)}M`;
  if (tokens >= 1_000) return `${Math.round(tokens / 1_000)}k`;
  return String(tokens);
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
    return { left: null, words: "Not measured since the reset", tone: "none", resets: null };
  }
  const resets = resetWords(window);
  const asOf = window.measured_at_ms != null ? ` · ${asOfWords(window.measured_at_ms)}` : "";
  if (window.state === "exhausted") {
    const until = window.resets_at_ms != null ? ` until ${resetClock(window.resets_at_ms)}` : "";
    return { left: 0, words: `Used up${until}${asOf}`, tone: "out", resets };
  }
  if (!measured(window)) {
    const used = window.burned_tokens > 0 ? ` · ${tokenWords(window.burned_tokens)} tokens used` : "";
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
