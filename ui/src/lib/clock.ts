/**
 * The app's one clock for words that say how long ago or how long since
 * ("waiting 7 min", "6 min ago", "last open 11 min ago"): ONE shared
 * timer for the whole window (AJ-E403 Req.5, AJ-E805's idle budget),
 * never a timer per row. It ticks every 15 s while the window is shown
 * and focused, half as often while another app is in front, and not at
 * all while the window is hidden or covered; when it's shown again it
 * ticks at once (`everyWhileShown`).
 *
 * AJ-E2312 item 5 (SK48-6, the 0.4.8 soak's K3 lab): the shell's clock
 * moved the rows it was handed, but the bell's panel, a session's notices
 * and "Your apps" read their own: the bell's and the notices' times were
 * the moment they opened, and "Your apps" read the system's clock at each
 * draw ("last open 11 min ago" beside rows still "waiting 7 min"). Every
 * relative time reads this one now.
 */
import { useSyncExternalStore } from "react";
import { everyWhileShown } from "./shown-window";

/** How often it ticks while the window is shown and focused. */
export const CLOCK_EVERY_MS = 15_000;

let now = Date.now();
const listeners = new Set<() => void>();
let stop: (() => void) | null = null;

function tick(): void {
  now = Date.now();
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (stop === null) {
    // The first reader after a quiet spell reads the time as it is.
    now = Date.now();
    stop = everyWhileShown(tick, CLOCK_EVERY_MS, { unfocusedTimes: 2 });
  }
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && stop !== null) {
      stop();
      stop = null;
    }
  };
}

const snapshot = (): number => now;

/** The shared now, epoch ms: every reader moves on the same tick. */
export function useClock(): number {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}
