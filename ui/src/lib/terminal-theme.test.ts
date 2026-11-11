/**
 * 0.4.1 vitest — the terminal's colours: all 16 ANSI colours and its own
 * defined, each readable on the background, bold, dim and italic drawn
 * as styles, and both terminals (live and replay) drawing with them.
 */
import { describe, expect, it } from "vitest";
import { TERMINAL_STYLE, TERMINAL_THEME } from "./terminal-theme";

const ANSI = [
  "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
  "brightBlack", "brightRed", "brightGreen", "brightYellow", "brightBlue", "brightMagenta", "brightCyan", "brightWhite",
] as const;

function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi! + 0.05) / (lo! + 0.05);
}

describe("the terminal's palette", () => {
  it("defines all 16 ANSI colours, the foreground and the background", () => {
    const theme: Record<string, string | undefined> = TERMINAL_THEME;
    for (const name of [...ANSI, "foreground", "background", "cursor"]) {
      expect(theme[name], name).toMatch(/^#[0-9a-f]{6}$/);
    }
    // Sixteen different colours: no bright one the same as its normal.
    expect(new Set(ANSI.map((name) => TERMINAL_THEME[name])).size).toBe(16);
  });

  it("reads on its background: every text colour 4.5:1 or more", () => {
    for (const name of [...ANSI.filter((n) => n !== "black"), "foreground"] as const) {
      expect(contrast(TERMINAL_THEME[name], TERMINAL_THEME.background), name).toBeGreaterThanOrEqual(4.5);
    }
    // Black is a fill: it stands off the background, and text on it reads.
    expect(contrast(TERMINAL_THEME.black, TERMINAL_THEME.background)).toBeGreaterThan(1.4);
    expect(contrast(TERMINAL_THEME.foreground, TERMINAL_THEME.black)).toBeGreaterThanOrEqual(4.5);
  });

  it("draws bold, dim and every tool colour as intended", () => {
    expect(TERMINAL_STYLE.fontWeightBold).toBe("bold");
    expect(TERMINAL_STYLE.fontWeight).toBe("normal");
    expect(TERMINAL_STYLE.drawBoldTextInBrightColors).toBe(true);
    // AA for normal text; xterm halves it for dim, so dim stays dim.
    expect(TERMINAL_STYLE.minimumContrastRatio).toBe(4.5);
    expect(TERMINAL_STYLE.theme).toBe(TERMINAL_THEME);
  });

  it("is the one both terminals draw with", () => {
    const sources = import.meta.glob<string>(["../components/TerminalView.tsx", "../components/ReplayTerminal.tsx"], { query: "?raw", import: "default", eager: true });
    expect(Object.keys(sources)).toHaveLength(2);
    for (const source of Object.values(sources)) {
      expect(source).toContain("...TERMINAL_STYLE");
      expect(source).not.toMatch(/background:\s*"#/);
    }
  });
});
