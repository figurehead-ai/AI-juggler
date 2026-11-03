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
  return /window not found|not running|didn't bring/i.test(outcome);
}

export function goToToast(outcome: string): void {
  showToast({ id: "go-to", message: outcome, tone: outcome === FOCUSED ? "success" : "warning" });
}
