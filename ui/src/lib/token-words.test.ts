import { describe, expect, it } from "vitest";
import { tokenWords } from "./quotaWindow";
import { formatTokens } from "../components/indicator";

// 0.1.1's runway said "10329M tokens used" for a busy Claude account.
describe("token counts in words", () => {
  it("the runway's words go up to billions", () => {
    expect(tokenWords(950)).toBe("950");
    expect(tokenWords(12_345)).toBe("12k");
    expect(tokenWords(1_234_567)).toBe("1.2M");
    expect(tokenWords(812_000_000)).toBe("812M");
    expect(tokenWords(10_329_000_000)).toBe("10.3B");
    expect(tokenWords(123_400_000_000)).toBe("123B");
  });

  it("a session's count too", () => {
    expect(formatTokens(18_200)).toBe("18.2k");
    expect(formatTokens(2_802_000_000)).toBe("2.8B");
  });
});
