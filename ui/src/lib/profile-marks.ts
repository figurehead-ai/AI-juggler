/**
 * AJ-E2104 — a profile's badge: a one- or two-letter monogram on its own
 * colour. The colours are the palette's kind (one luminance, white
 * letters 4.5:1 or better on each), on hues kept away from the status
 * colours (warning amber, live cyan, success green, danger red, the
 * accent violet), so a profile is never read as a state. Each profile
 * the rail knows gets its own slot, by name, whatever the order.
 */
import { distinctInitials, initialsOf, swatchOf } from "./palette";

/** The hues, in the order slots are given out. The status tokens sit at
 * about 22° (danger), 84° (warning), 163° (success), 212° (live) and
 * 280° (accent); each of these is 20° or more from all of them. */
export const PROFILE_HUES = [325, 115, 245, 50, 355, 185] as const;

/** The status hues the profile hues keep away from. */
export const STATUS_HUES = [22, 84, 163, 212, 280] as const;

const slots = new Map<string, number>();
const letters = new Map<string, string>();
let assignedKey = "";

/** Give each profile name its slot and letters (cheap and idempotent). */
export function assignProfiles(names: string[]): void {
  const sorted = [...new Set(names.filter(Boolean))].sort((a, b) => a.localeCompare(b));
  const key = sorted.join("\u0000");
  if (key === assignedKey) return;
  assignedKey = key;
  slots.clear();
  letters.clear();
  const picked = distinctInitials(sorted);
  sorted.forEach((name, index) => {
    slots.set(name, index % PROFILE_HUES.length);
    letters.set(name, picked[index]!);
  });
}

function slotOf(name: string): number {
  const slot = slots.get(name);
  if (slot !== undefined) return slot;
  let hash = 0;
  for (let i = 0; i < name.length; i += 1) hash = (hash * 31 + name.charCodeAt(i)) % 100_000;
  return hash % PROFILE_HUES.length;
}

/** The profile's colour, as CSS (the same in both themes). */
export function profileColor(name: string): string {
  const { l, c, h } = swatchOf(PROFILE_HUES[slotOf(name)]!);
  return `oklch(${l.toFixed(3)} ${c.toFixed(3)} ${h})`;
}

/** The profile's monogram: told apart from the other profiles once
 * assigned, else from its name alone. */
export function profileMonogram(name: string): string {
  return letters.get(name) ?? initialsOf(name);
}
