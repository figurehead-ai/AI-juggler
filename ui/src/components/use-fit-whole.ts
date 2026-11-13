/**
 * D-100 (the 0.4.2 review): the words in `ref`'s box show whole or not at
 * all. A word too wide for what's left beside an App's name was clipped
 * ("Claude ap"); now each child that doesn't fit whole is left out, again
 * whenever the box changes width.
 *
 * 0.4.4: a child marked `data-fit="first"` is placed first, then the
 * others in their order, up to the first that doesn't fit. An App running
 * with nothing open keeps "nothing open" over its app word (its kind mark
 * and icon already say which app), and the word never stands in for it:
 * the word took the room at the rail's default 264 px. One left out takes
 * no room, so the rest close up.
 */
import { useLayoutEffect, type RefObject } from "react";

/** The children of `box` that fit, by the rule above. */
function fitting(box: HTMLElement, kids: readonly HTMLElement[]): Set<HTMLElement> {
  const room = box.getBoundingClientRect().width + 0.5;
  const gap = Number.parseFloat(getComputedStyle(box).columnGap) || 0;
  const first = (kid: HTMLElement) => kid.dataset.fit === "first";
  const keep = new Set<HTMLElement>();
  let used = 0;
  for (const kid of [...kids.filter(first), ...kids.filter((other) => !first(other))]) {
    used += (keep.size > 0 ? gap : 0) + kid.getBoundingClientRect().width;
    if (used > room) break;
    keep.add(kid);
  }
  return keep;
}

export function useFitWhole(ref: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const box = ref.current;
    if (!box) return undefined;
    const fit = () => {
      const kids = Array.from(box.children).filter((child): child is HTMLElement => child instanceof HTMLElement);
      // Each measured at its own width (the box's own width never depends on them: it grows from 0).
      for (const kid of kids) kid.style.display = "";
      const keep = fitting(box, kids);
      for (const kid of kids) if (!keep.has(kid)) kid.style.display = "none";
    };
    fit();
    if (typeof ResizeObserver === "undefined") return undefined;
    const watch = new ResizeObserver(fit);
    watch.observe(box);
    return () => watch.disconnect();
  });
}
