/**
 * 0.5.1 F61 — an icon's name, drawn: on hover AND on keyboard focus, in a
 * tip over the page beside the icon (WhyTip's pattern: portalled, so a
 * clipped column can't cut it; Escape, a scroll or the wheel closes it).
 * A native `title` is shown by the app's web view only after a still
 * hover in a key window and never on focus, so the folded rail's icons
 * had no name a sighted keyboard user could see. The name stays the
 * control's accessible name (`aria-label`); the tip only shows it.
 */
import { type ButtonHTMLAttributes, type JSX, type ReactNode } from "react";
import { TipLayer, keyboardFocus, useTipSpot } from "./WhyTip";

/** The name tip's place: to the element's right, centred on it. */
function beside(box: DOMRect): { left: number; top: number } {
  return { left: box.right + 8, top: box.top + box.height / 2 };
}

/** A name tip for one element, drawn to its right: its ref, show/hide,
 * and the tip (`null` while hidden). */
function useNameTip<T extends HTMLElement>(words: string, testId: string) {
  const { ref, at, show, hide } = useTipSpot<T>(beside);
  const tip: ReactNode = at && (
    <TipLayer at={at} centred testId={testId}>
      {words}
    </TipLayer>
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
