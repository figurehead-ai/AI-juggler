/**
 * AJ-E1532 — projects and accounts are told apart at a glance: twelve
 * swatches, AA letters on each, all in sRGB (that each stands out from
 * both themes' surfaces: tests/bundle/palette-surfaces.test.ts), neighbours well apart (ΔE in OKLab); the same
 * account keeps its colour however the list is ordered; initials tell
 * accounts with a shared first word apart.
 */
import { describe, expect, it } from "vitest";
import { accountColor, accountInitials, assignPalette, distinctInitials, initialsOf, paletteIndex, PALETTE, PALETTE_HUES, swatchColor } from "./palette";
import { contrast, deltaE, inGamut, luminance } from "./oklch";

const WHITE = 1;

describe("the palette", () => {
  it("has twelve swatches, every one in sRGB", () => {
    expect(PALETTE).toHaveLength(12);
    for (const swatch of PALETTE) expect(inGamut(swatch), `hue ${swatch.h}`).toBe(true);
  });

  it("white letters are AA (4.5:1) on every swatch", () => {
    for (const swatch of PALETTE) {
      expect(contrast(luminance(swatch), WHITE), `hue ${swatch.h}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("neighbouring hues are well apart (ΔE ≥ 0.045 in OKLab), all the way round", () => {
    const sorted = [...PALETTE].sort((a, b) => a.h - b.h);
    const gaps = sorted.map((swatch, i) => deltaE(swatch, sorted[(i + 1) % sorted.length]!));
    expect(Math.min(...gaps)).toBeGreaterThanOrEqual(0.045);
  });

  it("the first swatches given out are far apart, and never warning amber or error red", () => {
    const apart = (a: number, b: number) => Math.min(Math.abs(a - b), 360 - Math.abs(a - b));
    const first = PALETTE_HUES.slice(0, 6);
    const gaps = first.flatMap((a, i) => first.slice(i + 1).map((b) => apart(a, b)));
    expect(Math.min(...gaps)).toBeGreaterThanOrEqual(30);
    for (const warm of [25, 55, 85]) expect(PALETTE_HUES.slice(0, 9)).not.toContain(warm);
  });

  it("a swatch is CSS the webview draws as it is", () => {
    expect(swatchColor(0)).toMatch(/^oklch\(0\.\d{3} 0\.\d{3} 265\)$/);
    expect(swatchColor(12)).toBe(swatchColor(0));
  });
});

describe("the accounts' colours", () => {
  const accounts = (ids: string[]) => ids.map((id) => ({ id, label: id }));

  it("gives each of up to twelve accounts its own, whatever the order", () => {
    const ids = Array.from({ length: 12 }, (_, n) => `account-${n}`);
    assignPalette(accounts(ids));
    const colours = ids.map(accountColor);
    expect(new Set(colours).size).toBe(12);
    assignPalette(accounts([...ids].reverse()));
    expect(ids.map(accountColor)).toEqual(colours);
  });

  it("an account the list doesn't know yet still gets a stable one", () => {
    assignPalette([]);
    expect(paletteIndex("someone-new")).toBe(paletteIndex("someone-new"));
    expect(paletteIndex("someone-new")).toBeLessThan(12);
  });
});

describe("the accounts' initials", () => {
  it("part words at spaces, dashes, underscores and dots: first and last, or a word's first two letters", () => {
    expect(initialsOf("Work Max")).toBe("WM");
    expect(initialsOf("codex")).toBe("Co");
    expect(initialsOf("  ")).toBe("?");
  });

  it("labels that share their first words are known by the words after them (QA, the real setup)", () => {
    expect(distinctInitials(["claude-work", "claude-personal", "claude-client-acme", "claude-client-globex", "claude-max-2"])).toEqual([
      "Wo",
      "Pe",
      "Ac",
      "Gl",
      "M2",
    ]);
    expect(distinctInitials(["Work Max", "Work Pro", "Personal"])).toEqual(["Ma", "Pr", "Pe"]);
    expect(distinctInitials(["claude", "claude-work"])).toEqual(["Cl", "Wo"]);
  });

  it("where two would still meet, the later takes its next letters (QA, the real setup: claude-work and codex-work)", () => {
    expect(distinctInitials(["claude", "claude-work", "codex", "codex-work"])).toEqual(["Cl", "Wo", "Co", "CW"]);
    expect(distinctInitials(["a-work", "b-work"])).toEqual(["AW", "BW"]);
    expect(distinctInitials(["x-same", "x-same"])).toEqual(["Sa", "XS"]);
  });

  it("are given with the colours, by id", () => {
    assignPalette([
      { id: "1", label: "claude-work" },
      { id: "2", label: "claude-personal" },
    ]);
    expect(accountInitials("1", "claude-work")).toBe("Wo");
    expect(accountInitials("unknown", "Work Max")).toBe("WM");
  });
});
