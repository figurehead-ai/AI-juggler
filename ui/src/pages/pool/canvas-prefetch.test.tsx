/**
 * X2 (D-62.20): a pool of 50 opens at once. The rc26 X2 walk measured
 * 120–140 ms from the click to the canvas: its members, then the canvas
 * read for them, both read only once the page opened. A pool read ahead
 * (`prefetchPool`: its card hovered, focused, or among the first shown)
 * paints both in the page's first render, and reads them again behind.
 */
import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { MOCK_SESSIONS } from "../../preview/fixtures";
import { refreshPools } from "../../lib/pools";
import { resetCanvasTests, seams, seedTeam } from "./canvas.test-kit";
import { prefetchPool, useCanvas } from "./useCanvas";

beforeEach(() => {
  resetCanvasTests();
  seedTeam();
});

describe("a pool read ahead", () => {
  it("paints its members and its canvas in the first render, with no read waited for", async () => {
    await act(async () => {
      await refreshPools();
      await prefetchPool("pool-team");
    });
    const reads = (seams().__AIJ_POOL_CANVAS_CALLS__ ?? []).length;
    const { result } = renderHook(() => useCanvas("pool-team", MOCK_SESSIONS));
    expect(result.current.members.map((member) => member.role)).toEqual(["lead", "member"]);
    expect(result.current.canvas?.pool_id).toBe("pool-team");
    expect(result.current.list.loading).toBe(false);
    expect((seams().__AIJ_POOL_CANVAS_CALLS__ ?? []).length).toBe(reads);
  });

  it("is read once, however often its card is pointed at", async () => {
    await act(async () => {
      await refreshPools();
      await Promise.all([prefetchPool("pool-team"), prefetchPool("pool-team")]);
      await prefetchPool("pool-team");
    });
    const { result } = renderHook(() => useCanvas("pool-team", MOCK_SESSIONS));
    expect(result.current.canvas).not.toBeNull();
  });

  it("without it, the first render has no member to draw: the page reads them", async () => {
    await act(async () => {
      await refreshPools();
    });
    const { result } = renderHook(() => useCanvas("pool-team", MOCK_SESSIONS));
    expect(result.current.members).toEqual([]);
    expect(result.current.list.loading).toBe(true);
  });
});
