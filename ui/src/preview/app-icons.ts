/**
 * D-38 — the preview's app icons (the dev preview and the gallery's
 * captures, never the app): made-up test images drawn here as `data:`
 * URIs, plain shapes on colour, nobody's logo. No app's real icon is in
 * the repository; the app reads each one from its bundle at run time.
 * Used only when there is no backend to ask and no test seam set (a few
 * hundred bytes: drawn at once, so two captures of a screen match).
 */

/** A test icon: a macOS-shaped square in `from`→`to`, with `glyph` (SVG
 * markup on a 100-unit grid) in white. */
export function testIcon(from: string, to: string, glyph: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${from}"/><stop offset="1" stop-color="${to}"/></linearGradient></defs><rect x="9" y="9" width="82" height="82" rx="19" fill="url(#g)"/><rect x="9.5" y="9.5" width="81" height="81" rx="18.5" fill="none" stroke="#000" stroke-opacity=".12"/>${glyph}</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/** By the host registry's app id. */
export const PREVIEW_APP_ICONS: Record<string, string> = {
  // A desktop app: a ring and a dot.
  "claude-app": testIcon("#7a8cff", "#4b5bd6", '<circle cx="50" cy="50" r="20" fill="none" stroke="#fff" stroke-width="8"/><circle cx="50" cy="50" r="6" fill="#fff"/>'),
  // Another desktop app: a speech bubble.
  "codex-app": testIcon("#5bc489", "#2c8a5a", '<path d="M30 34h40a6 6 0 0 1 6 6v18a6 6 0 0 1-6 6H46l-12 9v-9h-4a6 6 0 0 1-6-6V40a6 6 0 0 1 6-6z" fill="#fff"/>'),
  // A terminal: a prompt.
  terminal: testIcon("#3a4152", "#1b1f29", '<path d="M30 38l14 12-14 12" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/><path d="M50 64h20" stroke="#fff" stroke-width="7" stroke-linecap="round"/>'),
  // An editor: two brackets.
  cursor: testIcon("#4a4f5c", "#22252d", '<path d="M40 34L26 50l14 16M60 34l14 16-14 16" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>'),
  // AI Juggler's own place keeps its own mark (IconJuggler).
};
