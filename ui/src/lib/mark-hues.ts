/**
 * AJ-E3206 — the colours an App's and an Account's colour slots name, slot
 * by slot (the backend's `COLOUR_SLOTS` is their count), the same in both
 * themes. Measured, not judged by eye (the search and its numbers: the
 * 0.4 review of the marks):
 *
 * - Clear of the states: every mark's hue is 35° or more from danger's
 *   and live's, 30° from warning's and 20° from success's, and every mark
 *   is `STATUS_MIN_DELTA_E` or more in OKLab from every status colour in
 *   both themes (live, success, warning, danger in styles.css), so a mark
 *   never reads as a state beside one.
 * - Apart from each other: the states own most of the wheel, so the marks
 *   differ in lightness as well as hue; of the sets that allow, this one
 *   keeps its closest pair furthest apart for normal, protan and deutan
 *   vision. Even so eight can't sit far apart, so colour never carries
 *   identity alone: letters, shape and name do.
 * - Readable: each stands 3:1 or more from both themes' surfaces, and its
 *   letters are AA, white on the darker ones and dark on the lighter ones
 *   (`markInk`).
 *
 * Given out in this order: the first two Apps, the first App and the first
 * Account (slot `ACCOUNT_HUE_OFFSET`), and the first three Apps are the
 * most distinct; no green first, where it would read as done.
 */
import type { Oklch } from "./oklch";

export const MARK_COLOURS: readonly Oklch[] = [
  { l: 0.627, c: 0.16, h: 255 },
  { l: 0.549, c: 0.143, h: 130 },
  { l: 0.629, c: 0.16, h: 345 },
  { l: 0.571, c: 0.16, h: 280 },
  { l: 0.562, c: 0.16, h: 320 },
  { l: 0.619, c: 0.136, h: 115 },
  { l: 0.645, c: 0.16, h: 325 },
  { l: 0.564, c: 0.16, h: 345 },
];

/** How far (OKLab ΔE) every mark stays from every status colour. */
export const STATUS_MIN_DELTA_E = 0.12;

/** How far (in degrees) every mark's hue stays from each state's. */
export const STATUS_HUE_GAPS = { danger: 35, live: 35, warning: 30, success: 20 } as const;
