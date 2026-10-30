/**
 * 0.3 — a segmented control: a few choices side by side, one chosen,
 * one click to change (the rail's "Group sessions by"). A radio group to
 * screen readers; one tab stop, the arrow keys move and choose (Home and
 * End too), as the platform's own segmented controls do.
 */
import { useRef, type JSX, type KeyboardEvent } from "react";

export interface Segment<T extends string> {
  value: T;
  label: string;
  /** A tooltip: what it groups by. */
  title?: string;
}

export function Segmented<T extends string>({
  label,
  value,
  segments,
  onChange,
  testId,
  className = "",
}: {
  /** The group's name for screen readers ("Group sessions by"). */
  label: string;
  value: T;
  segments: readonly Segment<T>[];
  onChange: (value: T) => void;
  testId?: string;
  className?: string;
}): JSX.Element {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const at = Math.max(
    0,
    segments.findIndex((segment) => segment.value === value),
  );
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
      onKeyDown={onKey}
      className={"flex gap-0.5 rounded-[10px] border border-hairline bg-surface/70 p-[2px] " + className}
    >
      {segments.map((segment, index) => {
        const on = index === at;
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
              "min-h-6 min-w-0 flex-auto truncate rounded-[7px] px-1 py-0.5 text-label outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent/60 " +
              (on ? "bg-surface-3 font-semibold text-ink shadow-[inset_0_0_0_1px_var(--color-hairline)]" : "text-muted hover:bg-surface-2 hover:text-ink")
            }
          >
            {segment.label}
          </button>
        );
      })}
    </div>
  );
}
