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
 * - a reset time is stated as such only when it is the vendor's (or a
 *   known weekly schedule); an estimate says so.
 */
import type { WindowDisplay } from "./bridge";

export type WindowTone = "ok" | "low" | "out" | "none";

export interface WindowView {
  /** Percent left — ONLY when a percentage was measured or computed. */
  left: number | null;
  /** The line, in words. */
  words: string;
  tone: WindowTone;
  /** The reset, in words, with its basis ("resets Wed 03:07 PM", or
   * "resets about Wed 03:07 PM, estimated"); null when there is none. */
  resets: string | null;
}

/** "Wed 03:07 PM" — the clock words every window uses. */
export function resetClock(ms: number): string {
  return new Date(ms).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
}

/** The reset in words, labelled with where it came from. */
export function resetWords(window: WindowDisplay | undefined): string | null {
  if (!window?.resets_at_ms) return null;
  const at = resetClock(window.resets_at_ms);
  return window.reset_basis === "estimated" ? `resets about ${at}, estimated` : `resets ${at}`;
}

/** 12,345 → "12k", 1,234,567 → "1.2M" (tokens used in the window). */
export function tokenWords(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(tokens >= 10_000_000 ? 0 : 1)}M`;
  if (tokens >= 1_000) return `${Math.round(tokens / 1_000)}k`;
  return String(tokens);
}

/** True when the window carries a real percentage (never a default). */
export function measured(window: WindowDisplay | undefined): window is WindowDisplay & { percentage: number } {
  return (
    window !== undefined &&
    typeof window.percentage === "number" &&
    Number.isFinite(window.percentage) &&
    window.state !== "unknown" &&
    window.state !== "learning" &&
    window.state !== "unmeasured"
  );
}

/** One window, in words. `omitted` is the backend's reason when this
 * plan has no such window. */
export function windowView(window: WindowDisplay | undefined, omitted?: string): WindowView {
  if (!window) return { left: null, words: omitted ?? "Not measured yet", tone: "none", resets: null };
  if (window.state === "learning") {
    return { left: null, words: "Measuring — shows after a little use", tone: "none", resets: null };
  }
  const resets = resetWords(window);
  if (window.state === "exhausted") {
    const until = window.resets_at_ms && window.reset_basis !== "estimated" ? ` until ${resetClock(window.resets_at_ms)}` : "";
    return { left: 0, words: `Used up${until}`, tone: "out", resets };
  }
  if (!measured(window)) {
    const used = window.burned_tokens > 0 ? ` · ${tokenWords(window.burned_tokens)} tokens used` : "";
    const when = window.state === "unknown" ? "" : resets ? ` · ${resets}` : "";
    return { left: null, words: `Not measured yet${used}${when}`, tone: "none", resets };
  }
  const left = Math.max(0, Math.round(100 - window.percentage));
  if (left === 0) {
    return { left: 0, words: `Used up${window.resets_at_ms ? ` until ${resetClock(window.resets_at_ms)}` : ""}`, tone: "out", resets };
  }
  const stale = window.state === "stale" ? " · last known" : "";
  return {
    left,
    words: `${left}% left${resets ? ` · ${resets}` : ""}${stale}`,
    tone: left <= 25 ? "low" : "ok",
    resets,
  };
}
