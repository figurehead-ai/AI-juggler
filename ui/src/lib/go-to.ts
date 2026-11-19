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
import { listen } from "./ipc/transport";
import { placesList, placesRestart } from "./ipc/places";
import { refreshKnownPlaces } from "./places";
import { appName, desktopProfile } from "./app-name";
import { knownAppWord } from "./rail-apps";
import type { GoToOutcomeInfo } from "../types/generated/GoToOutcomeInfo";
import { goToToast } from "./go-to-words";

/** What a restart says before its answer. */
const RESTARTING = "Restarting it…";

/** 0.4.6 (the rc3c walk): what a Go to it or an Open did, when it came
 * after the 1.5 s it answers in (a profile's start). "Starting Claude app
 * · aij-test… the session opens once it's up" stayed for good, even
 * after the app had quit: what happened was never said. */
export const GO_TO_LATE = "go-to://late";

/** How long a pending answer waits for what happened: a start, then the
 * session's link sent again for up to 30 s (open_session). */
export const LATE_FOR = 45;

/** The pending answer the "go-to" toast shows, by its ticket; any newer
 * Go to it, Open or Restart takes its place. */
let waiting: { ticket: string; again?: () => Promise<GoToOutcomeInfo> } | null = null;
let hearing: Promise<unknown> | null = null;

function waitFor(ticket: string, words: string, again?: () => Promise<GoToOutcomeInfo>): void {
  waiting = { ticket, again };
  showToast({ id: "go-to", message: words, testId: "go-to-pending", seconds: LATE_FOR });
  hearing ??= listen<GoToOutcomeInfo>(GO_TO_LATE, (event) => heardLate(event.payload)).catch(() => (hearing = null));
}

/** What happened, for the pending answer with its ticket; another one's
 * (a Go to it since) is old news. */
export function heardLate(answer: GoToOutcomeInfo): void {
  if (waiting === null || answer.ticket !== waiting.ticket) return;
  showGoTo(answer, waiting.again);
}

/** Says what Open or Go to it did; offers Restart for an app that lost its
 * window. 0.4.1: when opening the session itself needs macOS to ask once
 * (`reached: "consent"`), offers "Open the session itself": only that
 * click (`again`) may let macOS ask. */
export function showGoTo(outcome: GoToOutcomeInfo, again?: () => Promise<GoToOutcomeInfo>): void {
  waiting = null;
  if (outcome.reached === "pending" && outcome.ticket) {
    waitFor(outcome.ticket, outcome.outcome, again);
    return;
  }
  if (outcome.reached === "consent" && again) {
    showToast({
      id: "go-to",
      message: outcome.outcome,
      tone: "warning",
      action: {
        label: "Open the session itself",
        run: () => {
          showToast({ id: "go-to", message: "Opening the session…" });
          again().then(
            (next) => showGoTo(next),
            (cause) => showToast({ id: "go-to", message: `Couldn't open it: ${errorText(cause)}`, tone: "error" }),
          );
        },
      },
      testId: "go-to-consent",
    });
    return;
  }
  if (outcome.reached === "started" && outcome.place_id) {
    followStart(outcome.place_id, outcome.outcome);
    return;
  }
  const placeId = outcome.reached === "lost" ? outcome.place_id : undefined;
  if (!placeId) {
    // 0.4: a partial outcome is a warning, a failure an error (go-to-words).
    goToToast(outcome.outcome, outcome.reached);
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

/** Restarts the remembered app or profile `placeId` and says how it went.
 * `confirmed`: a running one the user chose to restart (D-71). */
export function restartPlace(placeId: string, confirmed = false): void {
  waiting = null;
  showToast({ id: "go-to", message: RESTARTING });
  // Its process before: the follow waits for a new one (the go sitting:
  // Restart ended on "Restarting…" with no "is running", as Start has).
  const before = placesList()
    .then((rows) => rows.find((row) => row.id === placeId)?.app_pid ?? null)
    .catch(() => null);
  placesRestart(placeId, confirmed).then(
    (outcome) => {
      if (outcome.reached === "started" && outcome.place_id) {
        const id = outcome.place_id;
        void before.then((pid) => followStart(id, outcome.outcome, pid));
        return;
      }
      showToast({ id: "go-to", message: outcome.outcome, tone: outcome.reached === "none" ? "warning" : "neutral" });
    },
    (cause) => showToast({ id: "go-to", message: `Couldn't restart it: ${errorText(cause)}`, tone: "error" }),
  );
}

/** How often, and how long, a start is followed (the go sitting, finding
 * 3: "Starting Claude app · aij-test" stayed for minutes, the row on
 * Start, though it was up in seconds). */
const FOLLOW_EVERY_MS = 1_000;
export const FOLLOW_FOR = 30;

/** Follows a start: says "Starting …" until the app or profile runs (a
 * restart: under a process other than `oldPid`), then "… is running" (its
 * row reads the places again, so Start becomes Open), or after 30 s that
 * it didn't start. */
function followStart(placeId: string, starting: string, oldPid: number | null = null): void {
  // It stays while it's followed (6 s by default: the rerun saw it go
  // at 20 s with nothing said).
  showToast({ id: "go-to", message: starting, testId: "go-to-starting", seconds: FOLLOW_FOR + 5 });
  let tries = 0;
  const tick = async () => {
    tries += 1;
    const rows = await placesList().catch(() => null);
    const row = rows?.find((one) => one.id === placeId);
    const name = row ? appName(knownAppWord(row), desktopProfile(row.instance, row.profile)) : "It";
    if (row && row.app_pid !== null && row.app_pid !== oldPid) {
      void refreshKnownPlaces();
      showToast({ id: "go-to", message: `${name} is running`, tone: "success", testId: "go-to-started" });
      return;
    }
    if (tries >= FOLLOW_FOR) {
      void refreshKnownPlaces();
      showToast({ id: "go-to", message: `${name} didn't start`, detail: row?.cannot_start ?? "It isn't running 30 seconds after Start. Try Start again, or open it yourself.", tone: "error", testId: "go-to-not-started" });
      return;
    }
    window.setTimeout(() => void tick(), FOLLOW_EVERY_MS);
  };
  window.setTimeout(() => void tick(), FOLLOW_EVERY_MS);
}
