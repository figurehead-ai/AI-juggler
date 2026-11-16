/**
 * AJ-E1005 Req.3 and AJ-E2910 — Home's row keys (moved out of Home.tsx,
 * the size budget): J/K or the arrows move between rows, Enter opens
 * (each row's first control is a button), A runs the row's action (also
 * from the row's own action button, Mark reviewed); r, h, s and e on the
 * focused row. The keys come from the registry (lib/shortcuts.ts), with
 * its typing rule: never while typing in a field or the terminal. A row
 * is `[data-home-row]` inside its `[data-home-item]` (a pinned need, a
 * parked session, a feed card).
 */
import { useEffect, useRef, type FocusEvent, type KeyboardEvent, type MutableRefObject } from "react";
import { focusHandoverSoon } from "./handover-focus";
import { matches, type KeyLike } from "./shortcuts";

export function useHomeKeys({ busy, restoreUntil, onOpenSession, onArchive }: {
  busy: string | null;
  /** Until when focus goes back to the row at its place (a row that
   * Mark reviewed or Resume removed). */
  restoreUntil: MutableRefObject<number>;
  onOpenSession: (id: string) => void;
  onArchive: (sessionId: string) => void;
}): { attach: (node: HTMLDivElement | null) => void; onKeyDown: (event: KeyboardEvent<HTMLDivElement>) => void; onFocus: (event: FocusEvent<HTMLDivElement>) => void } {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const rowsNow = () => Array.from(rootRef.current?.querySelectorAll<HTMLElement>("[data-home-row]") ?? []);
  const rowIndexOf = (el: Element | null): number => {
    const item = el?.closest("[data-home-item]");
    return item ? rowsNow().findIndex((row) => row.closest("[data-home-item]") === item) : -1;
  };
  // AJ-E1504: the keys come from the registry (lib/shortcuts.ts), with
  // its typing rule: never while typing in a field or the terminal.
  const handleKey = (event: KeyLike & { target: EventTarget | null; preventDefault: () => void }) => {
    if (event.target instanceof HTMLElement && event.target.tagName === "SELECT") return;
    const rows = rowsNow();
    const at = rowIndexOf(document.activeElement);
    const focusRow = (index: number) => rows[Math.max(0, Math.min(rows.length - 1, index))]?.focus();
    // The arrows move only once a row has focus (J and K start it).
    const arrow = event.key.startsWith("Arrow");
    if (matches("home-next", event) && (!arrow || at >= 0)) {
      event.preventDefault();
      focusRow(at + 1);
    } else if (matches("home-prev", event) && (!arrow || at >= 0)) {
      event.preventDefault();
      focusRow(at < 0 ? 0 : at - 1);
    } else if (matches("home-action", event) && at >= 0) {
      const item = rows[at]!.closest("[data-home-item]");
      const action = item?.querySelector<HTMLElement>("[data-home-action]") ?? rows[at]!;
      event.preventDefault();
      action.click();
    } else if (at >= 0) rowKey(event, rows[at]!);
  };
  // AJ-E2910: r, h, s and e on the focused row (a key that doesn't apply
  // to it does nothing; its ⋯ says why).
  const rowKey = (event: KeyLike & { preventDefault: () => void }, row: HTMLElement) => {
    const item = row.closest<HTMLElement>("[data-home-item]");
    const id = item?.dataset.sessionId;
    if (matches("home-reply", event)) {
      event.preventDefault();
      row.click();
    } else if (matches("home-handover", event) && id && item?.dataset.hasConversation === "yes") {
      event.preventDefault();
      onOpenSession(id);
      focusHandoverSoon(id);
    } else if (matches("home-snooze", event) && item) {
      event.preventDefault();
      item.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    } else if (matches("home-archive", event) && id && item?.dataset.ended === "yes") {
      event.preventDefault();
      onArchive(id);
    }
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => handleKey(event);
  // Marking a row reviewed or resuming it removes or disables it, and its
  // focus falls to <body>: put focus back on the row now at that place,
  // and keep the keys working from <body> while Home is shown.
  const lastRow = useRef<number | null>(null);
  useEffect(() => {
    if (busy !== null || Date.now() > restoreUntil.current || lastRow.current === null) return;
    if (document.activeElement !== document.body) return;
    const rows = rowsNow();
    if (rows.length > 0) rows[Math.min(lastRow.current, rows.length - 1)]!.focus();
  });
  useEffect(() => {
    const fromBody = (event: globalThis.KeyboardEvent) => {
      // Never behind a dialog: a modal over Home (the limit dialog, the
      // usage-stats question) owns the keys, even when a click on its
      // text dropped focus to <body>.
      if (event.defaultPrevented || document.querySelector('[aria-modal="true"]')) return;
      if (document.activeElement === document.body) handleKey(event);
    };
    window.addEventListener("keydown", fromBody);
    return () => window.removeEventListener("keydown", fromBody);
  });
  const onFocus = (event: FocusEvent<HTMLDivElement>) => {
    const index = rowIndexOf(event.target);
    if (index >= 0) lastRow.current = index;
  };
  const attach = (node: HTMLDivElement | null) => {
    rootRef.current = node;
  };
  return { attach, onKeyDown, onFocus };
}
