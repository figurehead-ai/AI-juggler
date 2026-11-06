/**
 * Walk K2: a project's pool card counts its ended sessions from the
 * pool's count, not from the first page of members (a 500-session pool
 * said "50 ended").
 */
import { describe, expect, it } from "vitest";
import { poolStates } from "./ProjectPool";

describe("the pool card's states", () => {
  const page = ["Working", "Asks you", ...Array<string>(48).fill("Ended")];

  it("takes the ended ones from the pool's count", () => {
    expect(poolStates(page, 498)).toBe("1 working · 1 asks you · 498 ended");
  });

  it("counts the page while the pool's count isn't known yet", () => {
    expect(poolStates(page, null)).toBe("1 working · 1 asks you · 48 ended");
  });

  it("says nothing ended when none has", () => {
    expect(poolStates(["Working"], 0)).toBe("1 working");
  });
});
