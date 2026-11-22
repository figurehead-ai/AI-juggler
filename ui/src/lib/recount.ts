/**
 * SK48-14 (the 0.4.8 soak, PD4): after a stop, every count re-reads.
 * A stop's sheet re-read its list, but the Processes tile, the top bar's
 * count and Needs you waited for the backend's next change event, which a
 * window WebKit counts as hidden (covered by other windows) doesn't hear:
 * the list said "Nothing needs you" beside "1 to clean up". A stop, a
 * clean-up or a mark asks for a recount now and once more a tick later
 * (the graph is built on the 2 s tick), so every count reads the one
 * graph the list read.
 */

const RECOUNT = "aij:recount";

/** How long after a stop the second recount comes: one graph tick on. */
export const RECOUNT_AGAIN_MS = 2_500;

/** Every count re-reads now, and once more a tick later. */
export function askRecount(): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new Event(RECOUNT));
  window.setTimeout(() => window.dispatchEvent(new Event(RECOUNT)), RECOUNT_AGAIN_MS);
}

/** Runs `read` on each recount; returns the stop. */
export function onRecount(read: () => void): () => void {
  if (typeof window === "undefined") return () => undefined;
  window.addEventListener(RECOUNT, read);
  return () => window.removeEventListener(RECOUNT, read);
}
