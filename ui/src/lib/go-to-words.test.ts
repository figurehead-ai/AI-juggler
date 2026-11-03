import { describe, expect, it } from "vitest";
import { FOCUSED, partialGoTo } from "./go-to-words";

describe("the 0.4 review: a Go to it is a success only when the window came forward", () => {
  it("names the partial outcomes", () => {
    expect(partialGoTo(FOCUSED)).toBe(false);
    expect(partialGoTo("App opened — window not found")).toBe(true);
    expect(partialGoTo("App not running — Relaunch?")).toBe(true);
    expect(partialGoTo("macOS didn't bring the app forward")).toBe(true);
    expect(partialGoTo("Stopped")).toBe(false);
  });
});
