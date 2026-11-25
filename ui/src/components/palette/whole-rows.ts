/**
 * 0.5.1 F44: ⌘K's list ends on a whole row. At its cap the list cut
 * Suggested's heading in half; now its height stops at the last row that
 * shows whole, and never on a group's heading alone (a heading without
 * its first row says nothing). Taller content scrolls as before.
 */
import { useLayoutEffect, useRef, type RefObject } from "react";

export interface ListPart {
  /** Its bottom edge, from the top of the list's content. */
  bottom: number;
  /** A group's heading. */
  head: boolean;
}

/** The height that shows only whole rows within `room`, or null when
 * not even one fits (the cap stands). */
export function wholeRowsHeight(parts: readonly ListPart[], room: number): number | null {
  let end: number | null = null;
  for (const part of parts) {
    if (part.bottom > room + 0.5) break;
    if (!part.head) end = part.bottom;
  }
  return end;
}

/** A ref for a list (capped by its CSS max-height) that ends on a whole
 * row, measured after every render. Options are `[role=option]`, headings
 * carry `data-list-head`. */
export function useWholeRows<T extends HTMLElement>(): RefObject<T | null> {
  const list = useRef<T | null>(null);
  useLayoutEffect(() => {
    const el = list.current;
    if (!el) return;
    el.style.height = "";
    if (el.scrollHeight <= el.clientHeight + 1) return;
    const style = getComputedStyle(el);
    const padTop = parseFloat(style.paddingTop) || 0;
    const padBottom = parseFloat(style.paddingBottom) || 0;
    const top = el.getBoundingClientRect().top + el.clientTop - el.scrollTop;
    const parts = Array.from(el.querySelectorAll<HTMLElement>("[role='option'], [data-list-head]"), (part) => ({
      bottom: part.getBoundingClientRect().bottom - top,
      head: part.hasAttribute("data-list-head"),
    }));
    const end = wholeRowsHeight(parts, el.clientHeight - padBottom);
    if (end !== null && end > padTop) el.style.height = `${Math.ceil(end + padBottom)}px`;
  });
  return list;
}
