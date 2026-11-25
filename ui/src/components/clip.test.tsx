/**
 * AJ-E1546 R4: a clip is muted, plays only while it's on screen (paused
 * when scrolled off), and under Reduce Motion is its still poster, never
 * a video.
 */
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/** One IntersectionObserver per clip; `show(on)` says it scrolled on or off screen. */
const io = vi.hoisted(() => {
  const observers: MockObserver[] = [];
  class MockObserver implements IntersectionObserver {
    readonly root = null;
    readonly rootMargin = "0px";
    readonly scrollMargin = "0px";
    readonly thresholds = [0.35];
    target: Element | null = null;
    constructor(readonly callback: IntersectionObserverCallback) {
      observers.push(this);
    }
    observe(target: Element) {
      this.target = target;
    }
    unobserve() {}
    disconnect() {}
    takeRecords(): IntersectionObserverEntry[] {
      return [];
    }
  }
  Object.defineProperty(globalThis, "IntersectionObserver", { value: MockObserver, configurable: true, writable: true });
  return {
    observers,
    show(on: boolean) {
      for (const observer of observers) {
        const target = observer.target;
        if (!target) continue;
        const box = target.getBoundingClientRect();
        observer.callback([{ boundingClientRect: box, intersectionRatio: on ? 1 : 0, intersectionRect: box, isIntersecting: on, rootBounds: null, target, time: 0 }], observer);
      }
    },
  };
});

import { Clip } from "./Clip";

let reduce = false;
const play = vi.fn(() => Promise.resolve());
const pause = vi.fn();

beforeEach(() => {
  reduce = false;
  io.observers.length = 0;
  play.mockClear();
  pause.mockClear();
  vi.spyOn(HTMLMediaElement.prototype, "play").mockImplementation(play);
  vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(pause);
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: reduce && query.includes("prefers-reduced-motion: reduce"),
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("a clip (AJ-E1546 R4)", () => {
  it("is muted and inline, and loads nothing until it's on screen", () => {
    const view = render(<Clip id="perm-focus" label="Jump to a session's own window" />);
    const video = view.container.querySelector("video")!;
    expect(video.muted).toBe(true);
    expect(video.hasAttribute("playsinline")).toBe(true);
    expect(video.getAttribute("src")).toBeNull();
    expect(play).not.toHaveBeenCalled();
  });

  it("plays while on screen and pauses when it scrolls off", () => {
    const view = render(<Clip id="perm-focus" label="Jump to a session's own window" />);
    const video = view.container.querySelector("video")!;
    act(() => io.show(true));
    expect(video.getAttribute("src")).toMatch(/clips\/perm-focus-(dark|light)\.webm$/);
    expect(play).toHaveBeenCalledTimes(1);
    act(() => io.show(false));
    expect(pause).toHaveBeenCalled();
    play.mockClear();
    act(() => io.show(true));
    expect(play).toHaveBeenCalledTimes(1);
  });

  it("is its still poster under Reduce Motion: no video, nothing plays", () => {
    reduce = true;
    const view = render(<Clip id="perm-focus" label="Jump to a session's own window" />);
    const frame = view.getByTestId("clip-perm-focus");
    expect(frame.getAttribute("data-still")).toBe("true");
    expect(view.container.querySelector("video")).toBeNull();
    expect(frame.querySelector("img")?.getAttribute("src")).toMatch(/clips\/perm-focus-(dark|light)\.png$/);
    act(() => io.show(true));
    expect(play).not.toHaveBeenCalled();
  });
});
