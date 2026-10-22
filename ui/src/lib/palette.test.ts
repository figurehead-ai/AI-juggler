/**
 * AJ-E1532 (brought forward with AJ-E1529): accounts are told apart at a
 * glance — up to twelve, each its own colour; the same account, the same
 * colour, however the list is ordered; neighbouring hues well apart.
 */
import { describe, expect, it } from "vitest";
import { assignPalette, paletteHue, PALETTE_HUES } from "./palette";
import { initialsOf } from "../components/indicator";

describe("the account palette", () => {
  it("gives each of up to twelve accounts its own colour, whatever the order", () => {
    const ids = Array.from({ length: 12 }, (_, n) => `account-${n}`);
    assignPalette(ids);
    const hues = ids.map(paletteHue);
    expect(new Set(hues).size).toBe(12);
    assignPalette([...ids].reverse());
    expect(ids.map(paletteHue)).toEqual(hues);
  });

  it("an account the list doesn't know yet still gets a stable colour", () => {
    assignPalette([]);
    expect(paletteHue("someone-new")).toBe(paletteHue("someone-new"));
    expect(PALETTE_HUES).toContain(paletteHue("someone-new"));
  });

  it("the first accounts get hues far apart, and never warning amber or error red", () => {
    const apart = (a: number, b: number) => Math.min(Math.abs(a - b), 360 - Math.abs(a - b));
    const first = PALETTE_HUES.slice(0, 6);
    const gaps = first.flatMap((a, i) => first.slice(i + 1).map((b) => apart(a, b)));
    expect(Math.min(...gaps)).toBeGreaterThanOrEqual(40);
    expect(PALETTE_HUES.slice(0, 10)).not.toContain(0);
    expect(PALETTE_HUES.slice(0, 10)).not.toContain(45);
  });

  it("neighbouring hues are at least 20° apart, all the way round", () => {
    const sorted = [...PALETTE_HUES].sort((a, b) => a - b);
    const gaps = sorted.map((hue, i) => ((sorted[(i + 1) % sorted.length]! - hue + 360) % 360));
    expect(Math.min(...gaps)).toBeGreaterThanOrEqual(20);
  });
});

describe("the account disc's initials", () => {
  it("part words at spaces, dashes, underscores and dots, first and last", () => {
    expect(["claude-work", "claude-personal", "claude-client-acme", "claude-client-globex", "claude-max-2"].map(initialsOf)).toEqual(["CW", "CP", "CA", "CG", "C2"]);
    expect(initialsOf("Work Max")).toBe("WM");
    expect(initialsOf("codex")).toBe("CO");
    expect(initialsOf("  ")).toBe("?");
  });
});
