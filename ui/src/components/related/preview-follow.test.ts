import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const refreshPaged = vi.fn();
vi.mock("../../lib/paged", () => ({ refreshPaged: (key: string) => refreshPaged(key) }));

const { usePreviewFollow } = await import("./preview-follow");

describe("usePreviewFollow (0.5.1 F49)", () => {
  it("reads the list again when the preview's rows change, never on its first render or when off", () => {
    const view = renderHook(({ on, keys }) => usePreviewFollow(on, "list:a", keys), { initialProps: { on: true, keys: "a\nb" } });
    expect(refreshPaged).not.toHaveBeenCalled();
    view.rerender({ on: true, keys: "a\nb" });
    expect(refreshPaged).not.toHaveBeenCalled();
    view.rerender({ on: true, keys: "new1\na\nb" });
    expect(refreshPaged).toHaveBeenCalledTimes(1);
    expect(refreshPaged).toHaveBeenCalledWith("list:a");
    view.rerender({ on: false, keys: "x" });
    expect(refreshPaged).toHaveBeenCalledTimes(1);
  });
});
