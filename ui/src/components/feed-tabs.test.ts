/**
 * 0.5.2 (PR #257's Happo review): the feed's tabs that fit, by the row's
 * own room: hidden from the end ("This Mac" first), never wrapping, and the
 * chosen tab always among those shown.
 */
import { describe, expect, it } from "vitest";
import { fitTabs } from "./FeedTabs";

// All, Needs you, Sessions, Tasks, Pools, Switches & limits, This Mac.
const WIDTHS = [36, 88, 76, 56, 54, 132, 80];
const ALL = WIDTHS.reduce((a, b) => a + b, 0) + 6 * 6;

describe("the feed's tabs in their row's room", () => {
  it("shows every tab when the row holds them", () => {
    expect(fitTabs(WIDTHS, ALL, 0)).toEqual([0, 1, 2, 3, 4, 5, 6]);
  });

  it("hides from the end, This Mac first", () => {
    expect(fitTabs(WIDTHS, ALL - 1, 0)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(fitTabs(WIDTHS, ALL - 90, 0)).toEqual([0, 1, 2, 3, 4]);
  });

  it("keeps the chosen tab in the row when it would be one to go", () => {
    // This Mac chosen: it takes the last place that fits.
    const shown = fitTabs(WIDTHS, ALL - 1, 6);
    expect(shown).toContain(6);
    expect(shown).toEqual([0, 1, 2, 3, 4, 6]);
    // The row never holds more than its room.
    const span = shown.reduce((sum, i) => sum + WIDTHS[i]!, 0) + 6 * (shown.length - 1);
    expect(span).toBeLessThanOrEqual(ALL - 1);
  });
});
