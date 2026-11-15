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

/** A chosen option with its own words: a segment, tab, chip or pill. */
export const SELECTED = "bg-selected font-semibold text-selected-ink inset-ring-[1.5px] inset-ring-selected-ring";

/** A selected row whose words style themselves (the rail's rows set their
 * title semibold ink when selected): the fill and the edge only. */
export const SELECTED_FILL = "bg-selected inset-ring-[1.5px] inset-ring-selected-ring";

/** A chosen card that already draws a 1 px border: the border and an inner
 * ring make the 2 px solid edge the pool canvas's selected card has. */
export const SELECTED_CARD = "border-selected-ring bg-selected inset-ring-1 inset-ring-selected-ring";
