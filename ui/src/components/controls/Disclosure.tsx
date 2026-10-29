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
  testId,
  className = "",
  summaryClassName = "",
}: {
  summary: ReactNode;
  children: ReactNode;
  /** Open at first. */
  open?: boolean;
  testId?: string;
  className?: string;
  summaryClassName?: string;
}): JSX.Element {
  return (
    <details open={open} data-testid={testId} className={"group " + className}>
      <summary
        className={
          "flex w-fit cursor-pointer list-none items-center gap-1.5 rounded-md text-meta text-muted outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60 [&::-webkit-details-marker]:hidden " +
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
