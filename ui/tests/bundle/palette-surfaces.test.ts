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

/** Each surface with its theme and scope: the app's own, or a
 * conversation's `.reading` one (0.3.4), whose block names its theme. */
function surfaces(): { theme: string; scope: string; name: string; hex: string }[] {
  const light = css.indexOf('[data-theme="light"] {');
  const reading = css.indexOf(".reading {");
  const readingLight = css.indexOf('[data-theme="light"] .reading {');
  return [...css.matchAll(/--color-(canvas|surface(?:-2|-3)?):\s*(#[0-9a-fA-F]{6})/g)].map((match) => {
    const at = match.index!;
    // styles.css's order: the dark theme, the light one, then `.reading`
    // dark and `.reading` light.
    const scope = reading >= 0 && at > reading ? "reading" : "app";
    const theme = scope === "reading" ? (at > readingLight ? "light" : "dark") : at > light ? "light" : "dark";
    return { theme, scope, name: match[1]!, hex: match[2]! };
  });
}

describe("the palette on the app's surfaces", () => {
  it("reads four surfaces per theme, and a conversation's three", () => {
    const all = surfaces();
    for (const theme of ["dark", "light"]) {
      expect(all.filter((s) => s.theme === theme && s.scope === "app")).toHaveLength(4);
      expect(all.filter((s) => s.theme === theme && s.scope === "reading").map((s) => s.name)).toEqual(["surface", "surface-2", "surface-3"]);
    }
  });

  it("every swatch stands out (3:1) from every surface in both themes", () => {
    for (const surface of surfaces()) {
      for (const swatch of PALETTE) {
        expect(contrast(luminance(swatch), hexLuminance(surface.hex)), `hue ${swatch.h} on ${surface.theme} ${surface.name}`).toBeGreaterThanOrEqual(3);
      }
    }
  });
});
