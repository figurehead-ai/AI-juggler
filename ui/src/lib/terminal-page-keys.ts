/**
 * An ended terminal's keys on its page (rc26 K1 follow-up). An ended
 * terminal's field never holds focus (lib/terminal-caret holdInput), so
 * the keys xterm read from it — ⌘F to find, ⌘+ ⌘− ⌘0 for its text size —
 * and ⌘C to copy are read here instead, at the page, for one terminal
 * picked by its id: the one focus is in, else the only ended one on
 * screen. Never while a dialog is open or a text field has focus (its
 * keys are its own), and ⌘C only when the page has no selection of its
 * own. ⌘↩ stays the page's main button.
 */
import { useEffect, useRef, type RefObject } from "react";
import { isTextField } from "./caret-leaves";
import { terminalKey, type TerminalKey } from "./terminalTools";

type PageKeys = { host: HTMLElement; tool: (key: TerminalKey) => void; copy: () => void };

const ended = new Map<string, PageKeys>();

/** The ended terminal a key on the page is for, by id. */
export function pageKeysTarget(doc: Document = document): string | null {
  if (doc.querySelector('[aria-modal="true"]') || isTextField(doc.activeElement)) return null;
  const rootOf = (keys: PageKeys) => keys.host.closest('[data-testid="terminal"]') ?? keys.host;
  const entries = [...ended.entries()];
  const within = entries.find(([, keys]) => doc.activeElement && rootOf(keys).contains(doc.activeElement));
  if (within) return within[0];
  const shown = entries.filter(([, keys]) => keys.host.getClientRects().length > 0);
  return shown.length === 1 ? shown[0]![0] : null;
}

function onKey(event: KeyboardEvent): void {
  if (event.defaultPrevented) return;
  const id = pageKeysTarget();
  const keys = id === null ? undefined : ended.get(id);
  if (!keys) return;
  const tool = terminalKey(event);
  const copy = event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "c" && !document.getSelection()?.toString();
  if ((!tool || tool === "maximize") && !copy) return;
  event.preventDefault();
  if (tool && tool !== "maximize") keys.tool(tool);
  else keys.copy();
}

/** While `active` (the terminal ended), its page's keys reach it. */
export function usePageKeys(id: string, hostRef: RefObject<HTMLElement | null>, active: boolean, tool: RefObject<(key: TerminalKey) => void>, copy: () => void): void {
  const copyRef = useRef(copy);
  useEffect(() => {
    copyRef.current = copy;
  });
  useEffect(() => {
    const host = hostRef.current;
    if (!active || !host) return undefined;
    if (ended.size === 0) window.addEventListener("keydown", onKey);
    ended.set(id, { host, tool: (key) => tool.current(key), copy: () => copyRef.current() });
    return () => {
      ended.delete(id);
      if (ended.size === 0) window.removeEventListener("keydown", onKey);
    };
  }, [id, hostRef, active, tool]);
}
