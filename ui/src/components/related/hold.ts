/**
 * E31 (AJ-E3103; relations-map §7.4 "Counts change while a page is
 * open") — a list never jumps while you read it. When it is read again
 * (a session started, changed state or ended), the rows you see keep
 * their places and only their words change; a row that would come in
 * above what you see waits behind "N new"; one that moved to another
 * section stays put until you ask for the order again. A row that is gone
 * leaves, and the rows under it close up (D-47's rule for the rail). A
 * row that arrives below everything you see (the next page) just joins
 * the end. Pure, so it is unit-tested.
 */

export interface Held<B extends string> {
  /** The keys shown, in the order shown. */
  keys: string[];
  /** Each shown row's section when it was placed (its heading). */
  bands: Map<string, B>;
  /** Rows read that wait behind "N new". */
  waiting: Set<string>;
  /** Shown rows now in another section ("Show in order"). */
  moved: number;
}

/** The first hold of a list: everything as read. */
export function holdAll<B extends string>(keys: readonly string[], bandOf: (key: string) => B): Held<B> {
  return { keys: [...keys], bands: new Map(keys.map((key) => [key, bandOf(key)])), waiting: new Set(), moved: 0 };
}

/**
 * The list as read again (`fresh`, in the backend's order) against what
 * is shown (`held`). `bandOf` says a fresh row's section.
 */
export function holdAgain<B extends string>(held: Held<B>, fresh: readonly string[], bandOf: (key: string) => B): Held<B> {
  const present = new Set(fresh);
  const shown = new Set(held.keys);
  // Where the last row still shown sits in the fresh order: anything
  // after it is below what you see.
  let lastShown = -1;
  fresh.forEach((key, at) => {
    if (shown.has(key)) lastShown = at;
  });
  const keys = held.keys.filter((key) => present.has(key));
  const bands = new Map<string, B>();
  for (const key of keys) bands.set(key, held.bands.get(key) ?? bandOf(key));
  const waiting = new Set([...held.waiting].filter((key) => present.has(key)));
  fresh.forEach((key, at) => {
    if (shown.has(key) || waiting.has(key)) return;
    if (at > lastShown) {
      keys.push(key);
      bands.set(key, bandOf(key));
    } else waiting.add(key);
  });
  const moved = keys.filter((key) => bands.get(key) !== bandOf(key)).length;
  return { keys, bands, waiting, moved };
}

/** Words for the button that shows the list in order again: "3 new", or
 * "Show in order" when only sections changed; `null` when nothing waits. */
export function waitingWords(held: Pick<Held<string>, "waiting" | "moved">): string | null {
  if (held.waiting.size > 0) return `${held.waiting.size} new`;
  if (held.moved > 0) return "Show in order";
  return null;
}
