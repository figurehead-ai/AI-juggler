/**
 * 0.4.1 — the terminal's colours (the maintainer's walk: every CLI drew in
 * plain white). The backend now tells a tool it runs in a 256-colour,
 * 24-bit terminal (`juggler_supervisor::terminal_env`); this is the
 * palette its 16 ANSI colours draw in, from the app's design tokens
 * (styles.css), on the terminal's own near-black.
 *
 * The terminal stays dark in the light theme too, as before: the tools'
 * own themes (Claude Code's, OpenCode's) pick colours for a dark
 * background unless told otherwise, and a light terminal under a dark
 * tool theme is what turns their text unreadable.
 *
 * Every colour clears 6.5:1 on the background except black (a fill, not
 * a text colour); `minimumContrastRatio` lifts any cell under 4.5:1 (WCAG
 * AA) at draw time — a tool's own 24-bit colours too — and dim text keeps
 * half that ratio in xterm, so it still reads as dim.
 */
import type { ITerminalOptions, ITheme } from "@xterm/xterm";

/** The 16 ANSI colours (8 normal, 8 bright) and the terminal's own. */
export const TERMINAL_THEME = {
  background: "#05070b",
  foreground: "#cdd5e0",
  // The cursor and selection as they were (the accent's hue).
  cursor: "#7c7cf4",
  selectionBackground: "#7c7cf440",
  black: "#2f3645", // --color-hairline
  red: "#f87171", // --color-danger
  green: "#34d399", // --color-success
  yellow: "#fbbf24", // --color-warning
  blue: "#60a5fa",
  magenta: "#c792ea",
  cyan: "#22d3ee", // --color-live
  white: "#d2d8e1", // --color-ink-2
  brightBlack: "#8e97a6", // --color-faint
  brightRed: "#fca5a5",
  brightGreen: "#6ee7b7",
  brightYellow: "#fde68a",
  brightBlue: "#93c5fd",
  brightMagenta: "#e0b8ff",
  brightCyan: "#67e8f9",
  brightWhite: "#f3f5f9", // --color-ink
} as const satisfies ITheme;

/** How styles draw: bold in the bold face (and the bright colour, as
 * Terminal and iTerm do), every cell readable. Italic, dim and underline
 * need no option: the DOM and WebGL renderers draw all three. */
export const TERMINAL_STYLE = {
  theme: TERMINAL_THEME,
  fontWeight: "normal",
  fontWeightBold: "bold",
  drawBoldTextInBrightColors: true,
  minimumContrastRatio: 4.5,
} as const satisfies ITerminalOptions;
