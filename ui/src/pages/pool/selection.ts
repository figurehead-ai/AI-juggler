/**
 * 0.4.6 (AJ-E3012): one selection treatment on the pool screens — a solid
 * 2 px accent ring with a 2 px offset, and a raised surface. Dashes never
 * mean selection. Keyboard focus is its own, distinct ring (ink, further
 * out), so a focused card and a selected one are never confused. The
 * inspector's title edge carries the same accent as the selected card.
 *
 * (0.5 may add --color-selected; until then the accent token is it.)
 */

/** A selected card: the ring, its offset over the page, a raised surface. */
export const SELECTED = "bg-surface-3 shadow-card ring-2 ring-accent ring-offset-2 ring-offset-canvas";

/** Keyboard focus on a card or its name: an ink outline, outside the ring. */
export const FOCUS_RING = "outline-none focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ink-2";

/** The inspector's title edge when something is selected (the link). */
export const SELECTED_EDGE = "shadow-[inset_3px_0_0_var(--color-accent)]";
