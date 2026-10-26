import { describe, expect, it } from "vitest";
import { tokenWords } from "./quotaWindow";
import { formatTokens } from "../components/indicator";
import { shortCount } from "../pages/session/UsageTab";

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

  // 0.1.7: 999,999 read "1000k" on the Prices sheet.
  it("a count that rounds up to the next unit reads in that unit", () => {
    expect(formatTokens(999_999)).toBe("1.0M");
    expect(formatTokens(999_949)).toBe("999.9k");
    expect(formatTokens(999_999_999)).toBe("1.0B");
    expect(tokenWords(999_600)).toBe("1.0M");
    expect(tokenWords(999_499)).toBe("999k");
    expect(tokenWords(9_999_999)).toBe("10M");
    expect(shortCount(999_700)).toBe("1.0M");
    expect(shortCount(99_960)).toBe("100k");
    expect(shortCount(940)).toBe("940");
  });
});
