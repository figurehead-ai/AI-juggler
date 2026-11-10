/**
 * rc24/rc25 walks K1: a text field that loses focus to nothing editable
 * leaves its caret on a parked spot that isn't editable, so WKWebView
 * hides macOS's input-source mark (lib/caret-leaves says why); a field
 * whose window only went to the back keeps it, and the next field takes
 * it by itself.
 */
import { afterEach, beforeEach, expect, it } from "vitest";
import { caretLeaves, guardCaretMark, isTextField } from "./caret-leaves";

let undo: () => void = () => {};
beforeEach(() => {
  undo = guardCaretMark();
});
afterEach(() => {
  undo();
  document.body.replaceChildren();
  document.getSelection()?.removeAllRanges();
});

const parked = () => document.getSelection()?.anchorNode?.parentElement?.hasAttribute("data-caret-park") ?? false;

function field(tag: "textarea" | "input" = "textarea"): HTMLElement {
  const element = document.createElement(tag);
  document.body.append(element);
  return element;
}

it("parks the caret off any field when a field loses focus to nothing", () => {
  const terminal = field();
  terminal.focus();
  terminal.blur();
  expect(document.activeElement).toBe(document.body);
  expect(document.getSelection()?.type).toBe("Caret");
  expect(parked()).toBe(true);
  const park = document.querySelector("[data-caret-park]")!;
  expect(park.getAttribute("aria-hidden")).toBe("true");
  // One spot, however often.
  terminal.focus();
  terminal.blur();
  expect(document.querySelectorAll("[data-caret-park]")).toHaveLength(1);
});

it("takes the caret off the field when focus goes to something that isn't one, leaving focus there", () => {
  const terminal = field();
  const main = document.createElement("main");
  main.tabIndex = -1;
  document.body.append(main);
  terminal.focus();
  main.focus();
  expect(document.activeElement).toBe(main);
  // (jsdom then moves the caret to main itself; WebKit keeps it parked.)
  const at = document.getSelection()?.anchorNode;
  expect(document.getSelection()?.type).toBe("Caret");
  expect(at === terminal || terminal.contains(at ?? null)).toBe(false);
});

it("leaves the caret to the next field", () => {
  const first = field("input");
  const next = field();
  first.focus();
  next.focus();
  expect(document.activeElement).toBe(next);
  expect(parked()).toBe(false);
});

it("leaves a field the window only lost with the app", () => {
  const terminal = field();
  // The app went to the back: the field is still the focused one.
  terminal.focus();
  expect(caretLeaves({ target: terminal, relatedTarget: null })).toBe(false);
});

it("knows a field: text-like inputs, text areas, editable text", () => {
  const checkbox = Object.assign(document.createElement("input"), { type: "checkbox" });
  const search = Object.assign(document.createElement("input"), { type: "search" });
  expect([isTextField(checkbox), isTextField(search), isTextField(document.createElement("textarea")), isTextField(null)]).toEqual([false, true, true, false]);
  // (jsdom has no isContentEditable: a button says undefined there.)
  expect(isTextField(document.createElement("button"))).toBeFalsy();
});

it("is undone", () => {
  undo();
  const terminal = field();
  terminal.focus();
  terminal.blur();
  expect(parked()).toBe(false);
});
