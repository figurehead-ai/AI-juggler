/**
 * rc24/rc25 walk K1: after a session's page, every page showed macOS's
 * input-source mark (a small ⊖) at the bottom-left of the content area,
 * where the terminal's caret had been.
 *
 * WKWebView hides the mark only when the selection it reports next is a
 * caret somewhere that isn't editable. A field taken off the page while
 * it has focus does that (WebKit moves the caret to where the field
 * was). A field that loses focus while it stays on the page doesn't: a
 * click on anything that takes no focus (a header tab, a rail row) or a
 * blur() leaves no selection at all, and the mark stays where the caret
 * was, over every page after, until the window loses key. Moving focus to
 * a non-editable element (main, a dialog's button) or setting a caret a
 * moment later leaves it too. (Measured in a WKWebView harness, the
 * ⊖'s pixels counted: K1 report, 2026-11-10.)
 *
 * So when a text field loses focus to nothing editable, the caret goes,
 * in the same event, to a parked spot that isn't editable: an invisible,
 * aria-hidden character at the window's corner. The page's focus is left
 * as it was going (body, or the element taking it); a field the window
 * only lost along with the app keeps its caret.
 */

const NOT_TEXT = new Set(["button", "checkbox", "color", "file", "hidden", "image", "radio", "range", "reset", "submit"]);

/** Whether `node` takes typed text (and so gets macOS's caret mark). */
export function isTextField(node: unknown): node is HTMLElement {
  if (node instanceof HTMLTextAreaElement) return true;
  if (node instanceof HTMLInputElement) return !NOT_TEXT.has(node.type);
  return node instanceof HTMLElement && node.isContentEditable;
}

const PARK = "data-caret-park";

/** The parked spot's text: made on first use, kept on the page. */
function parking(doc: Document): Node {
  let park = doc.querySelector<HTMLElement>(`[${PARK}]`);
  if (!park) {
    park = doc.createElement("span");
    park.setAttribute(PARK, "");
    park.setAttribute("aria-hidden", "true");
    // Rendered (a caret needs a box) but never seen, hit or scrolled to.
    park.style.cssText = "position:fixed;left:0;top:0;width:1px;height:1px;overflow:hidden;opacity:0;pointer-events:none;user-select:text;-webkit-user-select:text";
    park.textContent = ".";
    doc.body.append(park);
  }
  return park.firstChild ?? park;
}

/** Whether `event` takes focus out of a text field to nothing that
 * takes the caret: not when the next field does, nor when the window
 * only lost focus with the app. */
export function caretLeaves(event: Pick<FocusEvent, "target" | "relatedTarget">, doc: Document = document): boolean {
  if (!isTextField(event.target)) return false;
  // The app went to the back: focus goes nowhere, yet the field is
  // still the focused one.
  if (event.relatedTarget === null && doc.activeElement === event.target) return false;
  // The next field's caret moves the mark itself.
  return !isTextField(event.relatedTarget);
}

/** Installs the rule once for the window. Returns its undo. */
export function guardCaretMark(doc: Document = document): () => void {
  const on = (event: FocusEvent) => {
    if (!caretLeaves(event, doc)) return;
    doc.getSelection()?.collapse(parking(doc), 0);
  };
  doc.addEventListener("focusout", on, true);
  return () => doc.removeEventListener("focusout", on, true);
}
