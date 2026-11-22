/**
 * The rail's toolbar controls: a filter chip with its count. (0.3: the
 * compact select that was here is the shared Picker now.)
 */
import type { ReactNode } from "react";
import { SELECTED_CARD } from "./controls/selected";

export function FilterChip({ active, onClick, count, children }: { active: boolean; onClick: () => void; count?: number; children: ReactNode }) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={
        "whitespace-nowrap rounded-full border px-1.5 py-0.5 text-label transition-colors outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " +
        // D-117 (selection): the one selected look, neutral (the accent is
        // for actions and focus).
        (active
          ? SELECTED_CARD + " font-semibold text-selected-ink"
          : "border-hairline font-medium text-muted hover:bg-surface hover:text-ink")
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
