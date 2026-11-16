/**
 * AJ-E2909: the list's edge, to resize it by dragging or from the
 * keyboard (WAI-ARIA's window splitter): Tab to it, ← and → by 8 px,
 * ⇧← and ⇧→ by 32, Home the narrowest, End the widest. A screen reader
 * hears its width. Each change is kept as a drag's is (debounced).
 * Double-click still cycles the density (its keyboard twin is ⌃D). The
 * shared edge (`Splitter`), set for the rail.
 */
import type { JSX } from "react";

import { Splitter, edgeWidthForKey, readEdgeWidth, type SplitterEdge } from "./Splitter";

export const WIDTH_MIN = 208;
export const WIDTH_MAX = 360;
export const WIDTH_DEFAULT = 264;
export const STORE_WIDTH = "aij.rail.width";

const RAIL_EDGE: SplitterEdge = { min: WIDTH_MIN, max: WIDTH_MAX, initial: WIDTH_DEFAULT, store: STORE_WIDTH, label: "Resize the list", side: "right", testId: "rail-splitter" };

/** The rail's stored width. */
export const readWidth = (): number => readEdgeWidth(RAIL_EDGE);

/** The width a key moves `width` to, or null for a key it ignores. */
export const widthForKey = (width: number, key: string, shift: boolean): number | null => edgeWidthForKey(RAIL_EDGE, width, key, shift);

export function RailSplitter(props: { width: number; onWidth: (width: number) => void; onDragging?: (dragging: boolean) => void; onCycleDensity?: () => void; controls: string }): JSX.Element {
  return <Splitter edge={RAIL_EDGE} {...props} />;
}
