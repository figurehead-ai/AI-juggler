/**
 * Design review 54 vitest — Re-create the office? makes its promise once:
 * the subtitle says what changes, and "where each area is" is said by the
 * Stays line and the example, not again in the intro.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RecreateSheet } from "./RecreateSheet";

afterEach(cleanup);

describe("Re-create the office?", () => {
  it("the subtitle is one sentence; the place is promised by Stays and the example only", () => {
    render(<RecreateSheet example="Claude app" onCancel={() => {}} onConfirm={() => {}} />);
    const sheet = screen.getByTestId("floor-recreate-sheet");
    expect(sheet.textContent).toContain("Every area gets a new look and desk layout.");
    expect(sheet.textContent).not.toContain("The order stays");
    expect(sheet.textContent).not.toContain("where you know it");
    expect(sheet.textContent).toContain("Where each area is.");
    expect(screen.getByTestId("floor-recreate-example").textContent).toContain("keeps its place on floor 1");
  });
});
