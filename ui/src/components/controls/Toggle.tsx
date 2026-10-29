/**
 * 0.3 — the one Checkbox and the one Radio (the redesign's "same fixes,
 * everywhere"): the accent fill with a white ✓ or dot, a clear focus
 * ring, and a disabled state that says why. The input stays the control
 * (its role, `checked`, label and test id are native); styles.css's
 * `.aj-check` / `.aj-radio` draw it the same in both themes.
 *
 * `reason`: why it can't be used now. Said to screen readers with the
 * control and shown as its tooltip; a line that has room shows it in
 * words too (ChoiceOption's reason slot).
 */
import { forwardRef, useEffect, useId, useImperativeHandle, useRef, type InputHTMLAttributes, type JSX } from "react";

export type ToggleProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  /** Why it is disabled (said with it, and as its tooltip). */
  reason?: string;
};

function useReason(props: ToggleProps): { describedBy?: string; title?: string; note: JSX.Element | null } {
  const id = `${useId().replace(/:/g, "")}-why`;
  const { reason, disabled } = props;
  if (!reason || !disabled) return { describedBy: props["aria-describedby"], title: props.title, note: null };
  return {
    describedBy: [props["aria-describedby"], id].filter(Boolean).join(" "),
    title: props.title ?? reason,
    note: (
      <span id={id} className="sr-only">
        {reason}
      </span>
    ),
  };
}

/** A checkbox: on or off (and `indeterminate`, some of a group). */
export const Checkbox = forwardRef<HTMLInputElement, ToggleProps & { indeterminate?: boolean }>(function Checkbox(
  { className = "", reason, indeterminate = false, ...props },
  ref,
) {
  const inner = useRef<HTMLInputElement | null>(null);
  useImperativeHandle(ref, () => inner.current as HTMLInputElement);
  useEffect(() => {
    if (inner.current) inner.current.indeterminate = indeterminate;
  }, [indeterminate]);
  const why = useReason({ ...props, reason });
  return (
    <>
      <input
        ref={inner}
        type="checkbox"
        {...props}
        title={why.title}
        aria-describedby={why.describedBy}
        className={`aj-check ${className}`}
      />
      {why.note}
    </>
  );
});

/** A radio: one of a group (`name`), the chosen one filled. */
export const Radio = forwardRef<HTMLInputElement, ToggleProps>(function Radio({ className = "", reason, ...props }, ref) {
  const why = useReason({ ...props, reason });
  return (
    <>
      <input ref={ref} type="radio" {...props} title={why.title} aria-describedby={why.describedBy} className={`aj-radio ${className}`} />
      {why.note}
    </>
  );
});
