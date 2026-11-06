/**
 * AJ-E2905: a control's keys as they are now — the user's change from
 * Settings › Keyboard, or as shipped — re-read when one changes, for its
 * `title` and `aria-keyshortcuts`.
 */
import { useEffect, useState } from "react";
import { onBindingsChanged } from "./shortcut-bindings";
import { ariaKeysOf, keysOf, type ShortcutId } from "./shortcuts";

export function useKeys(id: ShortcutId): {
  words: string;
  aria: string | undefined;
} {
  const [, bump] = useState(0);
  useEffect(() => onBindingsChanged(() => bump((n) => n + 1)), []);
  return { words: keysOf(id)[0] ?? "", aria: ariaKeysOf(id) };
}
