import type { IDisposable, Terminal } from "@xterm/xterm";

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

/** Where keys typed while the field is let go of go: the terminal's
 * own input (xterm's `input` and `paste`). */
export type CaretInput = { input(data: string, wasUserInput?: boolean): void; paste(text: string): void };

type Held = { key: KeyboardEventInit & { key: string } } | { paste: string };

const MODIFIERS = new Set(["Shift", "Control", "Alt", "Meta", "CapsLock", "Fn", "Process", "Dead"]);

/** Whether `event` is plain text: what typing it into the field writes. */
function typed(event: Pick<KeyboardEventInit, "ctrlKey" | "metaKey"> & { key: string }): boolean {
  return event.key.length === 1 && !event.ctrlKey && !event.metaKey;
}

/** Keeps the mark on a focused terminal's caret when its frame moves:
 * once the frame has settled, the field gives focus up and takes it back
 * after the mark has gone, unless something else took it meanwhile.
 * Nothing typed in that gap is lost: while focus is on nothing (no other
 * field, no dialog), keys and pastes are held, in order, and handed to the
 * terminal as the gap ends, before it has focus again. ⌘ keys stay the
 * app's; ⌘V ends the gap at once so the paste lands in the terminal. */
export function followCaret(field: HTMLTextAreaElement, to: CaretInput, mayRefocus: () => boolean): { moved: () => void; dispose: () => void } {
  const doc = field.ownerDocument;
  const win = doc.defaultView ?? window;
  const place = () => {
    const box = field.getBoundingClientRect();
    return `${box.left},${box.top}`;
  };
  let at = place();
  let settle = 0;
  let refocus = 0;
  let held: Held[] | null = null;
  const focused = () => (at = place());
  field.addEventListener("focus", focused);
  // Focus on nothing: the keys are the terminal's.
  const holding = () => (!doc.activeElement || doc.activeElement === doc.body) && !doc.querySelector('[aria-modal="true"]');
  const onKey = (event: KeyboardEvent) => {
    if (!held || !holding() || event.isComposing || MODIFIERS.has(event.key)) return;
    if (event.metaKey) {
      if (event.key.toLowerCase() === "v") end();
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    const { key, code, keyCode, shiftKey, ctrlKey, altKey } = event;
    held.push({ key: { key, code, keyCode, shiftKey, ctrlKey, altKey } });
  };
  const onPaste = (event: ClipboardEvent) => {
    if (!held || !holding()) return;
    event.preventDefault();
    held.push({ paste: event.clipboardData?.getData("text/plain") ?? "" });
  };
  const release = () => {
    win.removeEventListener("keydown", onKey, true);
    win.removeEventListener("paste", onPaste, true);
    const keys = held ?? [];
    held = null;
    return keys;
  };
  // The held keys, in order: text as written, the rest as the terminal's
  // own key handling reads them (arrows, ⌃C, Return, ⌃Tab).
  const deliver = (keys: Held[]) => {
    for (const item of keys) {
      if ("paste" in item) to.paste(item.paste);
      else if (typed(item.key)) to.input(item.key.key);
      else field.dispatchEvent(new KeyboardEvent("keydown", { ...item.key, bubbles: true, cancelable: true }));
    }
  };
  const end = () => {
    win.clearTimeout(refocus);
    deliver(release());
    const active = doc.activeElement;
    if (field.isConnected && (!active || active === doc.body) && mayRefocus()) field.focus({ preventScroll: true });
  };
  const moved = () => {
    win.clearTimeout(settle);
    settle = win.setTimeout(() => {
      if (doc.activeElement !== field || place() === at) return;
      held = [];
      win.addEventListener("keydown", onKey, true);
      win.addEventListener("paste", onPaste, true);
      field.blur();
      win.clearTimeout(refocus);
      refocus = win.setTimeout(end, CARET_REFOCUS_MS);
    }, CARET_SETTLE_MS);
  };
  return {
    moved,
    dispose: () => {
      win.clearTimeout(settle);
      win.clearTimeout(refocus);
      release();
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

/**
 * 0.4.5 (AJ-E805, idle CPU): a session terminal's cursor blinks while the
 * terminal is in use, then rests. Blinking for ever, a focused terminal
 * kept the window drawing with nothing typed or written (xterm's blink is
 * an endless animation). It blinks again at the next key or the next
 * output.
 */

/** How long the cursor blinks after the last key or output. */
const BLINK_FOR_MS = 10_000;

type Blinking = { options: Pick<Terminal["options"], "cursorBlink">; onData: Terminal["onData"]; onWriteParsed: Terminal["onWriteParsed"] };

/** Blinks `term`'s cursor for [`BLINK_FOR_MS`] after each key and each
 * output, from now. Its listeners go with the terminal. */
export function blinkWhileUsed(term: Blinking): IDisposable {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const rest = () => {
    try {
      term.options.cursorBlink = false;
    } catch {
      // The terminal was closed meanwhile.
    }
  };
  const wake = () => {
    if (!term.options.cursorBlink) term.options.cursorBlink = true;
    clearTimeout(timer);
    timer = setTimeout(rest, BLINK_FOR_MS);
  };
  const keyed = term.onData(wake);
  const written = term.onWriteParsed(wake);
  wake();
  return {
    dispose: () => {
      clearTimeout(timer);
      keyed.dispose();
      written.dispose();
    },
  };
}
