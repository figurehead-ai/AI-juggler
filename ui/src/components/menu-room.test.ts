/**
 * 0.5.2 (PR #257's menus gate): a menu too tall for either side of its
 * button, but not for the window, gets room by moving the button's
 * scrolling area, never by covering the button or scrolling inside.
 */
import { afterEach, describe, expect, it } from "vitest";
import { anchoredEdge, anchoredSpan } from "./RowMenu";
import { placed, withRoom } from "./menu-room";

afterEach(() => {
  document.body.innerHTML = "";
});

/** A button at `top` (28 px tall) in a scrolling area, in an 800 px window. */
function inScroller(top: number): { button: HTMLButtonElement; scroller: HTMLDivElement } {
  Object.defineProperty(window, "innerHeight", { value: 800, configurable: true });
  const scroller = document.createElement("div");
  scroller.style.overflowY = "auto";
  Object.defineProperty(scroller, "scrollHeight", { value: 3000 });
  Object.defineProperty(scroller, "clientHeight", { value: 800 });
  const button = document.createElement("button");
  scroller.appendChild(button);
  document.body.appendChild(scroller);
  button.getBoundingClientRect = () => {
    const y = top - scroller.scrollTop;
    return { top: y, bottom: y + 28, left: 0, right: 100, width: 100, height: 28, x: 0, y, toJSON: () => ({}) };
  };
  button.focus();
  return { button, scroller };
}

describe("a menu too tall for either side of its button", () => {
  it("moves the button up so the whole menu fits below it", () => {
    const { scroller } = inScroller(470);
    const at = { x: 0, y: 502, above: 466 };
    // 500 px: neither 290 below nor 458 above holds it.
    expect(anchoredSpan(at.y, at.above, 500, 800).max).not.toBeNull();
    const span = withRoom(at, 500, anchoredSpan);
    expect(span.max).toBeNull();
    expect(scroller.scrollTop).toBeGreaterThan(0);
    // Below the button, never over it.
    expect(span.start).toBeGreaterThanOrEqual(470 - scroller.scrollTop + 28);
    expect(span.start + 500).toBeLessThanOrEqual(800 - 8);
  });

  it("leaves a menu taller than the window to scroll inside, and one that fits alone", () => {
    const { scroller } = inScroller(470);
    expect(withRoom({ x: 0, y: 502, above: 466 }, 900, anchoredSpan).max).not.toBeNull();
    expect(scroller.scrollTop).toBe(0);
    expect(withRoom({ x: 0, y: 502, above: 466 }, 200, anchoredSpan)).toEqual({ start: 502, max: null });
  });

  it("with nothing that scrolls, opens beside its button, the whole menu in the window", () => {
    Object.defineProperty(window, "innerHeight", { value: 800, configurable: true });
    Object.defineProperty(window, "innerWidth", { value: 1280, configurable: true });
    const button = document.createElement("button");
    document.body.appendChild(button);
    button.getBoundingClientRect = () => ({ top: 395, bottom: 423, left: 227, right: 251, width: 24, height: 28, x: 227, y: 395, toJSON: () => ({}) });
    button.focus();
    const at = { x: 227, y: 427, above: 391, right: 251 };
    const place = placed(at, { width: 294, height: 495 }, anchoredSpan, anchoredEdge);
    expect(place.maxHeight).toBeUndefined();
    // Beside the button, never over it.
    expect(place.left).toBeGreaterThanOrEqual(251);
    expect(place.top).toBeGreaterThanOrEqual(8);
    expect(place.top + 495).toBeLessThanOrEqual(792);
  });
});
