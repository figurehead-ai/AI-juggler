/**
 * Where the rail's lines sit (AJ-E1554): a line's offset from the top and
 * the lines in view, for the rail's virtual list. Split from rail-lines.ts
 * (what the lines are) to keep each file to one concern and under its size.
 */
import { LINE_HEIGHT, type Line, type LineHeights } from "./rail-lines";

/** Where a line starts, from the top of the list. */
export function offsetOf(lines: Line[], index: number, heights: LineHeights = LINE_HEIGHT): number {
  return lines.slice(0, index).reduce((sum, line) => sum + heights[line.kind], 0);
}

/** The lines in view: `[start, end)`, and the space above and below. */
export function windowOf(lines: Line[], top: number, height: number, overscan = 240, heights: LineHeights = LINE_HEIGHT): { start: number; end: number; before: number; after: number } {
  let y = 0;
  let start = 0;
  while (start < lines.length && y + heights[lines[start]!.kind] < top - overscan) {
    y += heights[lines[start]!.kind];
    start += 1;
  }
  const before = y;
  let end = start;
  while (end < lines.length && y < top + height + overscan) {
    y += heights[lines[end]!.kind];
    end += 1;
  }
  let after = 0;
  for (let i = end; i < lines.length; i += 1) after += heights[lines[i]!.kind];
  return { start, end, before, after };
}

