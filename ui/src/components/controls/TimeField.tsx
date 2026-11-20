/**
 * 0.4.6 (D-126) — the one time field (no system time control, AJ-E2801),
 * the same look as DateField: a time of day typed as 24-hour text
 * ("22:00"), the colon put in as the hour is whole, or stepped with the
 * arrow keys (`step` minutes; Shift for an hour). Like DateField, it is
 * read whole and handed over (`onCommit`) only on a commit — leaving the
 * field, Enter, or a step — and only when it is a real time; a partial
 * or impossible one saves nothing and says why under the field; Escape
 * puts the saved value back. Unset, it is empty with its placeholder.
 */
import type { JSX, KeyboardEvent } from "react";
import { FIELD_SHELL, fieldEdge } from "./field-look";
import { useTypedField, type FieldProps } from "./typed-field";

const SIZES = {
  sm: "h-7 px-2 text-xs",
  md: "h-8 px-2.5 text-meta",
  lg: "h-9 px-3 text-body",
} as const;

const PATTERN = "22:00";

export type TimeRead = { kind: "empty" } | { kind: "partial" } | { kind: "invalid" } | { kind: "time"; hhmm: string };

const two = (n: number) => String(n).padStart(2, "0");

/** `text` read as a time of day: "7" and "07" are 07:00, "730" 07:30,
 * "7:30" and "0730" too; "7:" or "7:3" aren't whole yet; "25:00" isn't
 * a time. */
export function readTime(text: string): TimeRead {
  const t = text.trim();
  if (t === "") return { kind: "empty" };
  const parts = /^(\d{1,2})(?::(\d{0,2}))?$/.exec(t) ?? /^(\d{1,2})(\d{2})$/.exec(t);
  if (!parts) return { kind: "invalid" };
  const [, h, m] = parts;
  if (m !== undefined && m.length < 2) return { kind: "partial" };
  const hours = Number(h);
  const minutes = m === undefined ? 0 : Number(m);
  if (hours > 23 || minutes > 59) return { kind: "invalid" };
  return { kind: "time", hhmm: `${two(hours)}:${two(minutes)}` };
}

/** What's typed, the colon put in once two hour digits are (typing on). */
export function shapeTime(next: string): string {
  return /^\d{2}$/.test(next) ? `${next}:` : /^\d{3}$/.test(next) ? `${next.slice(0, 2)}:${next.slice(2)}` : next;
}

/** `hhmm` moved by `minutes`, round the clock. */
export function stepTime(hhmm: string | null, minutes: number): string {
  const read = hhmm ? readTime(hhmm) : null;
  const [h, m] = read?.kind === "time" ? read.hhmm.split(":").map(Number) : [0, 0];
  const total = (((h! * 60 + m! + minutes) % 1440) + 1440) % 1440;
  return `${two(Math.floor(total / 60))}:${two(total % 60)}`;
}

export function TimeField({
  value,
  onCommit,
  step = 15,
  placeholder = "Not set",
  size = "md",
  className = "",
  testId,
  ...rest
}: FieldProps & {
  /** The arrow keys' step, in minutes. */
  step?: number;
}): JSX.Element {
  const { text, error, hand, onKey: onFieldKey, input, hintId, errorId } = useTypedField({
    ...rest,
    value,
    onCommit,
    placeholder,
    testId,
    pattern: PATTERN,
    shown: (time) => time ?? "",
    read: (typed) => {
      const read = readTime(typed);
      if (read.kind === "empty") return { value: null };
      if (read.kind === "partial") return { error: `Not a whole time yet: type it as ${PATTERN}. Nothing was saved.` };
      if (read.kind === "invalid") return { error: `There's no such time: type it as ${PATTERN}. Nothing was saved.` };
      return { value: read.hhmm };
    },
    shape: shapeTime,
  });
  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (onFieldKey(event) || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    event.preventDefault();
    const read = readTime(text);
    const from = read.kind === "time" ? read.hhmm : value;
    const by = (event.shiftKey ? 60 : step) * (event.key === "ArrowUp" ? 1 : -1);
    hand(stepTime(from, by));
  };
  return (
    <span className={`inline-flex flex-col gap-1 ${className}`}>
      <span className={`${FIELD_SHELL} ${fieldEdge(error !== null)} ${SIZES[size]}`}>
        <input {...input} size={6} onKeyDown={onKey} />
      </span>
      <span id={hintId} className="sr-only">
        Type it as {PATTERN}, or step it with the arrow keys.
      </span>
      {error && (
        <span id={errorId} role="alert" className="text-label text-danger" data-testid={testId ? `${testId}-error` : undefined}>
          {error}
        </span>
      )}
    </span>
  );
}
