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

/** Where a tip is drawn, in the window's coordinates. */
type Spot = { left: number; top: number };

/**
 * A tip anchored to one element (the why-tip's and the name tip's
 * shared part): its ref, where the tip goes while it shows (`null` while
 * hidden, from `place`, a stable function of the element's box), and
 * show/hide. Escape, a scroll or the wheel hides it.
 */
export function useTipSpot<T extends HTMLElement>(place: (box: DOMRect) => Spot) {
  const ref = useRef<T | null>(null);
  const [at, setAt] = useState<Spot | null>(null);
  const show = useCallback(() => {
    const box = ref.current?.getBoundingClientRect();
    if (!box) return;
    setAt(place(box));
  }, [place]);
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
  return { ref, at, show, hide };
}

/** The why-tip's place: under the element, kept inside the window. */
export function below(box: DOMRect): Spot {
  return { left: Math.max(8, Math.min(box.left, window.innerWidth - TIP_W - 8)), top: box.bottom + 6 };
}

/** Under the element, its right edge at the element's (a control at a
 * panel's right end), kept inside the window. */
export function belowEnd(box: DOMRect): Spot {
  return { left: Math.max(8, Math.min(box.right - TIP_W, window.innerWidth - TIP_W - 8)), top: box.bottom + 6 };
}

/**
 * AJ-E1556: every tip's one look. Drawn over the page (portalled, the
 * popover layer: nothing in a panel, a count or a sticky head is drawn
 * over it, no column clips it), its words wrapping to its width, a long
 * word broken rather than cut, never ellipsised (the 0.5.1 Nightly: Bot's
 * New topic tip read "Nothing is do…" with a "23" over it).
 */
export const TIP_LOOK =
  "pointer-events-none fixed z-50 whitespace-normal wrap-break-word rounded-lg border border-hairline bg-overlay px-2.5 py-1.5 text-left text-label leading-snug text-ink shadow-popover";

/** AJ-E1556: a drawn tip kept inside the window (8 px in from each edge),
 * moved after it is laid out, once its real size is known. */
export function keepInWindow(tip: HTMLElement | null): void {
  if (!tip) return;
  tip.style.translate = "";
  const box = tip.getBoundingClientRect();
  const edge = 8;
  const dx = box.right > window.innerWidth - edge ? window.innerWidth - edge - box.right : box.left < edge ? edge - box.left : 0;
  const dy = box.bottom > window.innerHeight - edge ? window.innerHeight - edge - box.bottom : box.top < edge ? edge - box.top : 0;
  if (dx !== 0 || dy !== 0) tip.style.translate = `${Math.round(dx)}px ${Math.round(dy)}px`;
}

/** A tip's drawing, at `at` (`centred`: its middle there, beside what it
 * names): the one look, kept inside the window. */
export function TipLayer({ at, id, testId, width, centred = false, role = "tooltip", children }: { at: Spot; id?: string; testId: string; width?: number; centred?: boolean; role?: "tooltip" | "presentation"; children: ReactNode }): ReactNode {
  return createPortal(
    <span id={id} ref={keepInWindow} role={role} aria-hidden={role === "presentation" ? "true" : undefined} style={{ left: at.left, top: at.top, width, transform: centred ? "translateY(-50%)" : undefined }} className={TIP_LOOK + (width ? "" : " w-max max-w-64")} data-testid={testId} data-tip="">
      {children}
    </span>,
    document.body,
  );
}

/** To the element's right, its top at the element's (an icon in the
 * folded rail), kept inside the window once drawn. */
export function besideTop(box: DOMRect): Spot {
  return { left: box.right + 8, top: box.top };
}

/** A tip for one element (under it, unless `place` says otherwise): its
 * ref, show/hide, the tip's id while it shows, and the tip itself
 * (portalled; `null` while hidden). `role` "presentation": a picture of
 * words the element's name already says (hidden from screen readers). */
export function useTip<T extends HTMLElement>(content: ReactNode, testId: string, place: (box: DOMRect) => Spot = below, role: "tooltip" | "presentation" = "tooltip") {
  const { ref, at, show, hide } = useTipSpot<T>(place);
  const tipId = `tip-${useId().replace(/:/g, "")}`;
  const tip: ReactNode = at && (
    <TipLayer at={at} id={tipId} testId={testId} width={TIP_W} role={role}>
      {content}
    </TipLayer>
  );
  return { ref, show, hide, shown: at !== null, tipId, tip };
}

/** A why-tip for one element: its ref, show/hide, the tip's id while it
 * shows, and the tip itself (portalled; `null` while hidden). */
export function useWhyTip<T extends HTMLElement>(why: string, testId: string) {
  return useTip<T>(why, testId);
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
