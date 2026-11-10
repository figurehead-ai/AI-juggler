/**
 * rc26 walk K1: a modal dialog opened from a focused terminal could leave
 * the terminal's field focused behind its scrim. macOS draws its
 * input-source mark (⊖) above every page layer, so the mark showed sharp
 * over the dimmed window, and keys went to the field behind. ⌘⇧N (New
 * project) from a terminal did that: its sheet moves no focus at all.
 *
 * So, once for the window: when an `aria-modal` dialog appears and, a
 * frame later (after the dialog's own first focus), a text field outside
 * every open dialog still has focus, focus moves into the newest dialog:
 * its `[data-autofocus]`, else its first control, else the dialog itself.
 * The field's caret then leaves with it (lib/caret-leaves). Only a field
 * is taken from: focus on the page itself stays as it was, so dialogs
 * that set their own first focus are unchanged.
 */
import { isTextField } from "./caret-leaves";

const MODAL = '[aria-modal="true"]';
const CONTROL = "button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], summary, [tabindex]:not([tabindex='-1'])";

/** Whether `node` sits in an open modal dialog. */
function inModal(node: Element | null): boolean {
  return !!node?.closest(MODAL);
}

/** Where focus may go in `dialog`, in order: what it marks, its
 * controls, the dialog itself. */
function focusTargets(dialog: HTMLElement): HTMLElement[] {
  return [...dialog.querySelectorAll<HTMLElement>("[data-autofocus]"), ...dialog.querySelectorAll<HTMLElement>(CONTROL), dialog];
}

/** Moves focus into the newest open dialog when a field behind it has
 * it: to the first of its targets that takes it (a hidden one doesn't).
 * Returns whether it moved. */
export function takeFocusFromBehind(doc: Document = document): boolean {
  const dialog = Array.from(doc.querySelectorAll<HTMLElement>(MODAL)).at(-1);
  const active = doc.activeElement;
  if (!dialog || !isTextField(active) || inModal(active)) return false;
  for (const target of focusTargets(dialog)) {
    if (target === dialog && !dialog.hasAttribute("tabindex")) dialog.tabIndex = -1;
    target.focus({ preventScroll: true });
    if (doc.activeElement !== active) return true;
  }
  return false;
}

/** Installs the rule once for the window. Returns its undo. */
export function guardModalFocus(doc: Document = document): () => void {
  const win = doc.defaultView ?? window;
  let frame = 0;
  // Only what was added is looked at (a terminal's rows change many
  // times a second; none of them holds a dialog).
  const opens = (record: MutationRecord) =>
    record.type === "attributes"
      ? record.target instanceof Element && record.target.matches(MODAL)
      : Array.from(record.addedNodes).some((node) => node instanceof Element && (node.matches(MODAL) || !!node.querySelector(MODAL)));
  const check = (records: MutationRecord[]) => {
    if (frame || !records.some(opens)) return;
    // Two frames: the dialog's own first focus (an effect, or its next
    // frame) goes first.
    frame = win.requestAnimationFrame(() => {
      frame = win.requestAnimationFrame(() => {
        frame = 0;
        takeFocusFromBehind(doc);
      });
    });
  };
  const observer = new MutationObserver(check);
  observer.observe(doc.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["aria-modal"] });
  return () => {
    observer.disconnect();
    if (frame) win.cancelAnimationFrame(frame);
  };
}
