/**
 * D-117 (AJ-E3601 R5): the classic look is a hidden flag, on only when
 * asked for in a 0.5 build (or Dev, or the design preview), never in 0.4
 * or 0.6, and never by default.
 */
import { afterEach, describe, expect, it } from "vitest";
import { applyAttentionMode, classicAllowed, CLASSIC_KEY, wantsClassic } from "./attention-classic";

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.attention;
});

describe("the classic attention flag", () => {
  it("is allowed in a 0.5 build, Dev and the preview only", () => {
    expect(classicAllowed("0.5.0", "main")).toBe(true);
    expect(classicAllowed("0.5.3", "nightly")).toBe(true);
    expect(classicAllowed("0.4.6", "main")).toBe(false);
    expect(classicAllowed("0.6.0", "main")).toBe(false);
    expect(classicAllowed("0.4.6", "dev")).toBe(true);
    expect(classicAllowed("0.0.0-preview", "main")).toBe(true);
  });

  it("is off unless asked for, and only then marks the page", async () => {
    expect(wantsClassic()).toBe(false);
    expect(await applyAttentionMode({ version: "0.5.0", edition: "main" })).toBe(false);
    expect(document.documentElement.dataset.attention).toBeUndefined();
    localStorage.setItem(CLASSIC_KEY, "classic");
    expect(await applyAttentionMode({ version: "0.4.6", edition: "main" })).toBe(false);
    expect(await applyAttentionMode({ version: "0.5.0", edition: "main" })).toBe(true);
    expect(document.documentElement.dataset.attention).toBe("classic");
  });
});
