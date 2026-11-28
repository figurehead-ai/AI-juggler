/**
 * 0.5.2 (PR #257's menus gate, "Showing: Every app"): a menu from a button
 * that fits neither below nor above it, but fits the window beside the
 * button, gets its room: the button's scrolling area moves the button
 * (never past the window's edge) so the whole menu fits on one side. The
 * menu then never covers its button (the 0.4 design review, 19) and never
 * scrolls inside while the window could hold it all: every line takes its
 * own press. With nothing that scrolls (the rail's ⋯ on a short list), it
 * opens beside its button instead, as a submenu does. Only a menu taller
 * than the window still scrolls inside.
 */
import type { MenuAt } from "./RowMenu";

type Span = (at: number, end: number | undefined, size: number, room: number) => { start: number; max: number | null };
type Edge = (at: number, end: number | undefined, size: number, room: number) => number;

const INSET = 8;

/** The scrolling area a button sits in, if any. */
function scrollerOf(element: HTMLElement): HTMLElement | null {
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    if (parent.scrollHeight > parent.clientHeight && /(auto|scroll)/.test(getComputedStyle(parent).overflowY)) return parent;
  }
  return null;
}

/** Where the menu goes (`span`, `RowMenu`'s `anchoredSpan`), after making
 * room for it when it fits neither side of its button. */
export function withRoom(at: MenuAt, size: number, span: Span): { start: number; max: number | null } {
  const room = window.innerHeight;
  const first = span(at.y, at.above, size, room);
  const button = document.activeElement;
  if (first.max === null || at.above === undefined || !(button instanceof HTMLElement)) return first;
  const before = button.getBoundingClientRect();
  if (size + before.height + 8 > room - 2 * INSET) return first;
  const scroller = scrollerOf(button);
  if (!scroller) return first;
  // Below it when the button can rise far enough, else above it.
  const up = at.y + size - (room - INSET);
  const down = size - (at.above - INSET);
  if (up <= before.top - INSET) scroller.scrollTop += up;
  else scroller.scrollTop -= down;
  const box = button.getBoundingClientRect();
  return span(box.bottom + 4, box.top - 4, size, room);
}

/** Where a menu from `at` goes, `box` its drawn size: below or above its
 * button ([`withRoom`]), else beside it, else capped to scroll inside. */
export function placed(at: MenuAt, box: { width: number; height: number }, span: Span, edge: Edge): { left: number; top: number; maxHeight?: number } {
  const room = window.innerHeight;
  const width = window.innerWidth;
  const vertical = withRoom(at, box.height, span);
  const left = edge(at.x, at.right, box.width, width);
  if (vertical.max === null) return { left, top: vertical.start };
  const fitsBeside = at.above !== undefined && at.right !== undefined && box.height <= room - 2 * INSET;
  if (fitsBeside && at.right !== undefined && at.above !== undefined) {
    const right = at.right + 4;
    const side = right + box.width <= width - INSET ? right : at.x - 4 - box.width;
    if (side >= INSET) return { left: side, top: Math.max(INSET, Math.min(at.above + 4, room - INSET - box.height)) };
  }
  return { left, top: vertical.start, maxHeight: vertical.max };
}
