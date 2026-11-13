/**
 * D-103.6 — where Home's Needs you sits. Until the needs load, its
 * skeleton, sized from the last look (lib/needs-place): as many rows as
 * last time, as tall as the card was, so the same needs arriving move
 * nothing; none last time, no block at all; nothing kept yet (a first
 * run), one row. Then the card itself, whose shape it keeps for next time.
 */
import type { JSX, ReactNode } from "react";
import { NeedsPlaceholder } from "./Placeholder";
import { NEEDS_PLACE, keptShape, useKeepShape } from "../lib/needs-place";

/** Rows a skeleton draws at most (its height is the card's anyway). */
const ROWS_DRAWN = 6;

export function HomeNeedsPlace({ known, rows, children }: { known: boolean; rows: number; children: ReactNode }): JSX.Element | null {
  const box = useKeepShape<HTMLDivElement>(NEEDS_PLACE, known, rows);
  if (!known) {
    const kept = keptShape(NEEDS_PLACE);
    if (kept?.rows === 0) return null;
    return <NeedsPlaceholder rows={Math.min(kept?.rows ?? 1, ROWS_DRAWN)} height={kept && kept.height > 0 ? kept.height : null} label="Loading what needs you" />;
  }
  if (rows === 0) return null;
  return (
    <div ref={box} className="min-w-0">
      {children}
    </div>
  );
}
