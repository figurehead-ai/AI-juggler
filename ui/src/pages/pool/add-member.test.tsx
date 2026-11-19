/**
 * D-126 §3: "Add a member" replaces the always-open "Add a session"
 * palette: closed, one button at the canvas's top left; pressed, the same
 * list (drag one on, or Add), headed by the button that folds it again.
 */
import { cleanup, fireEvent, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { openPage, resetCanvasTests, seedTeam } from "./canvas.test-kit";

beforeEach(resetCanvasTests);
afterEach(cleanup);

describe("Add a member (D-126 §3)", () => {
  it("starts closed, even with room beside the nodes; opens the list and folds it again", async () => {
    seedTeam();
    const view = await openPage();
    const palette = view.getByTestId("canvas-palette");
    const add = within(palette).getByRole("button", { name: "Add a member" });
    expect(add.getAttribute("aria-expanded")).toBe("false");
    expect(view.queryByTestId("canvas-palette-body")).toBeNull();
    fireEvent.click(add);
    const open = within(view.getByTestId("canvas-palette")).getByRole("button", { name: /^Add a member/ });
    expect(open.getAttribute("aria-expanded")).toBe("true");
    expect(view.getByTestId("canvas-palette-body")).toBeTruthy();
    fireEvent.click(open);
    expect(view.queryByTestId("canvas-palette-body")).toBeNull();
  });
});
