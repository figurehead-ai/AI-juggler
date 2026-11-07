/**
 * AJ-E2103 R5: dragging a row by its grip moves it in Your order — it
 * lands where the pointer lets go, says "Moved to position n of m", and
 * is kept; Escape (or a cancelled pointer) moves nothing.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { useRef, type JSX } from "react";
import { landing, useArrange, type Arrange } from "./rail-arrange";
import { railOrder, resetRailOrderCache } from "../lib/rail-order-store";
import { itemsOf, type RailItem } from "../lib/rail-items";
import { instance } from "../test/fixtures";

beforeEach(() => {
  localStorage.clear();
  resetRailOrderCache();
});
afterEach(cleanup);

/** Three open sessions' rows, 40 px tall one under the other, each with
 * its grip; `seen` gets the arrange hook's latest state. */
function Rows({ items, seen }: { items: RailItem[]; seen: (arrange: Arrange) => void }): JSX.Element {
  const root = useRef<HTMLDivElement>(null);
  const arrange = useArrange({ root, enabled: true, lens: "place", groups: [], arranged: new Map([["g", items]]), drawn: () => items.length, showAll: () => {} });
  seen(arrange);
  return (
    <div ref={root}>
      {items.map((item, index) => (
        <div
          key={item.key}
          data-order-group="g"
          data-order-row={item.key}
          ref={(row) => {
            if (row) row.getBoundingClientRect = () => DOMRect.fromRect({ x: 0, y: index * 40, width: 200, height: 40 });
          }}
        >
          <span data-testid={`grip-${index}`} onPointerDown={(event) => arrange.gripRow(event, "g", item.key)} />
        </div>
      ))}
    </div>
  );
}

function setUp() {
  const items = itemsOf({ sessions: ["a", "b", "c"].map((id) => instance({ id: `s-${id}`, status: "running" })), conversations: [], projects: [], seen: {} }).slice(0, 3);
  let latest: Arrange | null = null;
  const view = render(<Rows items={items} seen={(arrange) => (latest = arrange)} />);
  return { items, view, now: () => latest };
}


it("lands the row where it's let go, says where, and keeps it", () => {
  const { items, view, now } = setUp();
  fireEvent.pointerDown(view.getByTestId("grip-0"), { button: 0, clientY: 20 });
  // Past the middle of the second row (60), not of the third (100).
  fireEvent.pointerMove(window, { clientY: 75 });
  expect(now()?.drag?.to).toBe(1);
  fireEvent.pointerUp(window, { clientY: 75 });
  expect(now()?.said).toBe("Moved to position 2 of 3");
  const rows = railOrder().rows;
  expect(rows.indexOf(items[1]!.key)).toBeLessThan(rows.indexOf(items[0]!.key));
});

it("Escape and a cancelled pointer move nothing", () => {
  const { view, now } = setUp();
  fireEvent.pointerDown(view.getByTestId("grip-0"), { button: 0, clientY: 20 });
  fireEvent.pointerMove(window, { clientY: 110 });
  fireEvent.keyDown(window, { key: "Escape" });
  expect(now()?.said).toBe("Move cancelled");
  expect(now()?.drag).toBeNull();
  fireEvent.pointerDown(view.getByTestId("grip-0"), { button: 0, clientY: 20 });
  fireEvent.pointerMove(window, { clientY: 110 });
  fireEvent.pointerCancel(window, { clientY: 110 });
  expect(railOrder().rows).toEqual([]);
});

it("counts the rows whose middle is above the pointer, leaving out the dragged one", () => {
  const boxes = ["a", "b", "c"].map((key, index) => ({ key, top: index * 40, bottom: index * 40 + 40 }));
  expect(landing(10, boxes, "c")).toBe(0);
  expect(landing(65, boxes, "a")).toBe(1);
  expect(landing(200, boxes, "a")).toBe(2);
});
