/**
 * D-118 (AJ-E3701 R1, R4) — Home's layout by the width of its main area:
 * two columns that scroll on their own (the feed `minmax(480px, 1fr)`
 * capped at 720 px for reading, the right column `clamp(320px, 30%,
 * 400px)`, 24 px apart); below 860 px they stack, the feed first;
 * under 640 px Your apps is a strip of icons.
 */
import { useCallback, useState } from "react";

/** Below this main-area width the columns stack (D-118: "narrow widths
 * stack"). Set just above the columns' own minimum, 824 px (480 + 24 +
 * 320), so the app's common sizes keep Your apps beside the feed: a 1024 px
 * window (the rail folds to icons below 1,100: ≈ 924 px of main area) and
 * a 1200 px one with the rail open (≈ 896). Only the narrowest windows
 * stack (0.5 session's design call, 2026-11-16). */
const STACK_BELOW = 860;
/** Below this, Your apps is a strip of icons. */
const STRIP_BELOW = 640;

export type HomeLayout = "columns" | "stacked" | "strip";

/** The layout for a main area `width` px wide (0: not measured yet, as
 * in a test's DOM: the columns). */
export function homeLayout(width: number): HomeLayout {
  if (width <= 0) return "columns";
  if (width < STRIP_BELOW) return "strip";
  return width < STACK_BELOW ? "stacked" : "columns";
}

/** The grid's columns (the ruling's widths). */
export const COLUMNS = "grid-cols-[minmax(480px,1fr)_clamp(320px,30%,400px)]";

/** Home's layout, measured on the box `ref` is given (and on resize). */
export function useHomeLayout(): [HomeLayout, (node: HTMLElement | null) => (() => void) | undefined] {
  const [layout, setLayout] = useState<HomeLayout>("columns");
  const ref = useCallback((node: HTMLElement | null) => {
    if (!node) return undefined;
    const measure = () => setLayout(homeLayout(node.getBoundingClientRect().width));
    measure();
    if (typeof ResizeObserver === "undefined") return undefined;
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  return [layout, ref];
}
