/**
 * 0.4.6 (AJ-E3012): one selection treatment on the pool screens — the
 * chosen card's accent ring over a raised surface, and keyboard focus as
 * its own ink ring further out. D-126 moved both to the shared controls
 * (`CHOSEN_CARD`, `CARD_FOCUS`), so every making screen's cards share
 * them; the pool's names stay here. The inspector's title edge carries
 * the same accent as the selected card.
 *
 * The raised surface is 0.5's `--color-selected` (D-117, cherry-picked);
 * the ring stays the accent, as the maintainer's complaint asked.
 */
export { CARD_FOCUS as FOCUS_RING, CHOSEN_CARD as SELECTED } from "../../components/controls/selected";

/** The inspector's title edge when something is selected (the link). */
export const SELECTED_EDGE = "shadow-[inset_3px_0_0_var(--color-accent)]";
