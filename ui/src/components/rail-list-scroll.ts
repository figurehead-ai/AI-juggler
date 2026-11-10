/**
 * The rail's list box (0.4.1: its one scroller, under a fixed head): its
 * scroll and view height (the list's windowing reads them), whether it's
 * scrolled down (styles.css .scroll-edge draws a hairline and a soft
 * shadow under the head only then, so a row cut at the top reads as more
 * above, not as the list's start), and the slot between the head and the
 * list where the "new above" pill takes its own space (rail-hold: it
 * pushes the list's view down while it shows, never covers a line).
 */
import { useEffect, useRef, useState, type RefObject, type UIEvent } from "react";

interface RailListScroll {
  ref: RefObject<HTMLDivElement | null>;
  top: number;
  height: number;
  /** The pill's slot (set by its ref), and its ref. */
  cueSlot: HTMLDivElement | null;
  setCueSlot: (element: HTMLDivElement | null) => void;
  /** The list box's own: its edge's flag and its scroll. */
  listProps: { "data-scrolled": "" | undefined; onScroll: (event: UIEvent<HTMLDivElement>) => void };
}

export function useRailListScroll(): RailListScroll {
  const ref = useRef<HTMLDivElement | null>(null);
  const [top, setTop] = useState(0);
  const [height, setHeight] = useState(640);
  const [cueSlot, setCueSlot] = useState<HTMLDivElement | null>(null);
  useEffect(() => {
    const element = ref.current;
    if (!element) return undefined;
    const observe = () => setHeight(element.clientHeight || 640);
    observe();
    // jsdom has no ResizeObserver; the initial measure above suffices.
    if (typeof ResizeObserver === "undefined") return undefined;
    const observer = new ResizeObserver(observe);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return {
    ref,
    top,
    height,
    cueSlot,
    setCueSlot,
    listProps: { "data-scrolled": top > 0 ? "" : undefined, onScroll: (event) => setTop(event.currentTarget.scrollTop) },
  };
}
