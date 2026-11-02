/**
 * AJ-E3206 — the hues an App's and an Account's colour slots name, slot by
 * slot (the backend's `COLOUR_SLOTS` is their count). One luminance, white
 * text 4.5:1 or better on each (lib/marks draws them). The status tokens
 * sit at about 22° (danger), 84° (warning), 163° (success), 212° (live)
 * and 280° (accent); each of these is 20° or more from all of them, so a
 * mark is never read as a state.
 */
export const PROFILE_HUES = [325, 115, 245, 50, 355, 185, 140, 305] as const;

/** The status hues the mark hues keep away from. */
export const STATUS_HUES = [22, 84, 163, 212, 280] as const;
