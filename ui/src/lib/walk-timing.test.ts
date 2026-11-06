/**
 * AJ-E3301 vitest — a walk step's click to feedback and click to done.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { startWalkTiming, type WalkStep } from "./walk-timing";
import { timed } from "./ipc/timing";

afterEach(() => vi.useRealTimers());

describe("AJ-E3301 walk timing", () => {
  it("times a click to its first change, and to done once nothing is in flight and the screen is quiet", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "performance"] });
    const stop = startWalkTiming();
    const steps = (window as unknown as { __AIJ_WALK__: WalkStep[] }).__AIJ_WALK__;
    window.dispatchEvent(new Event("pointerdown"));
    let release!: () => void;
    const call = timed("slow_read", () => new Promise<void>((resolve) => (release = resolve)));
    vi.advanceTimersByTime(40);
    document.body.appendChild(document.createElement("div"));
    await Promise.resolve();
    expect(steps[0]!.feedbackMs).toBe(40);
    vi.advanceTimersByTime(1000);
    expect(steps[0]!.doneMs).toBeNull(); // a call is still in flight
    release();
    await call;
    vi.advanceTimersByTime(400);
    expect(steps[0]!.doneMs).toBe(40);
    stop();
  });
});
