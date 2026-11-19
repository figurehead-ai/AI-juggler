/**
 * 0.5 (D-117) — one look for what is selected, chosen or current: a
 * segment, a tab, a chip, the rail's current row, a chosen card. The
 * maintainer could not tell the chosen one on the pool canvas at a glance
 * once the calm pass made it a lighter grey (1.16:1 on the control: the
 * 0.4 review's 32 again), so:
 *
 * - an edge in `--color-selected-ring`, ≥ 3:1 off every unselected item
 *   around it (WCAG 1.4.11; 4.59:1 or more on every surface and on the
 *   filled control, both themes; `attention-tokens.test.ts` and
 *   `tests/visual/contrast.spec.ts` gate it);
 * - and a second cue, never the colour alone: the raised fill
 *   (`--color-selected`) with its words semibold in ink.
 *
 * Neutral, never the accent: the accent is for actions and for keyboard
 * focus (its ring sits outside, this edge inside), so focus and selection
 * never look alike; and selection is never loud (the attention budget
 * counts amber, red and accent fills, none of which this is).
 */

/** The header's current page (D-118's mock): its words semibold in ink
 * beside the other tabs' regular muted ones, and no box. Weight is its
 * cue that isn't a colour (WCAG 1.4.1), with aria-current; a box here
 * read as a button among words. Only the top bar's page tabs wear it:
 * every other chosen thing keeps SELECTED. */
export const NAV_CURRENT = "font-semibold text-ink";

/** A chosen option with its own words: a segment, tab, chip or pill. */
export const SELECTED = "bg-selected font-semibold text-selected-ink inset-ring-[1.5px] inset-ring-selected-ring";

/** A selected row whose words style themselves (the rail's rows set their
 * title semibold ink when selected): the fill and the edge only. */
export const SELECTED_FILL = "bg-selected inset-ring-[1.5px] inset-ring-selected-ring";

/** A chosen card that already draws a 1 px border: the border and an inner
 * ring make the 2 px solid edge the pool canvas's selected card has. */
export const SELECTED_CARD = "border-selected-ring bg-selected inset-ring-1 inset-ring-selected-ring";

/**
 * 0.4.6 (AJ-E3012, the pool screens; D-126, every making screen): a
 * chosen card — a pool's node, a template, a RadioCard — a solid 2 px
 * accent ring with a 2 px offset and a raised surface. Dashes never mean
 * selection. Its keyboard focus is its own, distinct ring (ink, further
 * out), so a focused card and a chosen one are never confused.
 */
export const CHOSEN_CARD = "bg-selected shadow-card ring-2 ring-accent ring-offset-2 ring-offset-canvas";

/** Keyboard focus on a card or its name: an ink outline, outside the ring. */
export const CARD_FOCUS = "outline-none focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ink-2";
