/**
 * The 0.5.0 walk, F6: New pool's step 1 at 1024x680 put the ready-made
 * team cards two to a row by the window's width (`sm:`), in a column only
 * about 300 px wide: each card squeezed to about 140 px, a word to a line,
 * the first one blank. Choice cards go side by side only when the group
 * itself has the room (a container query), else one to a row.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { NewPoolTemplate } from "./NewPoolTemplate";

afterEach(cleanup);

describe("New pool's template cards (F6)", () => {
  it("go two to a row by the group's own room, never the window's", () => {
    render(<NewPoolTemplate chosen="blank" onChoose={() => undefined} first />);
    const group = screen.getByRole("radiogroup", { name: "How to start" });
    expect(group.className).toContain("@xl:grid-cols-2");
    expect(group.className).not.toContain("sm:grid-cols-2");
    expect(group.parentElement!.className).toContain("@container");
    // Start from scratch spans the row only where there are two.
    const scratch = screen.getAllByTestId("new-pool-template").find((card) => card.dataset.template === "blank")!;
    expect(scratch.className).toContain("@xl:col-span-2");
    expect(scratch.className).not.toContain("sm:col-span-2");
  });
});
