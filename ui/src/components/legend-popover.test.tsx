/**
 * The indicator legend (AJ-E403 Req.6) is a dialog with no control of its
 * own, and the page under it is inert while it is open (AJ-E3011 round 4),
 * its "?" too. Focus moves onto the legend as it opens, never left on the
 * page's body (the 0.5 gate triage: the cursor spec found no control and
 * focus nowhere); Escape closes it and puts focus back on the "?".
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { LegendPopover } from "./LegendPopover";

afterEach(cleanup);

it("opening the legend puts focus on it; Escape closes it and returns focus to the ?", async () => {
  render(<LegendPopover />);
  const opener = screen.getByRole("button", { name: "Indicator legend" });
  opener.focus();
  fireEvent.click(opener);
  const legend = await screen.findByRole("dialog", { name: "What the icons mean" });
  expect(document.activeElement).toBe(legend);
  fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog", { name: "What the icons mean" })).toBeNull();
  expect(document.activeElement).toBe(opener);
});
