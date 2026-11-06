/**
 * The carry summary's words (0.3.5, D-76): the conversation's cut is said
 * with the items, so "Everything goes along" never shows while turns stay
 * behind (the 0.4 design review).
 */
import { describe, expect, it } from "vitest";
import { carryWords } from "./CarrySummary";
import type { CarryItemData } from "../../lib/ipc/switch-preflight";

const item = (key: string): CarryItemData => ({ key, how: "copied" }) as unknown as CarryItemData;

describe("carryWords", () => {
  it("says everything goes along only when nothing stays behind", () => {
    expect(carryWords([item("a"), item("b")], null, "Personal", "Claude Code")).toEqual({ words: "Everything goes along · 2 items", warn: false });
  });

  it("says the turns a cut leaves out, as a warning", () => {
    expect(carryWords([item("a"), item("b")], null, "Personal", "Claude Code", undefined, 800)).toEqual({ words: "2 go along · the first 800 turns stay behind", warn: true });
    expect(carryWords([item("a")], null, "Personal", "Claude Code", undefined, 1).words).toBe("1 goes along · the first 1 turn stays behind");
  });
});
