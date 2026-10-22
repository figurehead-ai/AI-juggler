/** A step's done mark (first run's step marks and access rows). */
import type { JSX } from "react";

export function Check({ className = "" }: { className?: string }): JSX.Element {
  return (
    <svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className={className}>
      <path d="M5 12.5l4.5 4.5L19 7.5" />
    </svg>
  );
}
