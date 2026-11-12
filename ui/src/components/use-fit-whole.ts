/**
 * D-100 (the 0.4.2 review): the words in `ref`'s box show whole or not at
 * all. A word too wide for what's left beside an App's name was clipped
 * ("Claude ap"); now each child that doesn't fit whole is hidden, again
 * whenever the box changes width.
 */
import { useLayoutEffect, type RefObject } from "react";

export function useFitWhole(ref: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const box = ref.current;
    if (!box) return undefined;
    const fit = () => {
      const right = box.getBoundingClientRect().right;
      for (const child of Array.from(box.children)) {
        if (!(child instanceof HTMLElement)) continue;
        child.style.visibility = child.getBoundingClientRect().right <= right + 0.5 ? "" : "hidden";
      }
    };
    fit();
    if (typeof ResizeObserver === "undefined") return undefined;
    const watch = new ResizeObserver(fit);
    watch.observe(box);
    return () => watch.disconnect();
  });
}
