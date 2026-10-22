/**
 * AJ-E1532 — the hues accounts are told apart by: twelve, spread so that
 * neighbours stay apart to the eye (the greens and blues, which crowd on
 * an even wheel, get fewer slots). Each key (an account's id) gets a
 * stable one, so the same account has the same colour on every screen
 * and every launch. The discs and dots that use it keep their own
 * lightness and contrast rules (styles.css `.acct-disc`, AJ-E804).
 */

/** The twelve hues, degrees (HSL), in the order accounts get them: the
 * first few far apart on the wheel, and amber and red — the app's warning
 * and error colours — last, so they only appear with eleven accounts. */
export const PALETTE_HUES = [215, 325, 170, 270, 90, 25, 195, 295, 140, 245, 45, 0] as const;

/** The user's accounts, each with its own slot (assigned by id, so the
 * same account keeps its colour while the others come and go less often
 * than a hash would collide: twelve accounts, twelve colours). */
const assigned = new Map<string, number>();

/** Give each of the user's accounts its own slot (call when the list
 * loads; cheap and idempotent). */
export function assignPalette(keys: string[]): void {
  const sorted = [...new Set(keys)].sort();
  if (sorted.length === assigned.size && sorted.every((key) => assigned.has(key))) return;
  assigned.clear();
  sorted.forEach((key, index) => assigned.set(key, index % PALETTE_HUES.length));
}

/** A stable index into the palette for `key`: its assigned slot, else a
 * hash (a key the list doesn't know yet). */
export function paletteIndex(key: string | null): number {
  const slot = key === null ? undefined : assigned.get(key);
  if (slot !== undefined) return slot;
  const text = key ?? "unknown";
  let hash = 0;
  for (let i = 0; i < text.length; i += 1) {
    hash = (hash * 31 + text.charCodeAt(i)) % 100_000;
  }
  return hash % PALETTE_HUES.length;
}

/** The palette hue for `key`. */
export function paletteHue(key: string | null): number {
  return PALETTE_HUES[paletteIndex(key)]!;
}

/** The user's accounts, as loaded: each given its slot on the way in. */
export function withPalette<T extends { id: string }>(accounts: T[]): T[] {
  assignPalette(accounts.map((account) => account.id));
  return accounts;
}
