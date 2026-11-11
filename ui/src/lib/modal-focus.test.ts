/**
 * rc26 walk K1: a dialog opened from a focused terminal (⌘⇧N's New
 * project sets no first focus) left the terminal's field focused behind
 * its scrim, and macOS's ⊖ sat sharp over it. A dialog that appears takes
 * focus from a field behind it, a frame after its own first focus.
 */
import { afterEach, beforeEach, expect, it } from "vitest";
import { guardCaretMark } from "./caret-leaves";
import { guardModalFocus, takeFocusFromBehind } from "./modal-focus";

let undo: Array<() => void> = [];
beforeEach(() => {
  undo = [guardCaretMark(), guardModalFocus()];
});
afterEach(() => {
  undo.forEach((off) => off());
  document.body.replaceChildren();
});

const frames = () => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(done, 0))));

function terminal(): HTMLTextAreaElement {
  const field = document.createElement("textarea");
  document.body.append(field);
  field.focus();
  return field;
}

function dialog(...controls: HTMLElement[]): HTMLElement {
  const sheet = document.createElement("div");
  sheet.setAttribute("role", "dialog");
  sheet.setAttribute("aria-modal", "true");
  sheet.append(...controls);
  document.body.append(sheet);
  return sheet;
}

function button(words: string): HTMLButtonElement {
  const control = document.createElement("button");
  control.textContent = words;
  return control;
}

it("takes focus from a field behind a dialog that sets none of its own, and the caret leaves", async () => {
  const field = terminal();
  const sheet = dialog(button("First"), button("Second"));
  await frames();
  expect(document.activeElement).toBe(sheet.firstElementChild);
  expect(field.contains(document.getSelection()?.anchorNode ?? null)).toBe(false);
});

it("goes to what the dialog marks first, else the dialog itself", async () => {
  terminal();
  const marked = button("Marked");
  marked.setAttribute("data-autofocus", "");
  dialog(button("First"), marked);
  await frames();
  expect(document.activeElement).toBe(marked);
  document.body.replaceChildren();
  terminal();
  const bare = dialog();
  await frames();
  expect(document.activeElement).toBe(bare);
});

it("leaves a dialog's own first focus, and focus that is not in a field", async () => {
  terminal();
  const second = button("Second");
  dialog(button("First"), second);
  second.focus();
  await frames();
  expect(document.activeElement).toBe(second);
  document.body.replaceChildren();
  const outside = button("Outside");
  document.body.append(outside);
  outside.focus();
  dialog(button("First"));
  await frames();
  expect(document.activeElement).toBe(outside);
});

it("a field inside the dialog keeps focus", () => {
  const inside = document.createElement("input");
  dialog(inside);
  inside.focus();
  expect(takeFocusFromBehind()).toBe(false);
  expect(document.activeElement).toBe(inside);
});
