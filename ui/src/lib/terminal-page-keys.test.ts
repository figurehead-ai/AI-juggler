/**
 * An ended terminal's field never holds focus, so its ⌘F and ⌘C are read
 * at the page, for one terminal picked by id: the one focus is in, else
 * the only ended one on screen; never over a dialog or a text field.
 */
import { renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { pageKeysTarget, usePageKeys } from "./terminal-page-keys";
import type { TerminalKey } from "./terminalTools";

afterEach(() => document.body.replaceChildren());

function terminal(id: string) {
  const root = document.createElement("div");
  root.dataset.testid = "terminal";
  const host = document.createElement("div");
  const toolbar = document.createElement("button");
  root.append(toolbar, host);
  document.body.append(root);
  // jsdom lays nothing out: a terminal on screen has a box.
  Object.defineProperty(host, "getClientRects", { value: () => ({ length: 1 }) });
  const tool = vi.fn<(key: TerminalKey) => void>();
  const copy = vi.fn();
  const view = renderHook(() => usePageKeys(id, { current: host }, true, { current: tool }, copy));
  return { root, host, toolbar, tool, copy, view };
}

const press = (init: KeyboardEventInit) => window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));

it("⌘F finds and ⌘C copies in the only ended terminal on the page", () => {
  const one = terminal("t1");
  press({ key: "f", code: "KeyF", metaKey: true });
  expect(one.tool).toHaveBeenCalledWith("find");
  press({ key: "c", code: "KeyC", metaKey: true });
  expect(one.copy).toHaveBeenCalledTimes(1);
  // ⌘↩ stays the page's main button.
  press({ key: "Enter", metaKey: true });
  expect(one.tool).toHaveBeenCalledTimes(1);
  one.view.unmount();
  press({ key: "f", code: "KeyF", metaKey: true });
  expect(one.tool).toHaveBeenCalledTimes(1);
});

it("with two on screen, the keys go to the one focus is in, by id; none to a field or over a dialog", () => {
  const one = terminal("t1");
  const two = terminal("t2");
  expect(pageKeysTarget()).toBeNull();
  two.toolbar.focus();
  expect(pageKeysTarget()).toBe("t2");
  press({ key: "f", code: "KeyF", metaKey: true });
  expect(two.tool).toHaveBeenCalledWith("find");
  expect(one.tool).not.toHaveBeenCalled();
  const field = document.createElement("input");
  document.body.append(field);
  field.focus();
  expect(pageKeysTarget()).toBeNull();
  two.toolbar.focus();
  const dialog = document.createElement("div");
  dialog.setAttribute("aria-modal", "true");
  document.body.append(dialog);
  expect(pageKeysTarget()).toBeNull();
  one.view.unmount();
  two.view.unmount();
});
