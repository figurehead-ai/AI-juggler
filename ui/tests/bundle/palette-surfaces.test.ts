// @vitest-environment node
/**
 * AJ-E1532 — every swatch of the palette stands out (3:1, WCAG 1.4.11)
 * from every surface a mark or an avatar sits on, in both themes. The
 * surfaces are read from styles.css itself, so a theme change that
 * would hide a project's colour fails here.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { PALETTE } from "../../src/lib/palette";
import { contrast, hexLuminance, luminance } from "../../src/lib/oklch";

const css = readFileSync(fileURLToPath(new URL("../../src/styles.css", import.meta.url)), "utf8");

function surfaces(): { theme: string; name: string; hex: string }[] {
  const light = css.indexOf('[data-theme="light"]');
  return [...css.matchAll(/--color-(canvas|surface(?:-2|-3)?):\s*(#[0-9a-fA-F]{6})/g)].map((match) => ({
    theme: match.index! > light ? "light" : "dark",
    name: match[1]!,
    hex: match[2]!,
  }));
}

describe("the palette on the app's surfaces", () => {
  it("reads four surfaces per theme", () => {
    const all = surfaces();
    expect(all.filter((s) => s.theme === "dark")).toHaveLength(4);
    expect(all.filter((s) => s.theme === "light")).toHaveLength(4);
  });

  it("every swatch stands out (3:1) from every surface in both themes", () => {
    for (const surface of surfaces()) {
      for (const swatch of PALETTE) {
        expect(contrast(luminance(swatch), hexLuminance(surface.hex)), `hue ${swatch.h} on ${surface.theme} ${surface.name}`).toBeGreaterThanOrEqual(3);
      }
    }
  });
});
