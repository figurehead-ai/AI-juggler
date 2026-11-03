/**
 * The maintainer's report (2026-11-03): what isn't running stays out of the
 * way. The rail's Inactive apps starts folded; each section stays as the
 * user left it.
 */
import { afterEach, describe, expect, it } from "vitest";
import { readClosedSections, writeClosedSections } from "./rail-groups";

afterEach(() => localStorage.clear());

describe("the rail's sections, folded", () => {
  it("Inactive apps starts folded; Needs you and Open now start open", () => {
    expect([...readClosedSections()]).toEqual(["idle"]);
  });

  it("each stays as it was left, Inactive apps unfolded included", () => {
    writeClosedSections(new Set());
    expect([...readClosedSections()]).toEqual([]);
    writeClosedSections(new Set(["open", "idle"]));
    expect([...readClosedSections()].sort()).toEqual(["idle", "open"]);
  });

  it("a stored value it can't read: the defaults", () => {
    localStorage.setItem("aij.rail.sections.closed", "{nope");
    expect([...readClosedSections()]).toEqual(["idle"]);
    localStorage.setItem("aij.rail.sections.closed", JSON.stringify(["idle", "bogus", 3]));
    expect([...readClosedSections()]).toEqual(["idle"]);
  });
});
