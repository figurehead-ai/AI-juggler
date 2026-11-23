/**
 * AJ-E1550 requirement 5 (PAT-COLOR, DESIGN-SYSTEM §5.13): a runway bar's
 * colour follows its state, the same on every screen (Home, the Accounts
 * and Usage cards, a session's page, the rows' quota bars). Calm is not
 * monochrome (the maintainer: "Choose select colours and keep using them
 * to always mean one same thing"):
 *
 * - healthy (more than half left): green, `success-strong`;
 * - in between: a soft teal, `live-strong` at half strength;
 * - low (at or under `LOW_LEFT`, 25 %: the one rule that also stops an
 *   account being suggested and counts it Low): amber, `warning-strong`;
 * - out: a red bar the whole width with the "Out" chip beside it
 *   (`RoomChip`), and Out still counts in the header;
 * - unknown or not measured: no fill, a dashed neutral track and words.
 *
 * Colour is never alone: the words ("58% left", "Low · 12% left", "Out ·
 * resets Thu 9:00 AM") always go with it.
 */
import { LOW_LEFT } from "./room";

/** More than this much left is healthy (green). */
export const HEALTHY_LEFT = 50;

export type RunwayState = "healthy" | "mid" | "low" | "out" | "unknown";

/** The state of `left` % left; `credits` carry the work on past a used-up
 * window, so that one is low, not out. */
export function runwayState(left: number | null, credits = false): RunwayState {
  if (left === null) return "unknown";
  if (left <= 0) return credits ? "low" : "out";
  if (left <= LOW_LEFT) return "low";
  return left > HEALTHY_LEFT ? "healthy" : "mid";
}

/** The bar's fill: the state's strong step (PAT-COLOR); in between, teal
 * at a soft step (the strong teal at half strength). */
export const RUNWAY_FILL: Record<RunwayState, string> = {
  healthy: "bg-success-strong",
  mid: "bg-live-strong/50",
  low: "bg-warning-strong",
  out: "bg-danger-strong",
  unknown: "bg-transparent",
};

/** The bar's track: a neutral groove; unknown, a dashed outline of it. */
export function runwayTrack(state: RunwayState): string {
  return state === "unknown" ? "border border-dashed border-faint/60" : "bg-surface-3";
}

/** How wide the fill is drawn: its % left, and Out the whole width (a red
 * bar, never an empty groove that reads as nothing). */
export function runwayWidth(left: number | null, state: RunwayState): number {
  return state === "out" ? 100 : Math.max(0, Math.min(100, left ?? 0));
}
