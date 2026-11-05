/** D-64's "Last 3 inactive", and one said as "Last inactive" (design
 * review #70: "Last 1 inactive" read as a glitch). */
import { describe, expect, it } from "vitest";
import { recentWords } from "./rail-apps";

describe("an App's last inactive sessions, in words", () => {
  it("says how many, up to three, and one without a number", () => {
    expect(recentWords(1)).toBe("Last inactive");
    expect(recentWords(2)).toBe("Last 2 inactive");
    expect(recentWords(9)).toBe("Last 3 inactive");
  });
});
