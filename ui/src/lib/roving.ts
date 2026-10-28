/**
 * The rail's list as one stop on the Tab path (deep/11 §9: roving
 * tabindex in lists; the 0.2 walk, F11, counted 84 Tabs to get past the
 * rail: every row was four stops).
 *
 * A list is made of lines: a row (`data-rail-row`), a line the board drew
 * (`data-rail-line`, e.g. a place's header, the "Also:" line), or a lone
 * control. Tab reaches one line, the one last used (else the open page's
 * row, else the first), and the controls in it after its row (§9: "action
 * buttons inside row reachable by Tab after row"); every other line's
 * controls are out of the Tab path. ↑ and ↓ move between lines, Home and
 * End to the ends. Menus and dialogs a row opens keep their own keys.
 *
 * It works on the list as drawn (rows come and go as the list scrolls and
 * changes), so it sets `tabIndex` on what the rail's components render;
 * they set none of their own.
 */
import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";

const FOCUSABLE = "a[href], button, input, select, textarea, summary, [tabindex]";
const LINE = "[data-rail-row], [data-rail-line]";
/** A line's own way in: its row's button, a header's fold, else its first control. */
const STOP = "[data-rail-item], [data-rail-nav]";
/** Marks the controls whose `tabIndex` this sets. */
const OURS = "data-roving";
/** What a row opens keeps its own keys. */
const OWN_KEYS = "[role='menu'], [role='dialog'], [role='listbox'], [role='tooltip']";

function controls(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((element) => {
    if ((element as HTMLButtonElement).disabled) return false;
    // Out of the Tab path on purpose (a focus target), unless it's ours.
    if (element.getAttribute("tabindex") === "-1" && !element.hasAttribute(OURS)) return false;
    const own = element.closest(OWN_KEYS);
    if (own && root.contains(own)) return false;
    const hidden = element.closest("[hidden]");
    return !(hidden && root.contains(hidden));
  });
}

function lineOf(element: HTMLElement, root: HTMLElement): HTMLElement {
  const line = element.closest<HTMLElement>(LINE);
  return line && line !== root && root.contains(line) ? line : element;
}

/** The control a line is entered by. */
export function stopOf(line: HTMLElement): HTMLElement | null {
  if (line.matches(FOCUSABLE)) return line;
  return line.querySelector<HTMLElement>(STOP) ?? line.querySelector<HTMLElement>(FOCUSABLE);
}

/** The lines in the list, in order, each with its controls. */
export function linesOf(root: HTMLElement): { line: HTMLElement; controls: HTMLElement[] }[] {
  const out: { line: HTMLElement; controls: HTMLElement[] }[] = [];
  for (const control of controls(root)) {
    const line = lineOf(control, root);
    const last = out[out.length - 1];
    if (last?.line === line) last.controls.push(control);
    else {
      const earlier = out.find((entry) => entry.line === line);
      if (earlier) earlier.controls.push(control);
      else out.push({ line, controls: [control] });
    }
  }
  return out;
}

/** What names a line across renders: its row's key, else its stop's words. */
function keyOf(line: HTMLElement): string {
  const stop = stopOf(line);
  return stop?.dataset.railItem ?? stop?.getAttribute("aria-label") ?? stop?.textContent?.trim() ?? "";
}

/** Puts one line in the Tab path: the focused one, else the last one
 * used (`remembered`), else the open page's row, else the first. Returns
 * the focused line's key (what to remember), else `remembered`. */
export function rove(root: HTMLElement, remembered: string | null): string | null {
  const lines = linesOf(root);
  if (lines.length === 0) return remembered;
  const focus = document.activeElement;
  const focused = lines.find((entry) => focus instanceof Node && entry.line.contains(focus));
  const active =
    focused ??
    (remembered === null ? undefined : lines.find((entry) => keyOf(entry.line) === remembered)) ??
    lines.find((entry) => entry.line.matches("[data-rail-selected='true']") || entry.line.querySelector("[data-rail-selected='true']") !== null) ??
    lines[0]!;
  for (const entry of lines) {
    const index = entry === active ? 0 : -1;
    for (const control of entry.controls) {
      if (control.tabIndex !== index) control.tabIndex = index;
      control.setAttribute(OURS, "");
    }
  }
  return focused ? keyOf(focused.line) : remembered;
}

/** Moves focus `step` lines from the focused one (or to the first or last
 * with `"first"`/`"last"`). Returns false when there is no line there. */
export function moveLine(root: HTMLElement, step: 1 | -1 | "first" | "last"): boolean {
  const lines = linesOf(root);
  const focus = document.activeElement;
  const at = lines.findIndex((entry) => focus instanceof Node && entry.line.contains(focus));
  if (at < 0 && typeof step === "number") return false;
  const index = step === "first" ? 0 : step === "last" ? lines.length - 1 : at + step;
  const next = lines[index];
  if (!next) return false;
  (stopOf(next.line) ?? next.controls[0])?.focus();
  return true;
}

/** Whether `target` is a control of one of the list's lines (not a
 * field, nor a menu or dialog a row opened). */
export function onLine(root: HTMLElement, target: EventTarget | null): boolean {
  const element = target instanceof HTMLElement ? target : null;
  if (!element || element.closest(OWN_KEYS) || element.matches("input, textarea, select")) return false;
  return linesOf(root).some((entry) => entry.controls.includes(element));
}

/** ↑ ↓ Home End inside the list; true when the key was used. */
export function roveKey(root: HTMLElement, event: { key: string; altKey: boolean; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean; target: EventTarget | null }): boolean {
  if (event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return false;
  const target = event.target as HTMLElement | null;
  if (target?.closest(OWN_KEYS) || target?.matches("input, textarea, select")) return false;
  switch (event.key) {
    case "ArrowDown":
      return moveLine(root, 1);
    case "ArrowUp":
      return moveLine(root, -1);
    case "Home":
      return moveLine(root, "first");
    case "End":
      return moveLine(root, "last");
    default:
      return false;
  }
}

/** Keeps `root`'s list one stop on the Tab path while it renders and
 * changes, and moves through it with ↑ ↓ Home End (a key the list's own
 * handler already used is left alone). */
export function useRoving(root: RefObject<HTMLElement | null>): void {
  const remembered = useRef<string | null>(null);
  // After every render of the owner: what it drew is in place.
  useLayoutEffect(() => {
    if (root.current) remembered.current = rove(root.current, remembered.current);
  });
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const again = () => {
      remembered.current = rove(element, remembered.current);
    };
    // Rows its children draw on their own (a place's board, a menu).
    const observer = typeof MutationObserver === "undefined" ? null : new MutationObserver(again);
    observer?.observe(element, { childList: true, subtree: true });
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      if (roveKey(element, event)) event.preventDefault();
    };
    element.addEventListener("focusin", again);
    element.addEventListener("keydown", onKey);
    return () => {
      observer?.disconnect();
      element.removeEventListener("focusin", again);
      element.removeEventListener("keydown", onKey);
    };
  }, [root]);
}
