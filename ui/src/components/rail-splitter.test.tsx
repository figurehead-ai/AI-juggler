/**
 * AJ-E2909 vitest — the list's edge from the keyboard: focusable, named,
 * its width heard, ← → by 8, ⇧ by 32, Home/End to the ends, kept as a
 * drag's width is.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RailSplitter, STORE_WIDTH, WIDTH_MAX, WIDTH_MIN, readWidth, widthForKey } from "./RailSplitter";

function Host() {
  const [width, setWidth] = useState(264);
  return <RailSplitter width={width} onWidth={setWidth} controls="rail-list" />;
}

beforeEach(() => {
  localStorage.removeItem(STORE_WIDTH);
  vi.useFakeTimers();
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("AJ-E2909 — the rail's splitter", () => {
  it("is a focusable separator that says its width", () => {
    const { getByRole } = render(<Host />);
    const handle = getByRole("separator", { name: "Resize the list" });
    expect(handle.tabIndex).toBe(0);
    expect(handle.getAttribute("aria-valuenow")).toBe("264");
    expect(handle.getAttribute("aria-valuemin")).toBe(String(WIDTH_MIN));
    expect(handle.getAttribute("aria-valuemax")).toBe(String(WIDTH_MAX));
    expect(handle.getAttribute("aria-valuetext")).toBe("264 pixels wide");
    expect(handle.getAttribute("aria-controls")).toBe("rail-list");
  });

  it("moves by 8, by 32 with shift, to the ends with Home and End, and keeps it", () => {
    const { getByRole } = render(<Host />);
    const handle = getByRole("separator", { name: "Resize the list" });
    fireEvent.keyDown(handle, { key: "ArrowRight" });
    expect(handle.getAttribute("aria-valuenow")).toBe("272");
    fireEvent.keyDown(handle, { key: "ArrowLeft", shiftKey: true });
    expect(handle.getAttribute("aria-valuenow")).toBe("240");
    fireEvent.keyDown(handle, { key: "End" });
    expect(handle.getAttribute("aria-valuenow")).toBe(String(WIDTH_MAX));
    act(() => void vi.advanceTimersByTime(250));
    expect(readWidth()).toBe(WIDTH_MAX);
    fireEvent.keyDown(handle, { key: "Home" });
    act(() => void vi.advanceTimersByTime(250));
    expect(readWidth()).toBe(WIDTH_MIN);
  });

  it("never passes its bounds", () => {
    expect(widthForKey(WIDTH_MIN, "ArrowLeft", true)).toBe(WIDTH_MIN);
    expect(widthForKey(WIDTH_MAX, "ArrowRight", false)).toBe(WIDTH_MAX);
    expect(widthForKey(300, "a", false)).toBeNull();
  });

  it("keeps a waiting change when the window goes (a quit inside the 200 ms)", () => {
    const { getByRole } = render(<Host />);
    fireEvent.keyDown(getByRole("separator"), { key: "End" });
    expect(localStorage.getItem(STORE_WIDTH)).toBeNull();
    act(() => void window.dispatchEvent(new Event("pagehide")));
    expect(localStorage.getItem(STORE_WIDTH)).toBe(String(WIDTH_MAX));
  });
});
