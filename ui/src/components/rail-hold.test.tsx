/**
 * 0.4.1 — the line under the pointer or focus holds still when lines are
 * drawn, removed or resized above it (3efc37b61: a Needs you notice
 * arriving pushed a place's Open door ~94 px down under the click).
 * jsdom has no layout, so the list here lays itself out: each line's
 * height is its `data-h`, the spacers' their style.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { useRef, useState, type JSX } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LINE_KEY, useHoldStill } from "./rail-hold";

type Line = { key: string; h: number };

const VIEW = 300;

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

/** The fake layout: the list's children stacked, the box scrolled. A box
 * between them (`nested`, as the rail's list in the rail's scroll) is
 * styled to scroll but grows with its content, so it never does. */
function layout(box: HTMLElement, nested: boolean) {
  const inner = nested ? firstOf(box) : null;
  const list = firstOf(inner ?? box);
  const stacked = () => [...list.children];
  const height = () => stacked().reduce((sum, child) => sum + heightOf(child), 0);
  let top = 0;
  Object.defineProperty(box, "clientHeight", { configurable: true, get: () => VIEW });
  Object.defineProperty(box, "scrollHeight", { configurable: true, get: () => Math.max(VIEW, height()) });
  Object.defineProperty(box, "scrollTop", {
    configurable: true,
    get: () => top,
    // As a browser does: never past either end.
    set: (value: number) => {
      top = Math.max(0, Math.min(value, Math.max(VIEW, height()) - VIEW));
    },
  });
  if (inner) {
    Object.defineProperty(inner, "clientHeight", { configurable: true, get: height });
    Object.defineProperty(inner, "scrollHeight", { configurable: true, get: height });
    Object.defineProperty(inner, "scrollTop", { configurable: true, get: () => 0, set: () => {} });
  }
  box.getBoundingClientRect = () => rect(0, VIEW);
  const rectOf = (element: Element): DOMRect => {
    let y = 0;
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

function Board({ lines, nested = false }: { lines: Line[]; nested?: boolean }): JSX.Element {
  const root = useRef<HTMLDivElement | null>(null);
  const [box, setBox] = useState<HTMLDivElement | null>(null);
  const hold = useHoldStill(root);
  const list = (
    <div ref={root} {...hold.props}>
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
    <div
      data-testid="box"
      style={{ overflowY: "auto" }}
      ref={(element) => {
        if (element && element !== box) {
          rectOf = layout(element, nested);
          scroller = element;
          setBox(element);
        }
      }}
    >
      {nested ? <div style={{ overflowY: "auto" }}>{list}</div> : list}
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
    expect(view.getByTestId("box").scrollTop).toBe(94);
    // The notice is above, out of view, and the pill says so.
    expect(view.getByText("notice:signin").closest(`[${LINE_KEY}]`)!.getBoundingClientRect().bottom).toBeLessThanOrEqual(0);
    expect(view.getByTestId("rail-new-above").textContent).toBe("1 new above");
  });

  it("scrolls the box that scrolls, past one that grows with its content (the rail's list in the rail)", () => {
    const view = render(<Board lines={HEADERS} nested />);
    const was = topOf(view, "header:codex");
    fireEvent.pointerMove(view.getByText("header:codex"));
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} nested />);
    expect(topOf(view, "header:codex")).toBe(was);
    expect(view.getByTestId("box").scrollTop).toBe(94);
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
    expect(box.scrollTop).toBe(134);
  });

  it("the pill scrolls up to it and goes", () => {
    const view = render(<Board lines={HEADERS} />);
    fireEvent.pointerMove(view.getByText("header:codex"));
    view.rerender(<Board lines={[...NOTICE, ...HEADERS]} />);
    act(() => view.getByTestId("rail-new-above").click());
    expect(view.queryByTestId("rail-new-above")).toBeNull();
    expect(topOf(view, "notice:signin")).toBeGreaterThanOrEqual(0);
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
