import { afterEach, expect, it } from "vitest";
import { trackInputModality } from "./input-modality";

let stop: (() => void) | null = null;
afterEach(() => {
  stop?.();
  stop = null;
  delete document.documentElement.dataset.input;
});

// AJ-E3603: a script's focus target shows its ring only after a key.
it("says keyboard after a key, pointer after a press, nothing before either", () => {
  stop = trackInputModality();
  expect(document.documentElement.dataset.input).toBeUndefined();
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab" }));
  expect(document.documentElement.dataset.input).toBe("keyboard");
  window.dispatchEvent(new Event("pointerdown"));
  expect(document.documentElement.dataset.input).toBe("pointer");
});

it("a modifier alone is no key (⌘-click stays a click)", () => {
  stop = trackInputModality();
  window.dispatchEvent(new Event("pointerdown"));
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Meta" }));
  expect(document.documentElement.dataset.input).toBe("pointer");
});

it("stops when asked", () => {
  trackInputModality()();
  window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
  expect(document.documentElement.dataset.input).toBeUndefined();
});
