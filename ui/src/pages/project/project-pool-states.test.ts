/**
 * Walk K2: a project's pool card counts its ended sessions from the
 * pool's count, not from the first page of members (a 500-session pool
 * said "50 ended"). The 0.4.6 RC: an ended row says how it ended
 * ("Finished", "Stopped by you"), and the card said "50 finished · 500
 * ended"; ended rows are left to the pool's count by their state.
 */
import { describe, expect, it } from "vitest";
import { poolStates } from "./ProjectPool";

describe("the pool card's states", () => {
  const live = (word: string) => ({ word, ended: false });
  const gone = (word: string) => ({ word, ended: true });
  const page = [live("Working"), live("Asks you"), ...Array.from({ length: 48 }, () => gone("Ended"))];

  it("takes the ended ones from the pool's count", () => {
    expect(poolStates(page, 498)).toBe("1 working · 1 asks you · 498 ended");
  });

  it("counts the page while the pool's count isn't known yet", () => {
    expect(poolStates(page, null)).toBe("1 working · 1 asks you · 48 ended");
  });

  it("says nothing ended when none has", () => {
    expect(poolStates([live("Working")], 0)).toBe("1 working");
  });

  it("leaves every ended row to the pool's count, whatever word says how it ended", () => {
    const ended = [...Array.from({ length: 45 }, () => gone("Finished")), gone("Stopped by you"), gone("Crashed"), gone("Error"), gone("Ended"), gone("Finished")];
    expect(poolStates(ended, 500)).toBe("500 ended");
    expect(poolStates([live("Working"), ...ended], 500)).toBe("1 working · 500 ended");
  });
});
