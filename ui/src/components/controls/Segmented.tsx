/**
 * 0.3 — a segmented control: a few choices side by side, one chosen,
 * one click to change (the rail's "Group sessions by", a page's range).
 * A radio group to screen readers; one tab stop, the arrow keys move and
 * choose (Home and End too), as the platform's own segmented controls do.
 *
 * D-126: the one look, everywhere (the usage pages kept a second copy,
 * `usage/controls`, with its own look): filled, a hairline edge, the
 * chosen one in 0.5's SELECTED (a 3:1 neutral edge, the raised fill,
 * semibold ink; D-117), never the accent, which stays for actions and
 * focus. Two sizes: `md` (28 px, 13 px words) and `sm` (24 px, 12 px)
 * where room is tight (the rail). A segment may carry an icon before its
 * word (Member, Reviewer).
 */
import { useRef, type JSX, type KeyboardEvent } from "react";
import type { ControlIcon } from "./ChoiceLine";
import { SELECTED } from "./selected";

export interface Segment<T extends string> {
  value: T;
  label: string;
  /** A tooltip: what it groups by. */
  title?: string;
  /** An icon before the word. */
  icon?: ControlIcon;
  /** A count after the word, quieter (a filter's "Paused 3"). */
  count?: number;
}

export type SegmentedSize = "sm" | "md";

const SIZES: Record<SegmentedSize, { item: string; icon: number }> = {
  md: { item: "h-7 gap-1.5 px-2.5 text-meta", icon: 14 },
  sm: { item: "h-6 gap-1 px-2 text-label", icon: 12 },
};

/** The control's frame: the fill, its hairline edge, 3 px inside. */
export const SEGMENTED_FRAME = "seg-fill gap-0.5 rounded-[10px] bg-fill p-[3px] inset-ring-1 inset-ring-hairline/75";

export function Segmented<T extends string>({
  label,
  value,
  segments,
  onChange,
  testId,
  className = "",
  wrap = false,
  size = "md",
}: {
  /** The group's name for screen readers ("Group sessions by"). */
  label: string;
  value: T;
  segments: readonly Segment<T>[];
  onChange: (value: T) => void;
  testId?: string;
  className?: string;
  /** Filter chips with counts: whole words on more lines when narrow,
   * never cut ("Limit reache…" off the card at 390 px). */
  wrap?: boolean;
  size?: SegmentedSize;
}): JSX.Element {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const at = Math.max(
    0,
    segments.findIndex((segment) => segment.value === value),
  );
  const look = SIZES[size];
  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    const last = segments.length - 1;
    const next =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? (at + 1) % segments.length
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? (at - 1 + segments.length) % segments.length
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : null;
    if (next === null) return;
    event.preventDefault();
    refs.current[next]?.focus();
    onChange(segments[next]!.value);
  };
  return (
    <div
      role="radiogroup"
      aria-label={label}
      data-testid={testId}
      data-size={size}
      onKeyDown={onKey}
      className={(wrap ? "flex min-w-0 max-w-full flex-wrap " : "flex ") + SEGMENTED_FRAME + " " + className}
    >
      {segments.map((segment, index) => {
        const on = index === at;
        const Icon = segment.icon;
        return (
          <button
            key={segment.value}
            ref={(element) => {
              refs.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={on}
            tabIndex={on ? 0 : -1}
            title={segment.title}
            data-testid={testId ? `${testId}-${segment.value}` : undefined}
            onClick={() => onChange(segment.value)}
            // Each as wide as its word first, the rest shared: four equal
            // quarters cut "Account" to "Accou…" beside a short "App"
            // and "Pool" (the 0.3 re-render).
            className={
              (wrap ? "flex-none whitespace-nowrap " : "min-w-0 flex-auto ") +
              `inline-flex items-center justify-center rounded-[7px] ${look.item} outline-none focus-visible:ring-2 focus-visible:ring-accent/60 ` +
              (on ? SELECTED : "text-ink-2 hover:bg-surface-2 hover:text-ink")
            }
          >
            {Icon && <Icon aria-hidden="true" width={look.icon} height={look.icon} className="shrink-0" />}
            <span className={wrap ? "" : "min-w-0 truncate"}>{segment.label}</span>
            {segment.count !== undefined && <span className="ml-1 font-normal tabular-nums text-muted">{segment.count}</span>}
          </button>
        );
      })}
    </div>
  );
}
