/**
 * 0.4.1 — the line under the pointer or focus holds still when lines are
 * drawn, removed or resized above it (3efc37b61: a Needs you notice
 * arriving pushed a place's Open door ~94 px down under the click).
 * jsdom has no layout, so the list here lays itself out: each line's
 * height is its `data-h`, the spacers' their style. The pill's bar takes
 * BAR px above the list's box (the box's view starts below it, as the
 * rail's list under its head); a pill put inside the box instead is laid
 * over the view's top, where 0.4.0's sat on the section head's words.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { useRef, useState, type JSX } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LINE_KEY, useHoldStill } from "./rail-hold";

type Line = { key: string; h: number };

const VIEW = 300;
const BAR = 28;
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
const NOWHERE = rect(0, 0, 0);

/** The fake layout: the slot's bar (if any) on top, then the box, VIEW
 * high together; the list's children stacked in the box, scrolled. A box
 * between them (`nested`, as the rail's list in the rail's scroll) is
 * styled to scroll but grows with its content, so it never does. */
function layout(box: HTMLElement, nested: boolean, slot: HTMLElement) {
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

let rectOf: ((element: Element) => DOMRect) | null = null;
let scroller: HTMLElement | null = null;

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return rectOf ? rectOf(this) : NOWHERE;
  });
  // Brought to the box's top edge when above it ("nearest"; jsdom has none).
  HTMLElement.prototype.scrollIntoView = function (this: HTMLElement) {
    const top = this.getBoundingClientRect().top;
    if (scroller && top < 0) scroller.scrollTop += top;
  };
});
afterEach(() => {
  cleanup();
  rectOf = null;
  scroller = null;
  vi.restoreAllMocks();
  Reflect.deleteProperty(HTMLElement.prototype, "scrollIntoView");
});

function Board({ lines, nested = false, onKey }: { lines: Line[]; nested?: boolean; onKey?: () => void }): JSX.Element {
  const root = useRef<HTMLDivElement | null>(null);
  const [box, setBox] = useState<HTMLDivElement | null>(null);
  const [slot, setSlot] = useState<HTMLDivElement | null>(null);
  const hold = useHoldStill(root, { slot });
  // PlaceBoard's own keys (↑/↓, moves), beside the hook's handlers.
  const props = { ...hold.props, onKeyDown: onKey };
  const list = (
    <div ref={root} {...props}>
      {hold.before}
      {lines.map((line) => (
        <div key={line.key} {...{ [LINE_KEY]: line.key }} data-h={line.h}>
          <button type="button">{line.key}</button>
        </div>
      ))}
      {hold.after}
    </div>
  );
  return (
    <div>
      <div data-testid="slot" ref={setSlot} />
      {slot && (
        <div
          data-testid="box"
          style={{ overflowY: "auto" }}
          ref={(element) => {
            if (element && element !== box) {
              rectOf = layout(element, nested, slot);
              scroller = element;
              setBox(element);
            }
          }}
        >
          {nested ? <div style={{ overflowY: "auto" }}>{list}</div> : list}
        </div>
      )}
    </div>
  );
}

const HEADERS: Line[] = [
  { key: "section:open", h: 30 },
  { key: "header:claude", h: 32 },
  { key: "row:a", h: 44 },
  { key: "header:codex", h: 32 },
  { key: "row:b", h: 44 },
];
// A Needs you notice, with its section head: ~94 px (3efc37b61).
const NOTICE: Line[] = [
  { key: "section:needs", h: 30 },
  { key: "notice:signin", h: 64 },
];
const LONG: Line[] = Array.from({ length: 20 }, (_, index) => ({ key: `row:x${index}`, h: 44 }));

function topOf(view: ReturnType<typeof render>, key: string): number {
  return view.getByText(key).closest(`[${LINE_KEY}]`)!.getBoundingClientRect().top;
}

