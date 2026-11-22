/**
 * AJ-E3603 — the ring where a page puts the keyboard by script (the app's
 * `<main>`, a page's name: never a Tab stop, `tabindex="-1"` and
 * `outline-none`) shows only when the keyboard brought it there. Skip to
 * page, ↩ on a rail row or ⌘⇧A then show where the keys now are (WCAG
 * 2.4.7); a click on a tab, or a page opened before any key was pressed
 * (a gallery's capture), draws no accent line down the main pane.
 *
 * So, once for the window, the root says how the person last acted:
 * `data-input="keyboard"` after a key (a modifier alone is no key, as
 * ⌘-click is a click), `"pointer"` after a press of the pointer, nothing
 * before either. `styles.css` reads it.
 */

const MODIFIERS = new Set(["Meta", "Control", "Alt", "Shift", "CapsLock", "Fn"]);

/** Starts keeping `root.dataset.input`; the returned function stops it. */
export function trackInputModality(root: HTMLElement = document.documentElement): () => void {
  const key = (event: KeyboardEvent) => {
    if (!MODIFIERS.has(event.key)) root.dataset.input = "keyboard";
  };
  const pointer = () => {
    root.dataset.input = "pointer";
  };
  window.addEventListener("keydown", key, true);
  window.addEventListener("pointerdown", pointer, true);
  return () => {
    window.removeEventListener("keydown", key, true);
    window.removeEventListener("pointerdown", pointer, true);
  };
}
