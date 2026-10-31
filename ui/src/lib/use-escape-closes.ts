/**
 * Esc closes a modal sheet wherever focus is. On the sheet's own element
 * an Esc was lost while focus hadn't reached the sheet yet (an Esc right
 * after ⌘N, #198). The listener is set in a layout effect, so it hears
 * Esc from the sheet's first paint; an Esc something inside the sheet
 * already handled (a menu closing itself) is left alone.
 */
import { useLayoutEffect, useRef } from "react";

export function useEscapeCloses(onClose: () => void): void {
  const close = useRef(onClose);
  close.current = onClose;
  useLayoutEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) close.current();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
