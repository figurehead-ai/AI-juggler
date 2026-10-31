import { afterEach, describe, expect, it } from "vitest";
import { contextMenuFor, guardContextMenu } from "./context-menu";

afterEach(() => {
  document.body.innerHTML = "";
  window.getSelection()?.removeAllRanges();
});

const at = (element: Element, extra: Partial<MouseEvent> = {}) => ({ defaultPrevented: false, altKey: false, target: element, ...extra }) as MouseEvent;

describe("0.3.2 right-click", () => {
  it("shows no web-view menu (Reload) on plain UI, such as a group header", () => {
    const header = document.body.appendChild(document.createElement("button"));
    expect(contextMenuFor(at(header), false)).toBe("none");
    const undo = guardContextMenu(() => false);
    const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
    header.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    undo();
  });

  it("keeps it in text fields, the terminal and on selected text", () => {
    const field = document.body.appendChild(document.createElement("input"));
    expect(contextMenuFor(at(field), false)).toBe("native");
    const term = document.body.appendChild(document.createElement("div"));
    term.className = "xterm";
    expect(contextMenuFor(at(term.appendChild(document.createElement("canvas"))), false)).toBe("native");
    const para = document.body.appendChild(document.createElement("p"));
    para.textContent = "some words to copy";
    const range = document.createRange();
    range.selectNodeContents(para);
    window.getSelection()!.addRange(range);
    expect(contextMenuFor(at(para), false)).toBe("native");
  });

  it("leaves a row's own menu alone, and ⌥ keeps the web view's only in a Dev build", () => {
    const row = document.body.appendChild(document.createElement("div"));
    expect(contextMenuFor(at(row, { defaultPrevented: true }), false)).toBe("taken");
    expect(contextMenuFor(at(row, { altKey: true }), true)).toBe("native");
    expect(contextMenuFor(at(row, { altKey: true }), false)).toBe("none");
  });
});
