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
 *
 * D-126 (the kit): one field look with TimeField — filled, a hairline
 * edge, the calendar glyph first and the calendar's chevron last, the
 * accent's edge and halo while typing; `lg` is the making screens' 36 px.
 *
 * Never cut: the input is as wide as its pattern or its placeholder,
 * whichever is longer, and the field never narrower than its glyph, that
 * input and its chevron, whatever width a caller gives it (a caller's
 * `w-36` cut "Choose a day" to "Choose a da" once the kit added both
 * glyphs: the 0.5 baseline refresh).
 */
import { useMemo, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { IconCalendar, IconChevronDown } from "../icons-controls";
import { FIELD_SHELL, fieldEdge } from "./field-look";
import { dateFormat, datePattern, formatDate, readDate, shapeTyped } from "../../lib/date-text";
import { Calendar } from "./Calendar";
import { useTypedField, type FieldProps, type FieldRead } from "./typed-field";

const SIZES = {
  sm: "h-7 gap-1.5 pl-2 text-xs",
  md: "h-8 gap-2 pl-2.5 text-meta",
  lg: "h-9 gap-2 pl-3 text-body",
} as const;

export function DateField({
  value,
  onCommit,
  min,
  max,
  placeholder = "Not set",
  size = "md",
  className = "",
  testId,
  ...rest
}: FieldProps & { min?: string; max?: string }): JSX.Element {
  const format = useMemo(() => dateFormat(), []);
  const pattern = datePattern(format);
  const input = useRef<HTMLInputElement | null>(null);
  const button = useRef<HTMLButtonElement | null>(null);
  const [at, setAt] = useState<{ x: number; y: number; above: number; right: number } | null>(null);
  const outside = (iso: string) => (min !== undefined && iso < min) || (max !== undefined && iso > max);
  const read = (text: string): FieldRead => {
    const typed = readDate(text, format);
    if (typed.kind === "empty") return { value: null };
    if (typed.kind === "partial") return { error: `Not a whole date yet: type it as ${pattern}. Nothing was saved.` };
    if (typed.kind === "invalid") return { error: `There's no such day: type it as ${pattern}. Nothing was saved.` };
    if (outside(typed.iso)) return { error: `Choose a day ${rangeWords(min, max, format)}. Nothing was saved.` };
    return { value: typed.iso };
  };
  const { text, error, hand, onKey: onFieldKey, input: field, hintId, errorId } = useTypedField({
    ...rest,
    value,
    onCommit,
    placeholder,
    testId,
    pattern,
    shown: (day) => formatDate(day, format),
    read,
    shape: (typed) => shapeTyped(typed, format),
  });
  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (onFieldKey(event)) return;
    if (event.key === "ArrowDown" && event.altKey) {
      event.preventDefault();
      openCalendar();
    }
  };
  const openCalendar = () => {
    const box = (button.current?.parentElement ?? button.current)?.getBoundingClientRect();
    if (!box) return;
    setAt({ x: box.left, y: box.bottom + 4, above: box.top - 4, right: box.right });
  };
  const typed = readDate(text, format);
  return (
    <span className={`inline-flex min-w-max flex-col gap-1 ${className}`} data-testid={testId ? `${testId}-field` : undefined}>
      <span
        className={`${FIELD_SHELL} ${fieldEdge(error !== null)} ${SIZES[size]}`}
      >
        <IconCalendar width={14} height={14} aria-hidden="true" className="shrink-0 text-faint" />
        <input ref={input} {...field} autoCorrect="off" autoCapitalize="off" spellCheck={false} size={Math.max(pattern.length, placeholder.length) + 1} onKeyDown={onKey} />
        <button
          ref={button}
          type="button"
          aria-haspopup="dialog"
          aria-expanded={at !== null}
          aria-label="Choose from a calendar"
          title="Choose from a calendar"
          onClick={() => (at ? setAt(null) : openCalendar())}
          className="flex h-full shrink-0 items-center rounded-r-[10px] px-2 text-faint outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          data-testid={testId ? `${testId}-calendar` : undefined}
        >
          <IconChevronDown width={14} height={14} aria-hidden="true" />
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
