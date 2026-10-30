/**
 * The rail's toolbar controls: a filter chip with its count. (0.3: the
 * compact select that was here is the shared Picker now.)
 */
import type { ReactNode } from "react";

export function FilterChip({ active, onClick, count, children }: { active: boolean; onClick: () => void; count?: number; children: ReactNode }) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={
        "whitespace-nowrap rounded-full border px-1.5 py-0.5 text-label font-medium transition-colors outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
        (active
          ? "border-accent/40 bg-accent-soft text-accent-hi"
          : "border-hairline text-muted hover:bg-surface hover:text-ink")
      }
    >
      {children}
      {/* The name is what's shown (WCAG 2.5.3): "Needs me 5 items". */}
      {/* D-25: a zero count says nothing — only real counts show. */}
      {count !== undefined && count > 0 && (
        <>
          {" "}
          <span className="tabular-nums opacity-80">{count}</span>
          {" "}<span className="sr-only">items</span>
        </>
      )}
    </button>
  );
}
