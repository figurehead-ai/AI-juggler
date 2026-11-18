/**
 * D-118's mock: Home's counts line reads "7 open · 6 finished today · 1 to
 * review"; "finished today" shows the feed's Finished chip.
 */
import { afterEach, describe, expect, it } from "vitest";
import { homeCounts } from "./home-counts";
import { forgetFeedState, keptFilters } from "./feed-state";

afterEach(forgetFeedState);

describe("Home's counts line", () => {
  it("says open, finished today and to review in the mock's order, leaving out what is 0", () => {
    const parts = homeCounts({ open: 7, finishedToday: 6, toReview: 1, inactive: 0, hidden: [] });
    expect(parts.map((part) => part.words)).toEqual(["7 open", "6 finished today", "1 to review"]);
    expect(homeCounts({ open: 2, finishedToday: 0, toReview: 0, inactive: 1, hidden: [{ app: "Codex", count: 3 }] }).map((part) => part.words)).toEqual(["2 open", "1 inactive session", "3 in Codex hidden"]);
  });

  it("finished today shows the feed's Finished chip and keeps it, as the chip would", () => {
    homeCounts({ open: 1, finishedToday: 2, toReview: 0, inactive: 0, hidden: [] }).find((part) => part.key === "finished")!.run();
    expect(keptFilters().category).toBe("finished");
  });
});
