/**
 * rc26 walk K1: the ⊖ stayed below a stopped session's terminal, over
 * "ran … · scrollback replayed". macOS moves the mark only with the
 * caret: an ended terminal gives focus up for good, and a focused one
 * whose frame moved gives it up and takes it back once the mark has gone
 * (WKWebView harness: ~120 ms at least; lib/caret-leaves parks the caret
 * in between).
 */
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { CARET_REFOCUS_MS, CARET_SETTLE_MS, followCaret, holdInput } from "./terminal-caret";

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
  document.body.replaceChildren();
});

function sink(got: string[] = []) {
  return { input: (data: string) => got.push(`input:${data}`), paste: (text: string) => got.push(`paste:${text}`) };
}

function field(): HTMLTextAreaElement {
  const element = document.createElement("textarea");
  document.body.append(element);
  return element;
}

it("an ended terminal lets focus go and can't take it again; a live one can", () => {
  const textarea = field();
  const term = { textarea, options: {} as { disableStdin?: boolean } };
  textarea.focus();
  holdInput(term, true);
  expect(document.activeElement).not.toBe(textarea);
  textarea.focus();
  expect(document.activeElement).not.toBe(textarea);
  expect(term.options.disableStdin).toBe(true);
  holdInput(term, false);
  textarea.focus();
  expect(document.activeElement).toBe(textarea);
});

it("a focused terminal whose frame moved gives focus up, then takes it back", () => {
  const textarea = field();
  let top = 100;
  textarea.getBoundingClientRect = () => new DOMRect(10, top);
  const trail: string[] = [];
  textarea.addEventListener("blur", () => trail.push("blur"));
  textarea.addEventListener("focus", () => trail.push("focus"));
  const caret = followCaret(textarea, sink(), () => true);
  textarea.focus();
  trail.length = 0;
  // A size change that didn't move it: nothing.
  caret.moved();
  vi.advanceTimersByTime(CARET_SETTLE_MS + CARET_REFOCUS_MS);
  expect(trail).toEqual([]);
  top = 79;
  caret.moved();
  vi.advanceTimersByTime(CARET_SETTLE_MS);
  expect(trail).toEqual(["blur"]);
  vi.advanceTimersByTime(CARET_REFOCUS_MS);
  expect(trail).toEqual(["blur", "focus"]);
  caret.dispose();
});

it("doesn't take focus back from what took it meanwhile, nor when it may not", () => {
  const textarea = field();
  let top = 100;
  textarea.getBoundingClientRect = () => new DOMRect(10, top);
  let may = true;
  const caret = followCaret(textarea, sink(), () => may);
  textarea.focus();
  top = 50;
  caret.moved();
  vi.advanceTimersByTime(CARET_SETTLE_MS);
  const other = document.createElement("button");
  document.body.append(other);
  other.focus();
  vi.advanceTimersByTime(CARET_REFOCUS_MS);
  expect(document.activeElement).toBe(other);
  textarea.focus();
  top = 20;
  may = false;
  caret.moved();
  vi.advanceTimersByTime(CARET_SETTLE_MS + CARET_REFOCUS_MS);
  expect(document.activeElement).toBe(document.body);
  caret.dispose();
});

it("nothing typed while the field is let go of is lost: held in order and handed over before it has focus again", () => {
  const textarea = field();
  let top = 100;
  textarea.getBoundingClientRect = () => new DOMRect(10, top);
  const got: string[] = [];
  textarea.addEventListener("keydown", (event) => got.push(`key:${event.key}${event.ctrlKey ? "+ctrl" : ""}`));
  textarea.addEventListener("focus", () => got.push("focus"));
  const caret = followCaret(textarea, sink(got), () => true);
  textarea.focus();
  got.length = 0;
  top = 79;
  caret.moved();
  vi.advanceTimersByTime(CARET_SETTLE_MS);
  expect(document.activeElement).toBe(document.body);
  const press = (init: KeyboardEventInit) => document.body.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));
  const app: string[] = [];
  const appKeys = (event: KeyboardEvent) => app.push(event.key);
  window.addEventListener("keydown", appKeys);
  press({ key: "l" });
  press({ key: "s" });
  press({ key: "Shift" });
  press({ key: "A", shiftKey: true });
  press({ key: "c", ctrlKey: true });
  const paste = Object.assign(new Event("paste", { bubbles: true, cancelable: true }), { clipboardData: { getData: () => " -la" } });
  document.body.dispatchEvent(paste);
  press({ key: "Enter" });
  // ⌘ keys stay the app's.
  press({ key: "k", metaKey: true });
  expect(app).toEqual(["Shift", "k"]);
  expect(got).toEqual([]);
  vi.advanceTimersByTime(CARET_REFOCUS_MS);
  expect(got).toEqual(["input:l", "input:s", "input:A", "key:c+ctrl", "paste: -la", "key:Enter", "focus"]);
  // Afterwards keys are the field's own again, not held.
  press({ key: "x" });
  expect(got.at(-1)).toBe("focus");
  window.removeEventListener("keydown", appKeys);
  caret.dispose();
});

it("a field the user focused meanwhile keeps its keys; ⌘V ends the gap so the paste lands in the terminal", () => {
  const textarea = field();
  let top = 100;
  textarea.getBoundingClientRect = () => new DOMRect(10, top);
  const got: string[] = [];
  const caret = followCaret(textarea, sink(got), () => true);
  textarea.focus();
  top = 50;
  caret.moved();
  vi.advanceTimersByTime(CARET_SETTLE_MS);
  document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "v", metaKey: true, bubbles: true }));
  expect(document.activeElement).toBe(textarea);
  const search = document.createElement("input");
  document.body.append(search);
  top = 20;
  caret.moved();
  vi.advanceTimersByTime(CARET_SETTLE_MS);
  search.focus();
  const typedInSearch = new KeyboardEvent("keydown", { key: "q", bubbles: true, cancelable: true });
  search.dispatchEvent(typedInSearch);
  expect(typedInSearch.defaultPrevented).toBe(false);
  vi.advanceTimersByTime(CARET_REFOCUS_MS);
  expect(got).toEqual([]);
  expect(document.activeElement).toBe(search);
  caret.dispose();
});
