/**
 * D-126 — what the kit's typed fields (DateField, TimeField) share: their
 * props, and how typing in one goes. The text typed shows over the value
 * it was typed over: the saved value shows again once it changes (saved,
 * or set elsewhere), or on Escape. It is read whole and handed over
 * (`onCommit`) only on a commit — leaving the field, Enter, or the
 * field's own pick or step (`hand`) — and only when it is a real value
 * that differs; one that can't be read saves nothing and says why under
 * the field.
 */
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";

/** The props DateField and TimeField both take. */
export interface FieldProps {
  /** The saved value (a day, `YYYY-MM-DD`; a time, `HH:MM`, 24-hour);
   * `null` when unset. */
  value: string | null;
  /** A whole, real value (or `null`, emptied), on a commit only, and only
   * when it differs from `value`. */
  onCommit: (next: string | null) => void;
  id?: string;
  ariaLabel?: string;
  labelledBy?: string;
  describedBy?: string;
  /** Shown while it's empty and not being typed in. */
  placeholder?: string;
  size?: "sm" | "md" | "lg";
  className?: string;
  testId?: string;
}

/** The typed text read whole: the value to hand over, or why it can't be. */
export type FieldRead = { value: string | null } | { error: string };

/** A typed field's state, its input's props and its Enter and Escape. */
export function useTypedField({
  value,
  onCommit,
  id,
  ariaLabel,
  labelledBy,
  describedBy,
  placeholder,
  testId,
  pattern,
  shown,
  read,
  shape,
}: Omit<FieldProps, "size" | "className"> & {
  /** How it's typed ("22:00"), the placeholder while it's typed in. */
  pattern: string;
  /** A value as its typed text. */
  shown: (value: string | null) => string;
  read: (text: string) => FieldRead;
  /** What's typed, shaped while typing on (never while deleting). */
  shape: (typed: string) => string;
}) {
  const base = useId().replace(/:/g, "");
  const [draft, setDraft] = useState<{ text: string; over: string | null } | null>(null);
  const text = draft && draft.over === value ? draft.text : shown(value);
  const [focused, setFocused] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // What was last handed over: Enter, then leaving the field, saves once
  // even before the saved value comes back.
  const sent = useRef<string | null>(value);
  useEffect(() => {
    sent.current = value;
  }, [value]);
  const hand = (next: string | null) => {
    setError(null);
    setDraft({ text: shown(next), over: value });
    if (next === value || next === sent.current) return;
    sent.current = next;
    onCommit(next);
  };
  const commit = () => {
    const got = read(text);
    if ("error" in got) setError(got.error);
    else hand(got.value);
  };
  /** Enter commits; Escape undoes only the typing (a second Escape is the
   * sheet's). Whether the key was one of those. */
  const onKey = (event: KeyboardEvent<HTMLInputElement>): boolean => {
    if (event.key === "Enter") {
      event.preventDefault();
      commit();
      return true;
    }
    if (event.key === "Escape" && (text !== shown(value) || error)) {
      event.preventDefault();
      event.stopPropagation();
      setError(null);
      setDraft(null);
      return true;
    }
    return false;
  };
  const hintId = `${base}-hint`;
  const errorId = `${base}-error`;
  const input = {
    id,
    type: "text",
    inputMode: "numeric",
    autoComplete: "off",
    autoCorrect: "off",
    autoCapitalize: "off",
    spellCheck: false,
    "aria-label": ariaLabel,
    "aria-labelledby": labelledBy,
    "aria-describedby": [describedBy, hintId, error ? errorId : null].filter(Boolean).join(" "),
    "aria-invalid": error ? true : undefined,
    placeholder: text === "" && !focused ? placeholder : pattern,
    value: text,
    onFocus: () => {
      setFocused(true);
      // A save that failed may be tried again.
      sent.current = value;
    },
    onBlur: () => {
      setFocused(false);
      commit();
    },
    onChange: (event: { target: { value: string } }) => {
      const next = event.target.value;
      setError(null);
      setDraft({ text: next.length > text.length ? shape(next) : next, over: value });
    },
    className: "w-full min-w-0 bg-transparent tabular-nums outline-none placeholder:text-muted",
    "data-testid": testId,
    "data-value": value ?? "",
  } as const;
  return { text, error, hand, onKey, input, hintId, errorId };
}
