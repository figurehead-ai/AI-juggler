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
  const caret = followCaret(textarea, () => true);
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
  const caret = followCaret(textarea, () => may);
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
