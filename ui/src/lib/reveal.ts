/**
 * The aij-test proof: at 1761×1348 the delete-pool confirmation opened at
 * the panel's foot, clipped, its buttons out of reach. A confirmation (or
 * any part that appears in a scrolling panel) is brought into view inside
 * its own scrolling list, never by moving the window, and its safe choice
 * (`data-autofocus`, else its first button) takes focus.
 */
import { useLayoutEffect, type RefObject } from "react";

/** The nearest ancestor that scrolls vertically. */
function scroller(element: HTMLElement): HTMLElement | null {
  const marked = element.closest<HTMLElement>("[data-canvas-scroll]");
  if (marked) return marked;
  for (let at = element.parentElement; at; at = at.parentElement) {
    const overflow = getComputedStyle(at).overflowY;
    if ((overflow === "auto" || overflow === "scroll") && at.scrollHeight > at.clientHeight) return at;
  }
  return null;
}

/** Scrolls `element` fully into its list's view (a margin around it). */
export function revealInList(element: HTMLElement, margin = 12): void {
  const list = scroller(element);
  if (!list) return;
  const it = element.getBoundingClientRect();
  const room = list.getBoundingClientRect();
  if (it.bottom > room.bottom - margin) list.scrollTop += it.bottom - room.bottom + margin;
  else if (it.top < room.top + margin) list.scrollTop -= room.top + margin - it.top;
}

/** While `on`, `ref`'s element is in its list's view and its safe choice
 * has focus. */
export function useReveal(ref: RefObject<HTMLElement | null>, on = true): void {
  useLayoutEffect(() => {
    const element = ref.current;
    if (!on || !element) return;
    revealInList(element);
    (element.querySelector<HTMLElement>("[data-autofocus]") ?? element.querySelector<HTMLElement>("button:not(:disabled)"))?.focus({ preventScroll: true });
  }, [ref, on]);
}
