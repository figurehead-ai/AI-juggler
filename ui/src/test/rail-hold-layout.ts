/**
 * The rail hold's tests' fake layout (jsdom has none): each line's height
 * is its `data-h`, the spacers' their style; the pill's bar takes BAR px
 * above the list's box. Shared by `rail-hold.test.tsx` and
 * `rail-hold-own.test.tsx`.
 */
export type Line = { key: string; h: number };

const VIEW = 300;
export const BAR = 28;
const PILL = 24;

function heightOf(element: Element): number {
  if (!(element instanceof HTMLElement)) return 0;
  const h = element.dataset.h;
  if (h !== undefined) return Number(h);
  return parseFloat(element.style.height || "0") || 0;
}

function firstOf(element: Element): HTMLElement {
  const first = element.firstElementChild;
  if (!(first instanceof HTMLElement)) throw new Error("no first child");
  return first;
}

function rect(top: number, height: number, width = 240): DOMRect {
  return { top, bottom: top + height, left: 0, right: width, width, height, x: 0, y: top, toJSON: () => null };
}
export const NOWHERE = rect(0, 0, 0);

/** The fake layout: the slot's bar (if any) on top, then the box, VIEW
 * high together; the list's children stacked in the box, scrolled. A box
 * between them (`nested`, as the rail's list in the rail's scroll) is
 * styled to scroll but grows with its content, so it never does. */
export function layout(box: HTMLElement, nested: boolean, slot: HTMLElement) {
  const inner = nested ? firstOf(box) : null;
  const list = firstOf(inner ?? box);
  const stacked = () => [...list.children];
  const height = () => stacked().reduce((sum, child) => sum + heightOf(child), 0);
  const barH = () => (slot.childElementCount > 0 ? BAR : 0);
  const view = () => VIEW - barH();
  let top = 0;
  Object.defineProperty(box, "clientHeight", { configurable: true, get: view });
  Object.defineProperty(box, "scrollHeight", { configurable: true, get: () => Math.max(view(), height()) });
  Object.defineProperty(box, "scrollTop", {
    configurable: true,
    get: () => top,
    // As a browser does: never past either end.
    set: (value: number) => {
      top = Math.max(0, Math.min(value, Math.max(view(), height()) - view()));
    },
  });
  if (inner) {
    Object.defineProperty(inner, "clientHeight", { configurable: true, get: height });
    Object.defineProperty(inner, "scrollHeight", { configurable: true, get: height });
    Object.defineProperty(inner, "scrollTop", { configurable: true, get: () => 0, set: () => {} });
  }
  box.getBoundingClientRect = () => rect(barH(), view());
  slot.getBoundingClientRect = () => rect(0, barH());
  const rectOf = (element: Element): DOMRect => {
    if (slot.contains(element)) return element.closest("[data-testid='rail-new-above']") ? rect((BAR - PILL) / 2, PILL) : rect(0, barH());
    // Absolutely placed at the view's top, over the lines (0.4.0's pill).
    if (element.closest("[data-testid='rail-new-above']")) return rect(barH() + 4, PILL);
    let y = barH();
    for (const child of stacked()) {
      if (child === element || child.contains(element)) {
        return rect(y - top, heightOf(child));
      }
      y += heightOf(child);
    }
    return NOWHERE;
  };
  return rectOf;
}

