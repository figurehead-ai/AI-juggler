/**
 * The P9 walk: a pool's canvas fitted its members before the inspector
 * and a question took their space, so a member sat half under the
 * inspector. A fitted view refits as the room changes; one the person
 * moved stays where they put it.
 */
import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { forgetCanvasViews, useCanvasView } from "./canvas-view";

beforeEach(forgetCanvasViews);

const fitFor = (width: number) => ({ x: width / 2, y: 0, scale: 1 });

describe("a pool canvas's view", () => {
  it("refits when its room changes, until the person moves it", () => {
    const { result, rerender } = renderHook(({ width }) => useCanvasView("pool-a", () => fitFor(width), true, [width]), { initialProps: { width: 900 } });
    expect(result.current.view).toEqual(fitFor(900));

    // The inspector comes in: less room, fitted again.
    rerender({ width: 600 });
    expect(result.current.view).toEqual(fitFor(600));

    // The person pans: their view stays when the room changes again.
    act(() => result.current.setView({ x: 10, y: 20, scale: 1 }));
    rerender({ width: 700 });
    expect(result.current.view).toEqual({ x: 10, y: 20, scale: 1 });

    // Fit makes it follow the room again.
    act(() => result.current.fit());
    rerender({ width: 500 });
    expect(result.current.view).toEqual(fitFor(500));
  });

  it("waits until the members and the room are known", () => {
    const { result } = renderHook(() => useCanvasView("pool-b", () => fitFor(900), false, [900]));
    expect(result.current.view).toBeNull();
  });
});

describe("a pool canvas's view, as its members' places come in", () => {
  it("refits when the kept places arrive, and holds once the person moves a member", () => {
    let center = 0;
    const { result, rerender } = renderHook(({ places }) => useCanvasView("pool-c", () => ({ x: center, y: 0, scale: 1 }), true, [places]), { initialProps: { places: "m:0,0" } });
    expect(result.current.view?.x).toBe(0);
    center = 140;
    rerender({ places: "m:140,90" });
    expect(result.current.view?.x).toBe(140);
    act(() => result.current.hold());
    center = 300;
    rerender({ places: "m:300,90" });
    expect(result.current.view?.x).toBe(140);
  });
});
