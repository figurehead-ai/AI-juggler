/**
 * AJ-E2314 L4 (the 0.5.0 re-walk, DW1): with Always chosen, the new
 * watch's preview said "Ends at midnight tonight." while its headline said
 * "Until you remove it". Each lifetime's preview line says what it means.
 */
import { describe, expect, it } from "vitest";
import { previewFacts, type FormState } from "./watch-form-state";
import { PREVIEW } from "./watches-words";

const state = (over: Partial<FormState>): FormState => ({ targets: [], matching: "each", codes: [], params: {}, life: null, untilDay: null, untilTime: null, quiet: false, ...over });
const ends = (over: Partial<FormState>) => previewFacts({ state: state(over), offered: [], echo: null, was: null, book: new Map(), quiet: null, now: Date.now() }).ends;

describe("the new watch's preview says when it ends", () => {
  it("Always keeps watching until removed; Today ends tonight", () => {
    expect(ends({ life: "always" })).toBe(PREVIEW.alwaysEnds);
    expect(ends({ life: "today" })).toBe(PREVIEW.todayEnds);
    expect(ends({ life: "once" })).toBe(PREVIEW.onceEnds);
  });

  it("Until… with no time picked yet ends tonight, as it would be saved", () => {
    expect(ends({ life: "until" })).toBe(PREVIEW.todayEnds);
  });
});
