/**
 * A step that opens in a scrolling panel is brought into its view; the
 * connector card's steps in the pool inspector are brought to its top,
 * under the panel's head (the gallery's Remove refused: a step that only
 * just fitted stayed where it was, and the panel never scrolled).
 */
import { afterEach, describe, expect, it } from "vitest";

import { revealInList } from "./reveal";

const box = (top: number, bottom: number) => ({ top, bottom, left: 0, right: 100, width: 100, height: bottom - top, x: 0, y: top, toJSON: () => ({}) }) as DOMRect;

function panel(stepTop: number, stepBottom: number) {
  const list = document.createElement("div");
  list.setAttribute("data-canvas-scroll", "");
  const step = document.createElement("div");
  list.append(step);
  document.body.append(list);
  list.getBoundingClientRect = () => box(100, 600);
  step.getBoundingClientRect = () => box(stepTop, stepBottom);
  return { list, step };
}

afterEach(() => document.body.replaceChildren());

describe("revealInList", () => {
  it("a step already in view stays where it is; one past the foot comes just into view", () => {
    const fits = panel(400, 560);
    revealInList(fits.step);
    expect(fits.list.scrollTop).toBe(0);
    const below = panel(500, 680);
    revealInList(below.step);
    expect(below.list.scrollTop).toBe(92);
  });

  it("top: its top to the panel's top, under its head, though it fitted", () => {
    const fits = panel(400, 560);
    revealInList(fits.step, 12, true);
    expect(fits.list.scrollTop).toBe(288);
  });
});
