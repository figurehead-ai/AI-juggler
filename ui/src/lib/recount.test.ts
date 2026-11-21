/**
 * SK48-14 (the 0.4.8 soak, PD4 step 2): after a stop, the list said
 * "Nothing needs you" while the tile and the top bar still counted "1 to
 * clean up": they waited for a change event the window never heard. A
 * stop's recount makes every count read again, now and a tick later.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { askRecount, RECOUNT_AGAIN_MS } from "./recount";
import { onProcessCounts, onProcessesChanged, processesCounts } from "./ipc/processes";

describe("a stop's recount", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("re-reads the tile's and the top bar's counts at once and a tick later", async () => {
    // The made-up Mac's counts, loaded before the clock is held.
    await processesCounts();
    vi.useFakeTimers();
    const tile = vi.fn();
    const bar = vi.fn();
    const stopTile = onProcessesChanged(tile);
    const stopBar = onProcessCounts(bar);
    askRecount();
    await vi.advanceTimersByTimeAsync(10);
    expect(tile).toHaveBeenCalledTimes(1);
    expect(bar).toHaveBeenCalledTimes(1);
    expect(bar.mock.calls[0]?.[0]).toHaveProperty("to_clean");
    await vi.advanceTimersByTimeAsync(RECOUNT_AGAIN_MS);
    expect(tile).toHaveBeenCalledTimes(2);
    expect(bar).toHaveBeenCalledTimes(2);
    stopTile();
    stopBar();
    askRecount();
    await vi.advanceTimersByTimeAsync(RECOUNT_AGAIN_MS + 10);
    expect(tile).toHaveBeenCalledTimes(2);
    expect(bar).toHaveBeenCalledTimes(2);
  });
});