describe("a line arriving above the one under the pointer", () => {
  it("leaves the header under the pointer where it was, at the top of a short list (a spacer below)", () => {
    const view = render(<Board lines={HEADERS} />);
    const was = topOf(view, "header:codex");
    fireEvent.pointerMove(view.getByText("header:codex"));
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} />);
    expect(topOf(view, "header:codex")).toBe(was);
    // The notice's height, and the pill's bar's: the view starts lower.
    expect(view.getByTestId("box").scrollTop).toBe(94 + BAR);
    // The notice is above, out of view, and the pill says so.
    expect(view.getByText("notice:signin").closest(`[${LINE_KEY}]`)!.getBoundingClientRect().bottom).toBeLessThanOrEqual(BAR);
    expect(view.getByTestId("rail-new-above").textContent).toBe("1 new above");
  });

  it("scrolls the box that scrolls, past one that grows with its content (the rail's list in the rail)", () => {
    const view = render(<Board lines={HEADERS} nested />);
    const was = topOf(view, "header:codex");
    fireEvent.pointerMove(view.getByText("header:codex"));
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} nested />);
    expect(topOf(view, "header:codex")).toBe(was);
    expect(view.getByTestId("box").scrollTop).toBe(94 + BAR);
    expect(view.getByTestId("rail-new-above").textContent).toBe("1 new above");
  });

  it("scrolls by its height in a long list", () => {
    const lines = [...HEADERS, ...LONG];
    const view = render(<Board lines={lines} />);
    const box = view.getByTestId("box");
    box.scrollTop = 40;
    const was = topOf(view, "row:b");
    fireEvent.pointerMove(view.getByText("row:b"));
    view.rerender(<Board lines={[...NOTICE, ...lines]} />);
    expect(topOf(view, "row:b")).toBe(was);
    expect(box.scrollTop).toBe(134 + BAR);
  });

  it("the pill scrolls up to it and goes", () => {
    const view = render(<Board lines={HEADERS} />);
    fireEvent.pointerMove(view.getByText("header:codex"));
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} />);
    act(() => view.getByTestId("rail-new-above").click());
    expect(view.queryByTestId("rail-new-above")).toBeNull();
    expect(topOf(view, "notice:signin")).toBeGreaterThanOrEqual(0);
  });

  // 0.4.1: a control never covers text. The pill sat over the view's top,
  // on the section head's words ("Open now 11 · Your order").
  it("the pill takes its own space: no line's box in the list's view meets it", () => {
    const view = render(<Board lines={[...HEADERS, ...LONG]} />);
    const box = view.getByTestId("box");
    box.scrollTop = 40;
    fireEvent.pointerMove(view.getByText("row:b"));
    view.rerender(<Board lines={[...NOTICE, ...HEADERS, ...LONG]} />);
    const pill = view.getByTestId("rail-new-above");
    expect(box.contains(pill)).toBe(false);
    const at = pill.getBoundingClientRect();
    const frame = box.getBoundingClientRect();
    const lines = [...box.querySelectorAll<HTMLElement>(`[${LINE_KEY}]`)];
    expect(lines.length).toBeGreaterThan(0);
    for (const line of lines) {
      // What shows of it: clipped to the box's view.
      const shape = line.getBoundingClientRect();
      const top = Math.max(shape.top, frame.top);
      const bottom = Math.min(shape.bottom, frame.bottom);
      const meets = bottom > top && top < at.bottom && bottom > at.top;
      expect(meets, `${line.getAttribute(LINE_KEY)} under the pill`).toBe(false);
    }
  });

  it("the row under the pointer stays still as the pill's bar comes and goes", () => {
    const lines = [...HEADERS, ...LONG];
    const view = render(<Board lines={lines} />);
    const box = view.getByTestId("box");
    box.scrollTop = 40;
    const was = topOf(view, "row:x2");
    fireEvent.pointerMove(view.getByText("row:x2"));
    view.rerender(<Board lines={[...NOTICE, ...lines]} />);
    expect(view.getByTestId("rail-new-above")).toBeTruthy();
    expect(view.getByTestId("box").getBoundingClientRect().top).toBe(BAR);
    expect(topOf(view, "row:x2")).toBe(was);
    // Scrolled up to the notice by hand: the bar goes, the row still holds.
    box.scrollTop = 0;
    const now = topOf(view, "row:x2");
    act(() => {
      box.dispatchEvent(new Event("scroll"));
    });
    expect(view.queryByTestId("rail-new-above")).toBeNull();
    expect(topOf(view, "row:x2")).toBe(now);
  });

  it("the bar's pointer and keys are its own, not the list's (a portal in React's tree)", () => {
    const onKey = vi.fn();
    const view = render(<Board lines={HEADERS} onKey={onKey} />);
    fireEvent.pointerMove(view.getByText("header:codex"));
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} onKey={onKey} />);
    const pill = view.getByTestId("rail-new-above");
    fireEvent.keyDown(pill, { key: "ArrowDown" });
    expect(onKey).not.toHaveBeenCalled();
    fireEvent.keyDown(view.getByText("row:b"), { key: "ArrowDown" });
    expect(onKey).toHaveBeenCalledTimes(1);
  });

  it("the pill is a button in the bar, reached by Tab before the list", () => {
    const view = render(<Board lines={HEADERS} />);
    fireEvent.pointerMove(view.getByText("header:codex"));
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} />);
    const pill = view.getByRole("button", { name: "1 new above" });
    expect(view.getByTestId("slot").contains(pill)).toBe(true);
    expect(pill.compareDocumentPosition(view.getByTestId("box")) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("moves nothing for a line below it, and says nothing", () => {
    const view = render(<Board lines={HEADERS} />);
    fireEvent.pointerMove(view.getByText("header:claude"));
    view.rerender(<Board lines={[...HEADERS, { key: "row:c", h: 44 }]} />);
    expect(view.getByTestId("box").scrollTop).toBe(0);
    expect(view.queryByTestId("rail-new-above")).toBeNull();
  });

  it("with the pointer and focus elsewhere, the list draws as it comes", () => {
    const view = render(<Board lines={HEADERS} />);
    const was = topOf(view, "header:codex");
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} />);
    expect(topOf(view, "header:codex")).toBe(was + 94);
  });
});

