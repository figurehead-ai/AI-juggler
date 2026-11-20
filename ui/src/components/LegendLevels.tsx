/**
 * D-117 — the legend's four attention levels, by how loud each is drawn
 * (`IndicatorLegend`): only the top two are counted.
 */
import type { JSX } from "react";

/** D-117: the four attention levels, loudest first. Only the top two
 * are counted (Needs you, the bell, Home's title). */
// AJ-E3603: drawn as the rail draws them, the level's colour on the mark
// alone (a red ✕, an amber dot), the words in ink.
const LEGEND_LEVELS: { level: number; words: string; tone: string; mark: string; meaning: string }[] = [
  { level: 3, words: "Broken", tone: "text-ink", mark: "level-mark", meaning: "stopped; only you can get it going again (a red ✕, counted)" },
  { level: 2, words: "Needs you", tone: "text-ink", mark: "level-mark", meaning: "waits on your decision (an amber dot, counted)" },
  { level: 1, words: "To review", tone: "text-muted", mark: "size-1.5 shrink-0 rounded-full bg-success", meaning: "good to know, nothing to do now (listed, not counted)" },
  { level: 0, words: "Calm", tone: "text-muted", mark: "size-1.5 shrink-0 rounded-full bg-faint", meaning: "running as it should" },
];

export function LegendLevels(): JSX.Element {
  return (
    <dl className="grid grid-cols-[auto_1fr] items-center gap-x-2 gap-y-1 border-t border-hairline pt-1.5" aria-label="How loud" data-testid="legend-levels">
      {LEGEND_LEVELS.map((row) => (
        <div key={row.words} className="contents">
          <dt className={"flex items-center gap-2 whitespace-nowrap font-medium " + row.tone}>
            <span aria-hidden="true" className={row.mark} data-level={row.level} />
            {row.words}
          </dt>
          <dd className="text-muted">{row.meaning}</dd>
        </div>
      ))}
    </dl>
  );
}
