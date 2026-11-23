/**
 * AJ-E1551 — the rail's Needs you holds its place while the needs load
 * (D-103.6's rule, as Home's Needs you): Needs you comes with the needs
 * read, so until it's known the board keeps the height its Needs you had
 * last time (worked out from its lines' heights, kept for this data
 * folder), and Open now below never jumps down when they arrive. None last
 * time, no place.
 */
import { useEffect, useMemo, type JSX } from "react";
import type { Line, LineHeights } from "../lib/rail-lines";
import { RAIL_NEEDS_PLACE, keepShape, keptShape } from "../lib/needs-place";
import { LoadingState } from "./LoadingState";

export function RailNeedsHeld({ lines, heights, known }: { lines: readonly Line[]; heights: LineHeights; known: boolean }): JSX.Element | null {
  const shape = useMemo(() => {
    let height = 0;
    let rows = 0;
    for (const [index, line] of lines.entries()) {
      if (line.kind === "section" ? line.id !== "needs" : index === 0) break;
      height += heights[line.kind];
      if (line.kind !== "section") rows += 1;
    }
    return { rows, height: Math.round(height) };
  }, [lines, heights]);
  useEffect(() => {
    if (known) keepShape(RAIL_NEEDS_PLACE, shape);
  }, [known, shape]);
  const held = !known && shape.rows === 0 ? keptShape(RAIL_NEEDS_PLACE) : null;
  if (!held || held.rows === 0 || held.height === 0) return null;
  return (
    <LoadingState state="loading" what="what needs you" skeleton={<span className="block rounded-lg bg-skeleton motion-safe:animate-pulse" style={{ height: held.height }} />} empty={null} onRetry={() => undefined}>
      {null}
    </LoadingState>
  );
}
