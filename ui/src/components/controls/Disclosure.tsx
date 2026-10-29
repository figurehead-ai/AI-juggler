/**
 * 0.3 — the app's disclosure, in place of the browser's ▸ triangle: a
 * `<details>` whose summary carries the rail's fold chevron (turned
 * when open), a focus ring, and the same quiet type everywhere.
 */
import type { JSX, ReactNode } from "react";
import { FoldChevron } from "../this-mac-memory";

export function Disclosure({
  summary,
  children,
  open,
  onToggle,
  tone = "quiet",
  testId,
  className = "",
  summaryClassName = "",
}: {
  summary: ReactNode;
  children: ReactNode;
  /** Open at first (or, with `onToggle`, as its owner keeps it). */
  open?: boolean;
  onToggle?: (open: boolean) => void;
  /** A quiet line ("Details"), a group's heading in a card (Settings'
   * Terminal), or a section's title (a dialog's Details). */
  tone?: "quiet" | "heading" | "title";
  testId?: string;
  className?: string;
  summaryClassName?: string;
}): JSX.Element {
  return (
    <details open={open} onToggle={onToggle && ((event) => onToggle(event.currentTarget.open))} data-testid={testId} className={"group " + className}>
      <summary
        className={
          "flex w-fit min-h-6 cursor-pointer list-none items-center gap-1.5 rounded-md outline-none focus-visible:ring-2 focus-visible:ring-accent/60 [&::-webkit-details-marker]:hidden " +
          (tone === "title" ? "text-body font-semibold text-ink " : tone === "heading" ? "text-meta font-semibold text-ink " : "text-meta text-muted hover:text-ink ") +
          summaryClassName
        }
      >
        <FoldChevron />
        {summary}
      </summary>
      {children}
    </details>
  );
}
