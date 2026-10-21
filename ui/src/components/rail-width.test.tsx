/**
 * The rail's starting width (0.0.11 walk, Q1): nothing stored is the
 * default, 264 px — `Number(null)` is 0, which clamped every install to
 * the 208 px minimum since the first release.
 */
import { afterEach, describe, expect, it } from "vitest";
import { readWidth } from "./WorkflowRail";

afterEach(() => localStorage.clear());

describe("readWidth", () => {
  it("nothing stored: the default, not the minimum", () => {
    expect(readWidth()).toBe(264);
  });
  it("an empty or unreadable value: the default", () => {
    localStorage.setItem("aij.rail.width", "");
    expect(readWidth()).toBe(264);
    localStorage.setItem("aij.rail.width", "wide");
    expect(readWidth()).toBe(264);
  });
  it("a stored width is kept, inside the limits", () => {
    localStorage.setItem("aij.rail.width", "300");
    expect(readWidth()).toBe(300);
    localStorage.setItem("aij.rail.width", "100");
    expect(readWidth()).toBe(208);
    localStorage.setItem("aij.rail.width", "999");
    expect(readWidth()).toBe(360);
  });
});
