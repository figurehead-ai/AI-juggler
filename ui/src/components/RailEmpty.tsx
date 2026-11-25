/**
 * AJ-E2327 — the rail's empty state (`lib/rail-empty`): only when Open
 * now is really empty, worded for its lens; a search or a filter says its
 * own ("Nothing matches").
 */
import type { JSX } from "react";
import { railEmptyWords } from "../lib/rail-empty";
import type { Line } from "../lib/rail-lines";

export function RailEmpty({ lens, lines, filtering }: { lens: string; lines: readonly Line[]; filtering: boolean }): JSX.Element | null {
  const words = filtering ? null : railEmptyWords(lens, lines);
  if (!words) return null;
  return (
    <div className="flex flex-col gap-1.5 px-3 py-6" data-testid="rail-empty" data-lens={lens}>
      <p className="text-label font-medium text-ink">{words.title}</p>
      <p className="text-label leading-relaxed text-muted">{words.body}</p>
    </div>
  );
}
