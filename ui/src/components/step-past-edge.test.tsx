/**
 * 0.4.1 — ↓ on the last line drawn of a windowed list scrolls it a row
 * and moves on to the line that brings in, in the same key press. Moving
 * a frame later (when the scroll's event re-drew the list) lost the
 * focus: keys pressed before that frame scrolled the list away from it.
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { useReducer, useRef, type JSX } from "react";
import { afterEach, expect, it } from "vitest";
import { stepPastEdge } from "./PlaceBoardParts";

afterEach(cleanup);

/** A list that draws one more line each time it's redrawn (as a window
 * moving down would), and steps past its last line on ↓. */
function Windowed(): JSX.Element {
  const [drawn, redraw] = useReducer((n: number) => n + 1, 3);
  const list = useRef<HTMLDivElement | null>(null);
  const box = useRef<HTMLDivElement | null>(null);
  const onKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    if (event.key !== "ArrowDown" || !list.current || !box.current) return;
    const lines = list.current.querySelectorAll("[data-rail-line]");
    if (lines[lines.length - 1]?.contains(event.currentTarget)) stepPastEdge(list.current, box.current, 1, 46, redraw);
  };
  return (
    <div ref={box}>
      <div ref={list}>
        {Array.from({ length: drawn }, (_, n) => (
          <div key={n} data-rail-line>
            <button type="button" data-rail-item={`row:${n}`} onKeyDown={onKeyDown}>
              Row {n}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}

it("moves on to the line the scroll brought in, within the key press", () => {
  const view = render(<Windowed />);
  view.getByText("Row 2").focus();
  for (const next of [3, 4, 5]) {
    fireEvent.keyDown(document.activeElement!, { key: "ArrowDown" });
    expect(document.activeElement?.textContent).toBe(`Row ${next}`);
  }
});
