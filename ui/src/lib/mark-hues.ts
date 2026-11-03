/**
 * AJ-E3206 — the hues an App's and an Account's colour slots name, slot by
 * slot (the backend's `COLOUR_SLOTS` is their count). One luminance, white
 * text 4.5:1 or better on each (lib/marks draws them).
 *
 * Measured, not judged by hue angle: each mark is `STATUS_MIN_DELTA_E` or
 * more in OKLab from every status colour in both themes (live, success,
 * warning, danger in styles.css), so a mark is never read as a state; of
 * the sets that allow, this one keeps its closest pair furthest apart,
 * also as people with protanopia and deuteranopia see it. Slots are
 * given out in this order: the first two Apps, the first three, and the
 * first App and first Account (slot `ACCOUNT_HUE_OFFSET`) are the most
 * distinct. One luminance caps how far eight hues can sit apart, so
 * colour never carries identity alone: letters, shape and name do.
 */
export const PROFILE_HUES = [120, 235, 180, 325, 265, 150, 345, 5] as const;

/** How far (OKLab ΔE) every mark stays from every status colour. */
export const STATUS_MIN_DELTA_E = 0.1;
