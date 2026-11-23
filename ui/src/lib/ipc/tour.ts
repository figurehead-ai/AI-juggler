/**
 * AJ-E1546 — the app menu's Help › Take the tour: the backend says when
 * it was chosen (`app://take-tour`); the window starts the tour. The
 * preview has no app menu (your menu's Take the tour starts it there).
 */
import { listen } from "./transport";
import { hasLiveBackend } from "../bridge";

const TAKE_TOUR_ASKED = "app://take-tour";

/** Calls `on` when Help › Take the tour is chosen. Returns the stop. */
export function onTakeTourAsked(on: () => void): () => void {
  if (!hasLiveBackend()) return () => {};
  let disposed = false;
  let stop: (() => void) | null = null;
  void listen<null>(TAKE_TOUR_ASKED, () => on()).then((unlisten) => {
    if (disposed) unlisten();
    else stop = unlisten;
  });
  return () => {
    disposed = true;
    stop?.();
  };
}
