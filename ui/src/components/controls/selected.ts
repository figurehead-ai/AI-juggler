/**
 * 0.5 (D-117), AJ-E1550 (PAT-SURF, 2026-11-23) — one look for what is
 * selected, chosen or current: a segment, a tab, a chip, the rail's
 * current row, a chosen card. A soft fill and stronger words, never an
 * outline (the maintainer: "only time you can ever use a bright border is
 * on highlight or mouse over or to really really bring attention"):
 *
 * - the fill (`--color-selected`), a clear step off what is around it
 *   (1.26–1.44:1, both themes), its words semibold in ink: two cues, never
 *   the colour alone (WCAG 1.4.1);
 * - its edge (`--color-selected-edge`) is transparent, and becomes the
 *   `--color-selected-ring` (≥ 3:1, WCAG 1.4.11; `attention-tokens.test.ts`
 *   and `tests/visual/contrast.spec.ts` gate it) under Increase Contrast.
 *
 * Neutral, never the accent: the accent is for actions and for keyboard
 * focus, so focus and selection never look alike; and selection is never
 * loud (the attention budget counts amber, red and accent fills, none of
 * which this is).
 */
/** The header's current page (D-118's mock): its words semibold in ink
 * beside the other tabs' regular muted ones, and no box. Weight is its
 * cue that isn't a colour (WCAG 1.4.1), with aria-current; a box here
 * read as a button among words. Only the top bar's page tabs wear it:
 * every other chosen thing keeps SELECTED. */
export const NAV_CURRENT = "font-semibold text-ink";

/** A chosen option with its own words: a segment, tab, chip or pill. */
export const SELECTED = "bg-selected font-semibold text-selected-ink inset-ring-[1.5px] inset-ring-selected-edge";

/** A selected row whose words style themselves (the rail's rows set their
 * title semibold ink when selected): the fill (and the Increase Contrast
 * edge) only. */
export const SELECTED_FILL = "bg-selected inset-ring-[1.5px] inset-ring-selected-edge";

/** A chosen card or line that already draws a 1 px border: its border
 * fades (selected-edge) and the fill carries the choice; under Increase
 * Contrast the border and an inner ring make a 2 px solid edge. */
export const SELECTED_CARD = "border-selected-edge bg-selected inset-ring-1 inset-ring-selected-edge";

/**
 * 0.4.6 (AJ-E3012, the pool screens; D-126, every making screen): a
 * chosen card — a pool's node, a template, a RadioCard — a solid 2 px
 * accent ring with a 2 px offset and a raised surface. Dashes never mean
 * selection. AJ-E1550 (PAT-SURF): this is the one accent ring that isn't
 * focus — a highlight of what the user picked among cards in a making
 * screen (rich mode, D-126) — and it is never a recommendation's look
 * ("Suggested" is a tag) nor a list's selection (SELECTED, a fill). Its
 * keyboard focus is its own, distinct ring (ink, further out), so a
 * focused card and a chosen one are never confused.
 */
export const CHOSEN_CARD = "bg-selected shadow-card ring-2 ring-accent ring-offset-2 ring-offset-canvas";

/** Keyboard focus on a card or its name: an ink outline, outside the ring. */
export const CARD_FOCUS = "outline-none focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ink-2";