describe("the line with keyboard focus", () => {
  // jsdom keeps no :focus-visible: here focus came from the keyboard
  // unless a test says it came from a click.
  let fromKeyboard = true;
  const matches: unknown = Object.getOwnPropertyDescriptor(Element.prototype, "matches")?.value;
  beforeEach(() => {
    fromKeyboard = true;
    vi.spyOn(Element.prototype, "matches").mockImplementation(function (this: Element, selector: string) {
      if (selector === ":focus-visible") return fromKeyboard && this === document.activeElement;
      if (typeof matches !== "function") throw new Error("no Element.matches");
      return Boolean(Reflect.apply(matches, this, [selector]));
    });
  });

  it("holds still when a line above it goes, at the top of the list (a spacer above)", () => {
    const view = render(<Board lines={[...NOTICE, ...HEADERS]} />);
    const row = view.getByText("row:b");
    act(() => row.focus());
    const was = topOf(view, "row:b");
    view.rerender(<Board lines={HEADERS} />);
    expect(topOf(view, "row:b")).toBe(was);
    expect(document.activeElement).toBe(row);
  });

  it("holds still when a line above it grows", () => {
    const lines = [...HEADERS, ...LONG];
    const view = render(<Board lines={lines} />);
    act(() => view.getByText("row:x3").focus());
    const was = topOf(view, "row:x3");
    view.rerender(<Board lines={lines.map((line) => (line.key === "row:a" ? { ...line, h: 64 } : line))} />);
    expect(topOf(view, "row:x3")).toBe(was);
    // A line that grew isn't new: no pill.
    expect(view.queryByTestId("rail-new-above")).toBeNull();
  });

  it("a row focused by a click, the pointer gone: the list draws as it comes", () => {
    fromKeyboard = false;
    const view = render(<Board lines={HEADERS} />);
    act(() => view.getByText("row:b").focus());
    const was = topOf(view, "row:b");
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} />);
    expect(topOf(view, "row:b")).toBe(was + 94);
  });
});
