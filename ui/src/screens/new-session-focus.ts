/**
 * AJ-E804: opening New session moves focus into it (WCAG 2.4.3), onto
 * Start when it can, so Return starts the usual. 0.5.1 F49: the sheet now
 * opens at once (its code prefetched first), often before its App is
 * read, while Start is still disabled: focus then goes to the sheet's
 * first control and moves on to Start once it can take it, unless the
 * person has moved it themselves meanwhile. Opened at once, the window's
 * modal guard (lib/inert) has put focus on that first control already;
 * the sheet's own choice wins over it.
 */
import { useEffect, useRef, type RefObject } from "react";

export function useStartFocus(sheet: RefObject<HTMLElement | null>, start: RefObject<HTMLButtonElement | null>, ready: boolean, focusable: string): void {
  // Where the open put focus while Start couldn't take it (null: done).
  const placed = useRef<Element | null>(null);
  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      // The window's modal guard (lib/inert) puts focus on a new dialog's
      // first control when it opens from the page; the sheet's own first
      // focus wins over that (as the guard says), not over another choice.
      const first = sheet.current?.querySelector<HTMLElement>(focusable) ?? null;
      const now = document.activeElement;
      if (now !== first && sheet.current?.contains(now)) return;
      const button = start.current;
      if (button && !button.disabled) return button.focus();
      first?.focus();
      placed.current = first ?? sheet.current;
    });
    return () => cancelAnimationFrame(frame);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  // After every render: Start may come (or turn on) without `ready` changing.
  useEffect(() => {
    const was = placed.current;
    if (!ready || was === null || !start.current || start.current.disabled) return;
    placed.current = null;
    const now = document.activeElement;
    if (now === was || now === document.body || !sheet.current?.contains(now)) start.current.focus();
  });
}
