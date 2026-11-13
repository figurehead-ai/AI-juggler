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
import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";

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

/** The box's words as they are now: what decides what fits, besides its width. */
function wordsOf(box: HTMLElement): string {
  return Array.from(box.children, (child) => `${(child as HTMLElement).dataset.fit ?? ""}:${child.textContent ?? ""}`).join("\n");
}

/** Each child shown again, measured, and the ones that don't fit left out. */
function fit(box: HTMLElement): void {
  const kids = Array.from(box.children).filter((child): child is HTMLElement => child instanceof HTMLElement);
  // Each measured at its own width (the box's own width never depends on them: it grows from 0).
  for (const kid of kids) kid.style.display = "";
  const keep = fitting(box, kids);
  for (const kid of kids) if (!keep.has(kid)) kid.style.display = "none";
}

/** Fits `ref`'s words when they change and when the box changes width.
 * 0.4.5 (AJ-E805): it fitted them after every render, each time showing
 * every word, measuring them and hiding them again (an App's header
 * renders with every change: about 400 style writes a minute with 10
 * sessions at work, and the header drawn again for each). */
export function useFitWhole(ref: RefObject<HTMLElement | null>): void {
  const kept = useRef<{ box: HTMLElement; words: string; watch: ResizeObserver | null } | null>(null);
  useLayoutEffect(() => {
    const box = ref.current;
    if (kept.current && kept.current.box !== box) {
      kept.current.watch?.disconnect();
      kept.current = null;
    }
    if (!box) return;
    const words = wordsOf(box);
    if (kept.current?.words === words) return;
    fit(box);
    if (kept.current) kept.current.words = words;
    else {
      const watch = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(() => fit(box));
      watch?.observe(box);
      kept.current = { box, words, watch };
    }
  });
  useEffect(
    () => () => {
      kept.current?.watch?.disconnect();
      kept.current = null;
    },
    [],
  );
}
