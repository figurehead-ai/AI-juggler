import type { Terminal } from "@xterm/xterm";

/**
 * A terminal's field and macOS's input-source mark (⊖), which it draws at
 * the focused field's caret.
 *
 * rc26 walk K1: macOS moves the mark only when the caret does. A terminal whose
 * frame moved or changed size while focused (a session that ended gains
 * its "ran … · scrollback replayed" line below, which shrinks the frame;
 * ⌘B or ⌘⇧I move it) left the mark where the caret had been: below the
 * frame, over that line. Measured in a WKWebView harness: the mark goes
 * when the field loses focus to nothing (lib/caret-leaves parks the
 * caret), and comes back at the new place when it's focused again at
 * least ~120 ms later (sooner, it comes back at the old place).
 */
export const CARET_SETTLE_MS = 150;
export const CARET_REFOCUS_MS = 250;

/** Keeps the mark on a focused terminal's caret when its frame moves:
 * once the frame has settled, the field gives focus up and takes it back
 * after the mark has gone, unless something else took it meanwhile. */
export function followCaret(field: HTMLElement, mayRefocus: () => boolean): { moved: () => void; dispose: () => void } {
  const place = () => {
    const box = field.getBoundingClientRect();
    return `${box.left},${box.top}`;
  };
  let at = place();
  let settle = 0;
  let refocus = 0;
  const focused = () => (at = place());
  field.addEventListener("focus", focused);
  const moved = () => {
    window.clearTimeout(settle);
    settle = window.setTimeout(() => {
      if (field.ownerDocument.activeElement !== field || place() === at) return;
      field.blur();
      window.clearTimeout(refocus);
      refocus = window.setTimeout(() => {
        const active = field.ownerDocument.activeElement;
        if (field.isConnected && (!active || active === field.ownerDocument.body) && mayRefocus()) field.focus({ preventScroll: true });
      }, CARET_REFOCUS_MS);
    }, CARET_SETTLE_MS);
  };
  return {
    moved,
    dispose: () => {
      window.clearTimeout(settle);
      window.clearTimeout(refocus);
      field.removeEventListener("focus", focused);
    },
  };
}

/** An ended terminal takes no input, so its field never holds focus
 * (rc26 K1: the ⊖ stayed on a stopped session's terminal): it gives focus
 * up (its caret leaves with it) and can't take it again. */
export function holdInput(term: Pick<Terminal, "textarea" | "options">, gone: boolean): void {
  const field = term.textarea;
  term.options.disableStdin = gone;
  if (!field) return;
  if (gone && field.ownerDocument.activeElement === field) field.blur();
  field.disabled = gone;
}
