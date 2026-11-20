/**
 * A small menu's focus and closing (Bot's ⋯ menu, the composer's +):
 * its first item (`first`, a selector) takes focus when it opens; Esc
 * closes it (and goes no further, so the panel under it stays open), and
 * so does a press outside it.
 */
import { useEffect, type RefObject } from "react";

export function useMenuCloses(ref: RefObject<HTMLElement | null>, onClose: () => void, first: string): void {
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>(first)?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    const onDown = (event: MouseEvent) => {
      if (!(event.target instanceof Node) || !ref.current?.contains(event.target)) onClose();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown);
    };
  }, [ref, onClose, first]);
}
