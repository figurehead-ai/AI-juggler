/**
 * Back and Forward (walk A3): the pages the window went through, as a
 * browser keeps them. Every name opens its page (AJ-E3206), so one click
 * can take you far from where you were; ⌘[ and the ‹ in the top bar
 * bring you back, ⌘] and › forward again. Pure, so it's tested alone; the
 * shell keeps it (shell/use-navigation).
 */

/** At most this many pages each way. */
export const HISTORY_LIMIT = 50;

export interface NavHistory<Page> {
  /** The pages before this one, oldest first. */
  back: Page[];
  /** The pages after it, nearest first (after a Back). */
  forward: Page[];
}

export function emptyHistory<Page>(): NavHistory<Page> {
  return { back: [], forward: [] };
}

/** `left` was the page and another opened: it joins Back, and Forward
 * ends (as in a browser). */
export function visit<Page>(history: NavHistory<Page>, left: Page): NavHistory<Page> {
  return { back: [...history.back, left].slice(-HISTORY_LIMIT), forward: [] };
}

/** One page back from `here`: where to go, and the history after it;
 * `null` with nothing behind. */
export function goBack<Page>(history: NavHistory<Page>, here: Page): { to: Page; history: NavHistory<Page> } | null {
  const to = history.back[history.back.length - 1];
  if (to === undefined) return null;
  return { to, history: { back: history.back.slice(0, -1), forward: [here, ...history.forward].slice(0, HISTORY_LIMIT) } };
}

/** One page forward from `here`; `null` with nothing ahead. */
export function goForward<Page>(history: NavHistory<Page>, here: Page): { to: Page; history: NavHistory<Page> } | null {
  const to = history.forward[0];
  if (to === undefined) return null;
  return { to, history: { back: [...history.back, here].slice(-HISTORY_LIMIT), forward: history.forward.slice(1) } };
}
