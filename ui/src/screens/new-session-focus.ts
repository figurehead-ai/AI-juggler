/**
 * AJ-E804: opening New session moves focus into it (WCAG 2.4.3), onto
 * Start when it can, so Return starts the usual. 0.5.1 F49: the sheet now
 * opens at once (its code prefetched first), often before its App is
 * read, while Start is still disabled: focus then goes to the sheet's
 * first control and moves on to Start once it can take it, unless the
 * person has moved it themselves meanwhile.
 */
import { useEffect, useRef, type RefObject } from "react";

export function useStartFocus(sheet: RefObject<HTMLElement | null>, start: RefObject<HTMLButtonElement | null>, ready: boolean, focusable: string): void {
  // Where the open put focus while Start couldn't take it (null: done).
  const placed = useRef<Element | null>(null);
  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      if (sheet.current?.contains(document.activeElement)) return;
      const button = start.current;
      if (button && !button.disabled) return button.focus();
      const first = sheet.current?.querySelector<HTMLElement>(focusable) ?? null;
      first?.focus();
      placed.current = first ?? sheet.current;
    });
    return () => cancelAnimationFrame(frame);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => {
    const was = placed.current;
    if (!ready || was === null || !start.current || start.current.disabled) return;
    placed.current = null;
    const now = document.activeElement;
    if (now === was || now === document.body || !sheet.current?.contains(now)) start.current.focus();
  }, [ready, sheet, start]);
}
