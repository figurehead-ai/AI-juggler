/**
 * The go sitting's rerun (2026-11-07): "Opened the session in Claude app ·
 * aij-test" wore a warning, because only "Focused window" earned a tick.
 * The step reached decides: the session itself is a success, nothing an
 * error, part of the way a warning.
 */
import { describe, expect, it } from "vitest";
import { FOCUSED, goToTone } from "./go-to-words";

describe("a Go to it's tone", () => {
  it("is a success when it reached the session or the window came forward", () => {
    expect(goToTone("Opened the session in Claude app · aij-test", "thread")).toBe("success");
    expect(goToTone("Went to its tab in Terminal", "tab")).toBe("success");
    expect(goToTone("Opened its folder's window", "folder")).toBe("success");
    expect(goToTone(FOCUSED, "app")).toBe("success");
  });

  it("is a warning part of the way, and an error when nothing happened", () => {
    expect(goToTone("Brought Claude app forward; pick the session in its sidebar", "app")).toBe("warning");
    expect(goToTone("Window not found", "thread")).toBe("warning");
    expect(goToTone("Couldn't open it in Claude app", "none")).toBe("error");
  });
});
