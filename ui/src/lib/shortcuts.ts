/**
 * Inspector action shortcuts (AJ-E407): shared predicates so App wiring
 * and tests exercise the exact same rules.
 */

/** ⌘./Ctrl+. — Stop the selected live Instance (deep 02 SP-04). */
export function isStopShortcut(event: {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}): boolean {
  return (
    event.key === "." &&
    !event.shiftKey &&
    !event.altKey &&
    (event.metaKey || event.ctrlKey)
  );
}

/** ⌘⇧H/Ctrl+Shift+H — focus the hand-over section. */
export function isHandoverShortcut(event: {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}): boolean {
  return (
    event.key.toLowerCase() === "h" &&
    event.shiftKey &&
    !event.altKey &&
    (event.metaKey || event.ctrlKey)
  );
}
