/**
 * 0.5 (D-117) — the calm-attention tokens, measured from styles.css in
 * both themes (the source; ui/tests/visual/contrast.spec.ts measures the
 * running app's computed values too). Every meaning-bearing text token
 * clears WCAG AA (4.5:1) on all five surfaces; the count badges' ink
 * clears it on the badge; danger clears it on its own L3 tint.
 *
 * 0.4.6 (AJ-E3011) takes 0.5's new tokens only, for the pool screens; the
 * dark status ladder's retune and its APCA floors are 0.5's (fa5b883d8).
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

  // D-117 (selection): what is selected stands ≥ 3:1 off the unselected
  // items around it (WCAG 1.4.11) on every surface and on a filled control,
  // and its words are AA on its fill; never the accent (focus's colour).
  it("a selected item's edge is 3:1 off everything unselected around it, its words AA", () => {
    for (const around of [...SURFACES, "fill"] as const) {
      expect(ratio(token(theme, "selected-ring"), token(theme, around)), `selected-ring vs ${around}`).toBeGreaterThanOrEqual(3);
    }
    expect(ratio(token(theme, "selected-ink"), token(theme, "selected"))).toBeGreaterThanOrEqual(4.5);
    expect(token(theme, "selected-ring")).not.toBe(token(theme, "accent"));
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
