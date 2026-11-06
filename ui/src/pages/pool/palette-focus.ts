/**
 * The canvas's "Add members" step: opens the Add a session list, or, when
 * it's already open, still answers — by moving focus into it (the P10
 * walk: a click that changed nothing on screen).
 */
import type { RefObject } from "react";

export function openAndFocus(open: (open: boolean) => void, list: RefObject<HTMLElement | null>): () => void {
  return () => {
    open(true);
    requestAnimationFrame(() => list.current?.querySelector<HTMLElement>("input, button:not([disabled])")?.focus());
  };
}
