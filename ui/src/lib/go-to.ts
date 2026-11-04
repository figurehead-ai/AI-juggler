/**
 * Open and Go to it answer the same way everywhere: the outcome's words in
 * the "go-to" toast. 0.3.8: when the app runs on but lost its window
 * because macOS restarted its display (`reached: "lost"`), the toast asks
 * "Restart it?" and stays until answered; Restart quits that app or
 * profile and starts it again, as it ran. Nothing is quit without that
 * click.
 */
import { showToast } from "../components/Toast";
import { errorText } from "./bridge";
import { placesRestart } from "./ipc/places";
import type { GoToOutcomeInfo } from "../types/generated/GoToOutcomeInfo";
import { goToToast } from "./go-to-words";

/** What a restart says before its answer. */
export const RESTARTING = "Restarting it…";

/** Says what Open or Go to it did; offers Restart for an app that lost its
 * window. */
export function showGoTo(outcome: GoToOutcomeInfo): void {
  const placeId = outcome.reached === "lost" ? outcome.place_id : undefined;
  if (!placeId) {
    // 0.4: a partial outcome is a warning, a failure an error (go-to-words).
    goToToast(outcome.outcome);
    return;
  }
  showToast({
    id: "go-to",
    message: outcome.outcome,
    detail: "Restarting stops anything still working in it.",
    tone: "warning",
    action: { label: "Restart", run: () => restartPlace(placeId) },
    testId: "go-to-lost",
  });
}

/** Restarts the remembered app or profile `placeId` and says how it went. */
export function restartPlace(placeId: string): void {
  showToast({ id: "go-to", message: RESTARTING });
  placesRestart(placeId).then(
    (outcome) => showToast({ id: "go-to", message: outcome.outcome, tone: outcome.reached === "none" ? "warning" : "neutral" }),
    (cause) => showToast({ id: "go-to", message: `Couldn't restart it: ${errorText(cause)}`, tone: "error" }),
  );
}
