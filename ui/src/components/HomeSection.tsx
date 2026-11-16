/**
 * D-118 (AJ-E3701 R3) — a Home section's head: the calm section label
 * (D-117: the section tone, small tracked capitals), its count, and a
 * fold chevron (muted at rest, in ink on hover and focus), its state
 * remembered per viewer. The rail's own head (`SectionHead`) is the
 * pattern; on Needs you now the count carries the level (amber ⚠ n at L2,
 * red ✕ n at L3), the one loud mark Home's head gives a need. The glyph is
 * decoration: the button's name says the count in words.
 */
import type { JSX, ReactNode } from "react";
import { Chevron } from "./PlaceBoardParts";
import { levelCountWords, type AttentionLevel } from "../lib/attention";
import { useFold } from "../lib/feed-state";

export function HomeSection({
  id,
  label,
  count,
  level = 0,
  broken = [],
  extra,
  initial = true,
  className = "",
  region = false,
  children,
}: {
  /** Its id: the heading's, and the key its fold is kept under. */
  id: string;
  label: string;
  count?: number;
  /** The count's level (Needs you now only). */
  level?: AttentionLevel;
  /** Each L3 thing's word, said with the count. */
  broken?: readonly string[];
  /** Beside the head, before the chevron. */
  extra?: ReactNode;
  initial?: boolean;
  className?: string;
  /** A landmark of its own (the pinned zone: "Needs you now"). */
  region?: boolean;
  children: ReactNode;
}): JSX.Element {
  const [open, toggle] = useFold(id, initial);
  const loud = level >= 2 && count !== undefined && count > 0;
  const tone = level === 3 ? "text-danger" : level === 2 ? "text-warning" : "text-muted";
  const name = count !== undefined && count > 0 ? `${label} ${loud ? levelCountWords(count, broken) : count}` : label;
  // Only the pinned zone is a landmark of its own; the rest are headed
  // parts of their column.
  const Tag = region ? "section" : "div";
  return (
    <Tag aria-labelledby={region ? `${id}-h` : undefined} className={"flex min-w-0 flex-col gap-2 " + className} data-testid={`${id}-section`} data-open={open}>
      <div className="flex min-h-7 items-center gap-2">
        <h2 id={`${id}-h`} className="section-label flex min-w-0 flex-1">
          <button type="button" aria-expanded={open} aria-controls={`${id}-body`} aria-label={name} onClick={toggle} className="group/fold flex min-h-6 w-full items-center gap-1.5 rounded-md outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid={`${id}-fold`}>
            {label}
            {/* A space before the count: the visible words read "Your apps
                4", as the button's name begins (WCAG 2.5.3). */}
            {count !== undefined && count > 0 && " "}
            {count !== undefined && count > 0 && (
              <span className={"inline-flex items-center gap-1 font-semibold normal-case tracking-normal tabular-nums " + tone} data-level={level} data-testid="section-count">
                {loud && <span aria-hidden="true" className="level-glyph" data-level={level} />}
                {count}
              </span>
            )}
            <Chevron open={open} />
          </button>
        </h2>
        {extra}
      </div>
      <div id={`${id}-body`} hidden={!open} className="flex min-w-0 flex-col gap-2">
        {open && children}
      </div>
    </Tag>
  );
}
