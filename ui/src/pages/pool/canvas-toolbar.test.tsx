/**
 * The rc26 P3 walk: "3 more in the List view" sat at the canvas's bottom
 * right, where a toast ("docs is now in API change") covered it. Toasts
 * land at the window's bottom right, so the chip ends the view tools'
 * row at the bottom left instead.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { CanvasToolbar } from "./CanvasToolbar";

afterEach(() => cleanup());

const noop = () => undefined;

describe("what the canvas doesn't draw", () => {
  it("is said at the end of the view tools, bottom left, clear of the toasts", () => {
    const { getByTestId, container } = render(<CanvasToolbar scale={1} more={3} onZoom={noop} onFit={noop} onTidy={noop} />);
    const chip = getByTestId("canvas-more");
    expect(chip.textContent).toBe("3 more in the List view");
    const toolbar = container.querySelector("[data-canvas-toolbar]")!;
    expect(toolbar.contains(chip)).toBe(true);
    expect(toolbar.className).toContain("left-3");
    expect(toolbar.className).not.toMatch(/\bright-/);
  });

  it("says nothing when the canvas draws everyone", () => {
    const { queryByTestId } = render(<CanvasToolbar scale={1} more={0} onZoom={noop} onFit={noop} onTidy={noop} />);
    expect(queryByTestId("canvas-more")).toBeNull();
  });
});
