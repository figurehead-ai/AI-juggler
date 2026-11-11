/**
 * Why a control can't act, said in place: on hover AND on keyboard focus,
 * in a tip drawn over the page (a clipped row or a zoomed canvas can't
 * cut it; Escape, a scroll or the wheel closes it), and as the control's
 * accessible description.
 *
 * The rc19 walk (the "Shell only" chip) and the rc21 walk (an ended
 * session's greyed Resume): a native `title` is shown by the app's web
 * view only after a still hover in a key window, never on focus, and a
 * `disabled` button gets neither (no pointer events, no focus). So a
 * greyed control here stays focusable, says `aria-disabled`, and does
 * nothing when pressed but show its reason.
 */
import { useCallback, useEffect, useId, useRef, useState, type ButtonHTMLAttributes, type JSX, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { Button } from "./primitives";

/** The tip's width (`w-64`), kept inside the window. */
const TIP_W = 256;

/** Whether focus came from the keyboard (a click's focus shows no tip). */
export function keyboardFocus(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  try {
    return target.matches(":focus-visible");
  } catch {
    return true;
  }
}

/** A why-tip for one element: its ref, show/hide, the tip's id while it
 * shows, and the tip itself (portalled; `null` while hidden). */
export function useWhyTip<T extends HTMLElement>(why: string, testId: string) {
  const ref = useRef<T | null>(null);
  const tipId = `why-${useId().replace(/:/g, "")}`;
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);
  const show = useCallback(() => {
    const box = ref.current?.getBoundingClientRect();
    if (!box) return;
    setAt({ left: Math.max(8, Math.min(box.left, window.innerWidth - TIP_W - 8)), top: box.bottom + 6 });
  }, []);
  const hide = useCallback(() => setAt(null), []);
  useEffect(() => {
    if (!at) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") hide();
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", hide, true);
    window.addEventListener("wheel", hide, { passive: true });
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", hide, true);
      window.removeEventListener("wheel", hide);
    };
  }, [at, hide]);
  const tip: ReactNode =
    at &&
    createPortal(
      <span
        id={tipId}
        role="tooltip"
        style={{ left: at.left, top: at.top, width: TIP_W }}
        className="pointer-events-none fixed z-50 rounded-lg border border-hairline bg-surface px-2.5 py-1.5 text-label leading-snug text-ink shadow-card"
        data-testid={testId}
      >
        {why}
      </span>,
      document.body,
    );
  return { ref, show, hide, shown: at !== null, tipId, tip };
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "accent" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
  "data-testid"?: string;
};

/**
 * A `Button` that, while `why` is set, is greyed and says why: it keeps
 * its tab stop (`aria-disabled`, not `disabled`), carries the reason as
 * its description, shows it in a tip on hover and on keyboard focus, and
 * a press only shows the tip. With `why` null it is the plain button.
 */
export function WhyButton({ why, children, ...props }: ButtonProps & { why: string | null; children: ReactNode }): JSX.Element {
  if (why === null) return <Button {...props}>{children}</Button>;
  return (
    <Greyed why={why} {...props}>
      {children}
    </Greyed>
  );
}

function Greyed({ why, children, className = "", onClick: _onClick, disabled: _disabled, title: _title, ...props }: ButtonProps & { why: string; children: ReactNode }): JSX.Element {
  const { ref, show, hide, shown, tipId, tip } = useWhyTip<HTMLButtonElement>(why, `${props["data-testid"] ?? "why"}-tip`);
  return (
    <>
      <Button
        ref={ref}
        {...props}
        aria-disabled="true"
        aria-description={why}
        aria-describedby={shown ? tipId : undefined}
        className={`cursor-default opacity-40 ${className}`}
        onPointerEnter={show}
        onPointerLeave={hide}
        onFocus={(event) => keyboardFocus(event.currentTarget) && show()}
        onBlur={hide}
        onClick={(event) => {
          event.preventDefault();
          show();
        }}
      >
        {children}
      </Button>
      {tip}
    </>
  );
}
