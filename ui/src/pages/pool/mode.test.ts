/**
 * A pool's view: the List view under 1100 px (the canvas needs the room),
 * except a pool with no members yet, which opens on the canvas that teaches
 * setting it up (AJ-E3008, the P10 walk); the user's own choice wins.
 */
import { afterEach, describe, expect, it } from "vitest";
import { chosenMode, defaultMode, LIST_BELOW } from "./mode";

afterEach(() => localStorage.clear());

describe("a pool's view", () => {
  it("follows the width, but an empty pool opens on the canvas", () => {
    expect(defaultMode(LIST_BELOW - 1)).toBe("list");
    expect(defaultMode(LIST_BELOW)).toBe("canvas");
    expect(defaultMode(1024, true)).toBe("canvas");
  });

  it("keeps the user's own choice", () => {
    expect(chosenMode("p1")).toBeNull();
    localStorage.setItem("aij.pool.mode.p1", "list");
    expect(chosenMode("p1")).toBe("list");
  });
});
