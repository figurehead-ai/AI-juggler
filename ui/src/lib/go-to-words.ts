/**
 * What a Go to it says when it's done (the 0.4 review): the outcome in the
 * desktop app's own words, with a tick only when the window came forward.
 * A window not found, an app not running or one macOS kept behind is a
 * warning, never a success, in a toast and on a paired Mac's actions.
 */
import { showToast } from "../components/Toast";

/** The outcome that means it worked. */
export const FOCUSED = "Focused window";

/** A Go to it that ran but didn't bring the window forward (the app
 * opened without it, isn't running, or macOS kept it behind). */
export function partialGoTo(outcome: string): boolean {
  return /window not found|not running|didn't bring|permission needed/i.test(outcome);
}

/** The steps that reach the session itself: its thread, its terminal tab
 * or its folder's window (the go sitting's rerun: "Opened the session in
 * Claude app · aij-test" wore a warning). */
const EXACT = new Set(["thread", "tab", "folder"]);

/** The toast's tone for an outcome: a tick when it reached the session or
 * the window came forward, an error when nothing happened, else a
 * warning. */
export function goToTone(outcome: string, reached?: string): "success" | "warning" | "error" {
  if (reached === "none") return "error";
  if ((reached !== undefined && EXACT.has(reached)) || outcome === FOCUSED) return partialGoTo(outcome) ? "warning" : "success";
  return "warning";
}

export function goToToast(outcome: string, reached?: string): void {
  showToast({ id: "go-to", message: outcome, tone: goToTone(outcome, reached) });
}
