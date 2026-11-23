/**
 * AJ-E2327 — the user's own change isn't held: Add on the rail's "Running,
 * not added" took the offer under the pointer away and put its App, then
 * its sessions, into Open now above; the hold (made for notices arriving)
 * scrolled the rail's top out of view. With `ownChange()` the list keeps
 * its scroll position. The fake layout is `test/rail-hold-layout`.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { useRef, useState, type JSX } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LINE_KEY, useHoldStill } from "./rail-hold";
import { layout, NOWHERE, type Line } from "../test/rail-hold-layout";

let rectOf: ((element: Element) => DOMRect) | null = null;

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return rectOf ? rectOf(this) : NOWHERE;
  });
});
afterEach(() => {
  cleanup();
  rectOf = null;
  vi.restoreAllMocks();
});

function topOf(view: ReturnType<typeof render>, key: string): number {
  return view.getByText(key).closest(`[${LINE_KEY}]`)!.getBoundingClientRect().top;
}

describe("AJ-E2327: the user's own change isn't held", () => {
  function Own({ lines, own }: { lines: Line[]; own: boolean }): JSX.Element {
    const root = useRef<HTMLDivElement | null>(null);
    const [box, setBox] = useState<HTMLDivElement | null>(null);
    const [slot, setSlot] = useState<HTMLDivElement | null>(null);
    const hold = useHoldStill(root, { slot });
    const list = (
      <div ref={root} {...hold.props}>
        {hold.before}
        {lines.map((line) => (
          <div key={line.key} {...{ [LINE_KEY]: line.key }} data-h={line.h}>
            <button type="button" onClick={() => own && hold.ownChange()}>
              {line.key}
            </button>
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
                rectOf = layout(element, false, slot);
                setBox(element);
              }
            }}
          >
            {list}
          </div>
        )}
      </div>
    );
  }

  // Open now with one App, then "Running, not added" with two offers.
  const BEFORE: Line[] = [
    { key: "section:open", h: 30 },
    { key: "header:work", h: 32 },
    { key: "section:offered", h: 30 },
    { key: "offer:team-a", h: 32 },
    { key: "offer:lab", h: 32 },
  ];
  // Add on team-a: its offer goes, its App's header arrives in Open now,
  // then its sessions under it (the 0.5.0 walk: the rail's top went).
  const AFTER: Line[] = [
    { key: "section:open", h: 30 },
    { key: "header:work", h: 32 },
    { key: "header:team-a", h: 32 },
    { key: "row:t1", h: 44 },
    { key: "row:t2", h: 44 },
    { key: "section:offered", h: 30 },
    { key: "offer:lab", h: 32 },
  ];

  // The pointer rests over the next offer down; Add was team-a's.
  function add(own: boolean) {
    const view = render(<Own lines={BEFORE} own={own} />);
    fireEvent.pointerMove(view.getByText("offer:lab"));
    fireEvent.click(view.getByText("offer:team-a"));
    view.rerender(<Own lines={AFTER} own={own} />);
    return view;
  }

  it("Add keeps the rail's scroll position: its top stays in view, no pill", () => {
    const view = add(true);
    expect(view.getByTestId("box").scrollTop).toBe(0);
    expect(topOf(view, "section:open")).toBe(0);
    expect(view.queryByTestId("rail-new-above")).toBeNull();
  });

  it("before (no own change): the same Add was held, scrolling the top out of view", () => {
    const view = add(false);
    expect(view.getByTestId("box").scrollTop).toBeGreaterThan(0);
    expect(topOf(view, "section:open")).toBeLessThan(0);
  });
});
