/**
 * A8 (the AJ-E3306 rc22 walk) — the one date field, in place of the
 * system date control (AJ-E2801: no system control outside the shared
 * ones). The system control saved a half-typed date on every keystroke
 * (typing 11282026 into a renewal ended on 12/09/2026) and drew an unset
 * date as today's, as if it were set.
 *
 * Here a date is typed as text in the user's own order, the separators
 * put in as each part is whole, or picked from a month's calendar; it is
 * read whole and handed over (`onCommit`) only on a commit — leaving the
 * field, Enter, or a day picked — and only when it is a real day (inside
 * `min`…`max`). A partial or impossible one saves nothing and says why
 * under the field; Escape puts the saved value back. Unset, it is empty
 * with its placeholder ("Not set"), never today.
 * Alt+↓ opens the calendar (`Calendar`).
 */
import { useEffect, useId, useMemo, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { IconCalendar } from "../icons-controls";
import { dateFormat, datePattern, formatDate, readDate, shapeTyped } from "../../lib/date-text";
import { Calendar } from "./Calendar";

const SIZES = {
  sm: "h-7 text-xs",
  md: "h-8 text-meta",
} as const;

export function DateField({
  value,
  onCommit,
  min,
  max,
  id,
  ariaLabel,
  labelledBy,
  describedBy,
  placeholder = "Not set",
  size = "md",
  className = "",
  testId,
}: {
  /** The saved day, `YYYY-MM-DD`; `null` when unset. */
  value: string | null;
  /** A whole, real day (or `null`, emptied), on a commit only, and only
   * when it differs from `value`. */
  onCommit: (next: string | null) => void;
  min?: string;
  max?: string;
  id?: string;
  ariaLabel?: string;
  labelledBy?: string;
  describedBy?: string;
  /** Shown while it's empty and not being typed in. */
  placeholder?: string;
  size?: keyof typeof SIZES;
  className?: string;
  testId?: string;
}): JSX.Element {
  const format = useMemo(() => dateFormat(), []);
  const pattern = datePattern(format);
  const base = useId().replace(/:/g, "");
  const input = useRef<HTMLInputElement | null>(null);
  const button = useRef<HTMLButtonElement | null>(null);
  // What's typed, over the value it was typed over: the saved value
  // shows again once it changes (saved, or set elsewhere), or on Escape.
  const [draft, setDraft] = useState<{ text: string; over: string | null } | null>(null);
  const text = draft && draft.over === value ? draft.text : formatDate(value, format);
  const setText = (next: string) => setDraft({ text: next, over: value });
  const [focused, setFocused] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [at, setAt] = useState<{ x: number; y: number; above: number; right: number } | null>(null);
  // What was last handed over: Enter, then leaving the field, saves once
  // even before the saved value comes back.
  const sent = useRef<string | null>(value);
  useEffect(() => {
    sent.current = value;
  }, [value]);
  const outside = (iso: string) => (min !== undefined && iso < min) || (max !== undefined && iso > max);
  const hand = (next: string | null) => {
    setError(null);
    setText(formatDate(next, format));
    if (next === value || next === sent.current) return;
    sent.current = next;
    onCommit(next);
  };
  const commit = () => {
    const read = readDate(text, format);
    if (read.kind === "empty") return hand(null);
    if (read.kind === "partial") return setError(`Not a whole date yet: type it as ${pattern}. Nothing was saved.`);
    if (read.kind === "invalid") return setError(`There's no such day: type it as ${pattern}. Nothing was saved.`);
    if (outside(read.iso)) return setError(`Choose a day ${rangeWords(min, max, format)}. Nothing was saved.`);
    hand(read.iso);
  };
  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      commit();
    } else if (event.key === "Escape" && (text !== formatDate(value, format) || error)) {
      // Only the typing is undone; a second Escape is the sheet's.
      event.preventDefault();
      event.stopPropagation();
      setError(null);
      setDraft(null);
    } else if (event.key === "ArrowDown" && event.altKey) {
      event.preventDefault();
      openCalendar();
    }
  };
  const openCalendar = () => {
    const box = (button.current?.parentElement ?? button.current)?.getBoundingClientRect();
    if (!box) return;
    setAt({ x: box.left, y: box.bottom + 4, above: box.top - 4, right: box.right });
  };
  const hintId = `${base}-hint`;
  const errorId = `${base}-error`;
  const shown = text === "" && !focused ? placeholder : pattern;
  const typed = readDate(text, format);
  return (
    <span className={`inline-flex flex-col gap-1 ${className}`}>
      <span
        className={`inline-flex items-center rounded-lg border bg-surface pl-2 text-ink focus-within:ring-1 focus-within:ring-accent/60 ${error ? "border-danger/60" : "border-hairline"} ${SIZES[size]}`}
      >
        <input
          ref={input}
          id={id}
          type="text"
          inputMode="numeric"
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          aria-label={ariaLabel}
          aria-labelledby={labelledBy}
          aria-describedby={[describedBy, hintId, error ? errorId : null].filter(Boolean).join(" ")}
          aria-invalid={error ? true : undefined}
          placeholder={shown}
          value={text}
          size={pattern.length + 1}
          onFocus={() => {
            setFocused(true);
            // A save that failed may be tried again.
            sent.current = value;
          }}
          onBlur={() => {
            setFocused(false);
            commit();
          }}
          onChange={(event) => {
            const next = event.target.value;
            setError(null);
            // Shaped only while typing on, never while deleting.
            setText(next.length > text.length ? shapeTyped(next, format) : next);
          }}
          onKeyDown={onKey}
          className="w-full min-w-0 bg-transparent tabular-nums outline-none placeholder:text-muted"
          data-testid={testId}
          data-value={value ?? ""}
        />
        <button
          ref={button}
          type="button"
          aria-haspopup="dialog"
          aria-expanded={at !== null}
          aria-label="Choose from a calendar"
          title="Choose from a calendar"
          onClick={() => (at ? setAt(null) : openCalendar())}
          className="flex h-full shrink-0 items-center rounded-r-lg px-1.5 text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid={testId ? `${testId}-calendar` : undefined}
        >
          <IconCalendar width={14} height={14} aria-hidden="true" />
        </button>
      </span>
      <span id={hintId} className="sr-only">
        Type it as {pattern}, or choose it from the calendar.
      </span>
      {error && (
        <span id={errorId} role="alert" className="text-label text-danger" data-testid={testId ? `${testId}-error` : undefined}>
          {error}
        </span>
      )}
      {at && (
        <Calendar
          at={at}
          value={typed.kind === "date" ? typed.iso : value}
          min={min}
          max={max}
          testId={testId ? `${testId}-month` : undefined}
          onPick={(iso) => {
            setAt(null);
            hand(iso);
            input.current?.focus();
          }}
          onClose={(restore) => {
            setAt(null);
            if (restore) button.current?.focus();
          }}
        />
      )}
    </span>
  );
}

function rangeWords(min: string | undefined, max: string | undefined, format: ReturnType<typeof dateFormat>): string {
  if (min && max) return `from ${formatDate(min, format)} to ${formatDate(max, format)}`;
  if (min) return `on or after ${formatDate(min, format)}`;
  return `on or before ${formatDate(max, format)}`;
}
