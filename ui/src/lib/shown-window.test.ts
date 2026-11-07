import { afterEach, describe, expect, it, vi } from "vitest";
import { everyWhileShown, nextDelay } from "./shown-window";
import { keepIfSame, same } from "./same";

const setHidden = (hidden: boolean) => {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => (hidden ? "hidden" : "visible") });
  document.dispatchEvent(new Event("visibilitychange"));
};

afterEach(() => {
  vi.useRealTimers();
  setHidden(false);
});

describe("the window's own polling", () => {
  it("runs on its cadence when shown, slower unfocused, never hidden", () => {
    expect(nextDelay(2000, true, true, 4)).toBe(2000);
    expect(nextDelay(2000, true, false, 4)).toBe(8000);
    expect(nextDelay(2000, false, true, 4)).toBeNull();
  });

  it("pauses while hidden and runs once at once when shown again", () => {
    vi.useFakeTimers();
    vi.spyOn(document, "hasFocus").mockReturnValue(true);
    const run = vi.fn();
    const stop = everyWhileShown(run, 1000);
    vi.advanceTimersByTime(3000);
    expect(run).toHaveBeenCalledTimes(3);
    setHidden(true);
    vi.advanceTimersByTime(10_000);
    expect(run).toHaveBeenCalledTimes(3);
    setHidden(false);
    expect(run).toHaveBeenCalledTimes(4);
    vi.advanceTimersByTime(1000);
    expect(run).toHaveBeenCalledTimes(5);
    stop();
    vi.advanceTimersByTime(5000);
    expect(run).toHaveBeenCalledTimes(5);
  });
});

describe("an equal answer changes nothing", () => {
  it("compares the backend's answers by value", () => {
    expect(same({ a: [1, { b: "x" }], c: null }, { c: null, a: [1, { b: "x" }] })).toBe(true);
    expect(same([1, 2], [1, 2, 3])).toBe(false);
    expect(same({ a: 1 }, { a: 1, b: undefined })).toBe(false);
    const was = [{ id: "s1", state: "working" }];
    expect(keepIfSame(was, [{ id: "s1", state: "working" }])).toBe(was);
    const next = [{ id: "s1", state: "asks" }];
    expect(keepIfSame(was, next)).toBe(next);
  });
});
