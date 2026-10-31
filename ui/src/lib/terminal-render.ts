/**
 * 0.3.2 — how a terminal draws (the maintainer's 0.3.0 walk: Claude Code's
 * frames and bars "looked broken"). The DOM renderer drew block and box
 * characters from whatever fallback font WebKit picked; the WebGL renderer
 * draws them itself (custom glyphs). The fonts are names WebKit resolves
 * for a canvas: SF Mono by name often isn't found, ui-monospace is SF Mono.
 */
import type { Terminal } from "@xterm/xterm";

/** The terminal's fonts, in the order WebKit should try them. */
export const TERMINAL_FONTS = "'SF Mono', ui-monospace, Menlo, 'JetBrains Mono', monospace";

/** Loads the WebGL renderer into an opened terminal, with the first
 * terminal shown (never at start). A lost GL context, or none, leaves the
 * DOM renderer. `alive` says the terminal is still the one on screen. */
export function loadGlyphRenderer(term: Terminal, alive: () => boolean): void {
  void import("@xterm/addon-webgl").then(
    ({ WebglAddon }) => {
      if (!alive()) return;
      try {
        const gl = new WebglAddon();
        gl.onContextLoss(() => gl.dispose());
        term.loadAddon(gl);
      } catch {
        // No WebGL2 here: the DOM renderer stays.
      }
    },
    () => null,
  );
}
