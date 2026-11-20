/**
 * AJ-E3011 round 5 (Sam: "the page jumped after each Add, and one click
 * seemed lost"): a list a row leaves when it's added keeps that row where
 * it was, marked added, so the next row's Add stays under the pointer.
 * The rows added here are kept by key with the place they had; they go
 * when the list is closed (its owner starts a new map).
 */

/** Where an added row was, and the row as it was. */
export interface Kept<T> {
  row: T;
  at: number;
}

/** `rows` with each kept row back at its place, unless it's still in
 * `rows` (the add hasn't landed yet). */
export function keepInPlace<T>(rows: readonly T[], kept: ReadonlyMap<string, Kept<T>>, keyOf: (row: T) => string): { row: T; added: boolean }[] {
  const present = new Set(rows.map(keyOf));
  const out: { row: T; added: boolean }[] = rows.map((row) => ({ row, added: false }));
  const back = [...kept.entries()].filter(([key]) => !present.has(key)).sort((a, b) => a[1].at - b[1].at);
  for (const [, one] of back) out.splice(Math.min(one.at, out.length), 0, { row: one.row, added: true });
  return out;
}

/** The map after adding `row`, at `at` in the list as shown. */
export function keep<T>(kept: ReadonlyMap<string, Kept<T>>, key: string, row: T, at: number): Map<string, Kept<T>> {
  return new Map(kept).set(key, { row, at });
}
