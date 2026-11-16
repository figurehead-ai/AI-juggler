/**
 * AJ-E3011 round 4 (main's ruling, app-wide): while a menu or popover is
 * open, everything else on the page is inert — out of the accessibility
 * tree and the pointer's reach — so a press meant for one of its options
 * can never land on what's underneath (in the Dev window an accessibility
 * press on "Its role" › Reviewer reached "Take out of the pool" under it).
 *
 * Every sibling of the open element and of each of its ancestors, up to
 * the body, is made inert; nested menus and popovers share it by count,
 * so closing one leaves the other's in place. A press outside still
 * reaches the window (the menus' outside-press closes them), never the
 * control under it.
 */
import { useLayoutEffect, type RefObject } from "react";

/** How many open menus and popovers made each element inert. */
const held = new Map<Element, number>();

function hold(element: Element): void {
  const count = held.get(element) ?? 0;
  if (count === 0) {
    // Inert already by its own owner (a closed panel): not ours to undo.
    if (element.hasAttribute("inert")) return;
    element.setAttribute("inert", "");
  }
  held.set(element, count + 1);
}

function release(element: Element): void {
  const count = held.get(element);
  if (count === undefined) return;
  if (count > 1) held.set(element, count - 1);
  else {
    held.delete(element);
    element.removeAttribute("inert");
  }
}

/** The elements beside `open` and beside each of its ancestors. */
export function besides(open: Element): Element[] {
  const out: Element[] = [];
  for (let at: Element | null = open; at && at !== document.body && at.parentElement; at = at.parentElement) {
    for (const sibling of at.parentElement.children) {
      // Scripts, styles and a live region announcing a change stay as they are.
      if (sibling === at || sibling.tagName === "SCRIPT" || sibling.tagName === "STYLE" || sibling.getAttribute("aria-live")) continue;
      out.push(sibling);
    }
  }
  return out;
}

/** Makes everything but `ref`'s element inert while `on`.
 *
 * Its opener (what had focus as it opened) gets focus back when it closes
 * if focus was left nowhere: a close that hands focus back in the same
 * turn (Escape: `setOpen(false); button.focus()`) does so while the
 * opener is still inert, and an inert element takes no focus, so focus
 * fell to the page (0.4.6 RC, the legend's "?" and six more). */
export function useInertOutside(ref: RefObject<HTMLElement | null>, on = true): void {
  useLayoutEffect(() => {
    const open = ref.current;
    const others = on && open ? besides(open).filter((one) => !one.contains(open)) : [];
    const focused = document.activeElement;
    const opener = others.length > 0 && focused instanceof HTMLElement && focused !== document.body && !open?.contains(focused) ? focused : null;
    for (const one of others) hold(one);
    return () => {
      for (const one of others) release(one);
      const now = document.activeElement;
      if (opener?.isConnected && (now === null || now === document.body || (open?.contains(now) ?? false))) opener.focus({ preventScroll: true });
    };
  }, [ref, on]);
}

/** Tests: whatever is held is let go. */
export function releaseAllInert(): void {
  for (const element of held.keys()) element.removeAttribute("inert");
  held.clear();
}
