import { describe, expect, it } from "vitest";
import { wholeRowsHeight } from "./whole-rows";

// Rows 36 px, headings 28 px, from 6 px of padding.
const parts = [
  { bottom: 34, head: true },
  { bottom: 70, head: false },
  { bottom: 106, head: false },
  { bottom: 134, head: true },
  { bottom: 170, head: false },
  { bottom: 206, head: false },
];

describe("0.5.1 F44: ⌘K's list ends on a whole row", () => {
  it("stops at the last row that shows whole", () => {
    expect(wholeRowsHeight(parts, 200)).toBe(170);
    expect(wholeRowsHeight(parts, 206)).toBe(206);
  });
  it("never ends on a group's heading alone (Suggested cut in half at the default height)", () => {
    expect(wholeRowsHeight(parts, 150)).toBe(106);
    expect(wholeRowsHeight(parts, 134)).toBe(106);
  });
  it("leaves the cap when not even one row fits", () => {
    expect(wholeRowsHeight(parts, 40)).toBeNull();
    expect(wholeRowsHeight([], 100)).toBeNull();
  });
});
