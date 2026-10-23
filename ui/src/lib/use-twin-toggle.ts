/**
 * A disclosure whose button is replaced by its twin when it toggles (New
 * session's "Customise ▸" and "Customise ▾"): focus moves to the twin, so
 * it never drops to the page, where Esc and Tab no longer reach the sheet
 * (WCAG 2.4.3; AJ-E1535).
 */
import { useEffect, useRef, useState } from "react";

export function useTwinToggle() {
  const [open, setOpen] = useState(false);
  const toggled = useRef(false);
  const showRef = useRef<HTMLButtonElement | null>(null);
  const hideRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    if (toggled.current) (open ? hideRef : showRef).current?.focus();
  }, [open]);
  const toggle = (next: boolean) => {
    toggled.current = true;
    setOpen(next);
  };
  return { open, toggle, showRef, hideRef };
}
