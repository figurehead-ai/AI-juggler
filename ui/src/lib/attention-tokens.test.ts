/**
 * 0.5 (D-117) — the calm-attention tokens, measured from styles.css in
 * both themes (the source; ui/tests/visual/contrast.spec.ts measures the
 * running app's computed values too). Every meaning-bearing text token
 * clears WCAG AA (4.5:1) on all five surfaces; the count badges' ink
 * clears it on the badge; danger clears it on its own L3 tint; and in
 * dark the alarm colours keep their APCA floors (warning Lc 60, danger
 * Lc 52), so the alarm is never the quietest status colour again.
 */
import { describe, expect, it } from "vitest";
import styles from "../styles.css?raw";
import { contrast, hexLuminance } from "./oklch";

const SURFACES = ["canvas", "canvas-hi", "surface", "surface-2", "surface-3"] as const;
const TEXT = ["ink", "ink-2", "section", "muted", "faint", "warning", "danger", "success", "live"] as const;

/** A theme's own block: the first `@theme {`, or `[data-theme="light"] {`. */
function block(theme: "dark" | "light"): string {
  const start = theme === "dark" ? styles.indexOf("@theme {") : styles.indexOf('[data-theme="light"] {');
  return styles.slice(start, styles.indexOf("\n}", start));
}

function token(theme: "dark" | "light", name: string): string {
  const own = block(theme).match(new RegExp(`--color-${name}:\\s*(#[0-9a-fA-F]{6,8})\\b`))?.[1];
  // The light theme keeps a dark value it doesn't redefine.
  const hex = own ?? block("dark").match(new RegExp(`--color-${name}:\\s*(#[0-9a-fA-F]{6,8})\\b`))?.[1];
  if (!hex) throw new Error(`--color-${name} is not in styles.css (${theme})`);
  return hex;
}

const rgb = (hex: string) => [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16));
const toHex = (channels: number[]) => `#${channels.map((c) => Math.round(c).toString(16).padStart(2, "0")).join("")}`;

/** A translucent tint (#rrggbbaa) laid over a surface. */
function over(tint: string, surface: string): string {
  const alpha = parseInt(tint.slice(7, 9), 16) / 255;
  const [a, b] = [rgb(tint), rgb(surface)];
  return toHex(a.map((c, i) => c * alpha + b[i]! * (1 - alpha)));
}

const ratio = (a: string, b: string) => contrast(hexLuminance(a), hexLuminance(b));

/** APCA 0.0.98G (W3C Silver draft), |Lc| of text on a background. */
function apca(text: string, background: string): number {
  const y = (hex: string) => {
    const [r, g, b] = rgb(hex).map((c) => (c / 255) ** 2.4);
    const lum = 0.2126729 * r! + 0.7151522 * g! + 0.072175 * b!;
    return lum > 0.022 ? lum : lum + (0.022 - lum) ** 1.414;
  };
  const [txt, bg] = [y(text), y(background)];
  const sapc = bg > txt ? (bg ** 0.56 - txt ** 0.57) * 1.14 : (bg ** 0.65 - txt ** 0.62) * 1.14;
  const lc = Math.abs(sapc) < 0.1 ? 0 : sapc > 0 ? sapc - 0.027 : sapc + 0.027;
  return Math.abs(lc * 100);
}

describe.each(["dark", "light"] as const)("the calm-attention tokens (%s)", (theme) => {
  it("every text token is 4.5:1 or more on every surface", () => {
    for (const name of TEXT) {
      for (const surface of SURFACES) {
        expect(ratio(token(theme, name), token(theme, surface)), `${name} on ${surface}`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it("a section label sits between ink-2 and muted", () => {
    for (const surface of SURFACES) {
      const bg = token(theme, surface);
      expect(ratio(token(theme, "section"), bg), `section vs ink-2 on ${surface}`).toBeLessThan(ratio(token(theme, "ink-2"), bg));
      expect(ratio(token(theme, "section"), bg), `section vs muted on ${surface}`).toBeGreaterThan(ratio(token(theme, "muted"), bg));
    }
  });

  it("each count badge's ink is AA on the badge", () => {
    expect(ratio(token(theme, "count-ink"), token(theme, "count"))).toBeGreaterThanOrEqual(4.5);
    expect(ratio(token(theme, "count-alarm-ink"), token(theme, "count-alarm"))).toBeGreaterThanOrEqual(4.5);
  });

  it("danger and ink stay AA on the L3 tint over every surface", () => {
    const tint = token(theme, "danger-soft");
    expect(tint).toHaveLength(9);
    for (const surface of SURFACES) {
      const tinted = over(tint, token(theme, surface));
      expect(ratio(token(theme, "danger"), tinted), `danger on the tint over ${surface}`).toBeGreaterThanOrEqual(4.5);
      expect(ratio(token(theme, "ink"), tinted), `ink on the tint over ${surface}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("a filled control is a visible step off the page, never a border's job", () => {
    expect(token(theme, "fill")).not.toBe(token(theme, "canvas"));
    expect(ratio(token(theme, "fill"), token(theme, "canvas"))).toBeGreaterThan(1.05);
  });
});

describe("the dark alarm ladder (APCA)", () => {
  it("warning reads Lc 60 or more and danger Lc 52 or more on every surface", () => {
    for (const surface of SURFACES) {
      expect(apca(token("dark", "warning"), token("dark", surface)), `warning on ${surface}`).toBeGreaterThanOrEqual(60);
      expect(apca(token("dark", "danger"), token("dark", surface)), `danger on ${surface}`).toBeGreaterThanOrEqual(52);
    }
  });

  it("the calm L1 colours are no louder than the alarm (chroma ladder)", () => {
    // Danger was the quietest status colour in dark (Lc 45–49 against
    // live's 66–69); now live and success sit below warning.
    for (const surface of SURFACES) {
      const bg = token("dark", surface);
      expect(apca(token("dark", "live"), bg)).toBeLessThan(apca(token("dark", "warning"), bg));
      expect(apca(token("dark", "success"), bg)).toBeLessThan(apca(token("dark", "warning"), bg));
    }
  });
});

describe("the level aliases and App tone", () => {
  it("name the status tokens, once", () => {
    expect(styles).toMatch(/--level-3:\s*var\(--color-danger\)/);
    expect(styles).toMatch(/--level-2:\s*var\(--color-warning\)/);
    expect(styles).toMatch(/--level-1-done:\s*var\(--color-success\)/);
    expect(styles).toMatch(/--level-1-live:\s*var\(--color-live\)/);
    expect(styles).toMatch(/--tone-saturate:\s*0\.25/);
  });

  it("honour Increase Contrast", () => {
    expect(styles).toMatch(/@media \(prefers-contrast: more\)/);
  });

  it("no animation loops for ever (0.4.5's rule: a live dot breathes three times)", () => {
    expect(styles).not.toMatch(/animation:[^;]*\binfinite\b/);
    expect(styles).toMatch(/\.dot-live \{\s*animation: breathe 2\.4s var\(--ease-out-soft\) 3;/);
  });
});
