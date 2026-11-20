/**
 * A list's skeleton while its first page is read, or while its code
 * loads (SK-8: a pool's Tasks opened on an empty body for seconds before
 * its code arrived). Instant and paged: never an empty box.
 */
import type { JSX } from "react";

export function ListSkeleton({ label, rows }: { label: string; rows: number }): JSX.Element {
  return (
    <output className="flex flex-col gap-2" aria-label={label}>
      {Array.from({ length: rows }, (_, n) => (
        <span key={n} className="h-16 rounded-lg bg-skeleton motion-safe:animate-pulse" />
      ))}
    </output>
  );
}
