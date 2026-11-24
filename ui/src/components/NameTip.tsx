/**
 * 0.5.1 F61 — an icon's name, drawn: on hover AND on keyboard focus, in a
 * tip over the page beside the icon (WhyTip's pattern: portalled, so a
 * clipped column can't cut it; Escape, a scroll or the wheel closes it).
 * A native `title` is shown by the app's web view only after a still
 * hover in a key window and never on focus, so the folded rail's icons
 * had no name a sighted keyboard user could see. The name stays the
 * control's accessible name (`aria-label`); the tip only shows it.
 */
import { useCallback, useEffect, useRef, useState, type ButtonHTMLAttributes, type JSX, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { keyboardFocus } from "./WhyTip";

/** The tip's drawing, as the why-tip's. */
const TIP = "pointer-events-none fixed z-50 w-max max-w-64 rounded-lg border border-hairline bg-overlay px-2.5 py-1.5 text-label leading-snug text-ink shadow-popover";

/** A name tip for one element, drawn to its right: its ref, show/hide,
 * and the tip (`null` while hidden). */
export function useNameTip<T extends HTMLElement>(words: string, testId: string) {
  const ref = useRef<T | null>(null);
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);
  const show = useCallback(() => {
    const box = ref.current?.getBoundingClientRect();
    if (!box) return;
    setAt({ left: box.right + 8, top: box.top + box.height / 2 });
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
      <span role="tooltip" style={{ left: at.left, top: at.top, transform: "translateY(-50%)" }} className={TIP} data-testid={testId}>
        {words}
      </span>,
      document.body,
    );
  return { ref, show, hide, shown: at !== null, tip };
}

type Props = ButtonHTMLAttributes<HTMLButtonElement> & { "data-testid"?: string };

/**
 * A button whose name is drawn beside it on hover and keyboard focus
 * (`tip`); with `tip` null it is the plain button. Never a `title`.
 */
export function TipButton({ tip, children, onPointerEnter, onPointerLeave, onFocus, onBlur, ...props }: Props & { tip: string | null; children: ReactNode }): JSX.Element {
  const { ref, show, hide, tip: drawn } = useNameTip<HTMLButtonElement>(tip ?? "", `${props["data-testid"] ?? "button"}-tip`);
  if (tip === null) {
    return (
      <button type="button" onPointerEnter={onPointerEnter} onPointerLeave={onPointerLeave} onFocus={onFocus} onBlur={onBlur} {...props}>
        {children}
      </button>
    );
  }
  return (
    <>
      <button
        type="button"
        ref={ref}
        {...props}
        onPointerEnter={(event) => {
          show();
          onPointerEnter?.(event);
        }}
        onPointerLeave={(event) => {
          hide();
          onPointerLeave?.(event);
        }}
        onFocus={(event) => {
          if (keyboardFocus(event.currentTarget)) show();
          onFocus?.(event);
        }}
        onBlur={(event) => {
          hide();
          onBlur?.(event);
        }}
      >
        {children}
      </button>
      {drawn}
    </>
  );
}
