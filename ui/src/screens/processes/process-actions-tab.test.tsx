/**
 * R9 (the 0.5.0 re-walk, SK48-14): on Processes, Tab went from the search
 * box to "Pause all": no row's Stop… was in the Tab order (every row
 * button had tabIndex -1), so a keyboard user reached it only through
 * Enter on the row. The focused row's actions now take Tab (the row, then
 * its Stop… and More, then on); the other rows' stay out until their row
 * is focused, so Tab never walks every row's buttons (A11Y-4, LST-4).
 *
 * The other half of the walk's note, "every row twice to assistive tech",
 * is WebKit's table model, not the page: an AXTable lists its cells under
 * each AXRow and again under each AXColumn (VoiceOver reads rows). The
 * table is one treegrid, named once; lab_drive no longer walks AXColumn.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ProcessesScreen } from "./ProcessesScreen";
import { forgetProcessRows } from "./useProcessRows";

afterEach(cleanup);
beforeEach(() => {
  forgetProcessRows();
  try {
    localStorage.removeItem("aij.processes.view");
  } catch {
    // no storage: the default view
  }
});

describe("R9 a row's actions are reachable by Tab", () => {
  it("Tab goes from the focused row to its Stop… and its More, and other rows' buttons stay out", async () => {
    const user = userEvent.setup();
    const { container } = render(<ProcessesScreen />);
    const stopOf = (pid: number) => container.querySelector<HTMLElement>(`[data-testid="process-stop-${pid}"]`);
    await waitFor(() => expect(stopOf(7301)).not.toBeNull());
    const grids = container.querySelectorAll('[role="treegrid"]');
    expect(grids).toHaveLength(1);
    expect(grids[0]!.getAttribute("aria-label")).toBeTruthy();
    const row = stopOf(7301)!.closest<HTMLElement>('[role="row"]')!;
    row.focus();
    await waitFor(() => expect(row.getAttribute("aria-selected")).toBe("true"));
    expect(stopOf(7301)!.tabIndex).toBe(0);
    await user.tab();
    expect(document.activeElement).toBe(stopOf(7301));
    await user.tab();
    expect(document.activeElement?.getAttribute("aria-label") ?? "").toMatch(/^More for /);
    // Every other row's buttons are out of the Tab order.
    const others = [...container.querySelectorAll<HTMLElement>('[role="row"] button')].filter((button) => button.closest('[role="row"]') !== row);
    expect(others.length).toBeGreaterThan(0);
    for (const button of others) expect(button.tabIndex).toBe(-1);
  });
});
