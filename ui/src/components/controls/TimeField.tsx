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
import { useEffect, useId, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { FIELD_SHELL, fieldEdge } from "./field-look";

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
  id,
  ariaLabel,
  labelledBy,
  describedBy,
  placeholder = "Not set",
  size = "md",
  className = "",
  testId,
}: {
  /** The saved time, `HH:MM` (24-hour); `null` when unset. */
  value: string | null;
  /** A whole, real time (or `null`, emptied), on a commit only, and only
   * when it differs from `value`. */
  onCommit: (next: string | null) => void;
  /** The arrow keys' step, in minutes. */
  step?: number;
  id?: string;
  ariaLabel?: string;
  labelledBy?: string;
  describedBy?: string;
  placeholder?: string;
  size?: keyof typeof SIZES;
  className?: string;
  testId?: string;
}): JSX.Element {
  const base = useId().replace(/:/g, "");
  const [draft, setDraft] = useState<{ text: string; over: string | null } | null>(null);
  const text = draft && draft.over === value ? draft.text : (value ?? "");
  const [error, setError] = useState<string | null>(null);
  const [focused, setFocused] = useState(false);
  const sent = useRef<string | null>(value);
  useEffect(() => {
    sent.current = value;
  }, [value]);
  const hand = (next: string | null) => {
    setError(null);
    setDraft({ text: next ?? "", over: value });
    if (next === value || next === sent.current) return;
    sent.current = next;
    onCommit(next);
  };
  const commit = () => {
    const read = readTime(text);
    if (read.kind === "empty") return hand(null);
    if (read.kind === "partial") return setError(`Not a whole time yet: type it as ${PATTERN}. Nothing was saved.`);
    if (read.kind === "invalid") return setError(`There's no such time: type it as ${PATTERN}. Nothing was saved.`);
    hand(read.hhmm);
  };
  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      commit();
    } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      const read = readTime(text);
      const from = read.kind === "time" ? read.hhmm : value;
      const by = (event.shiftKey ? 60 : step) * (event.key === "ArrowUp" ? 1 : -1);
      hand(stepTime(from, by));
    } else if (event.key === "Escape" && (text !== (value ?? "") || error)) {
      // Only the typing is undone; a second Escape is the sheet's.
      event.preventDefault();
      event.stopPropagation();
      setError(null);
      setDraft(null);
    }
  };
  const hintId = `${base}-hint`;
  const errorId = `${base}-error`;
  return (
    <span className={`inline-flex flex-col gap-1 ${className}`}>
      <span className={`${FIELD_SHELL} ${fieldEdge(error !== null)} ${SIZES[size]}`}>
        <input
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
          placeholder={text === "" && !focused ? placeholder : PATTERN}
          value={text}
          size={6}
          onFocus={() => {
            setFocused(true);
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
            setDraft({ text: next.length > text.length ? shapeTime(next) : next, over: value });
          }}
          onKeyDown={onKey}
          className="w-full min-w-0 bg-transparent tabular-nums outline-none placeholder:text-muted"
          data-testid={testId}
          data-value={value ?? ""}
        />
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
