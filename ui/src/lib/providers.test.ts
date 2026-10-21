import { describe, expect, it } from "vitest";
import { canUseInPlace, ownSignIn, providerDisplayName, sameAsWords } from "./providers";

describe("providers — copy helpers", () => {
  it("names tools by id, unknown ones as themselves", () => {
    expect(providerDisplayName("codex")).toBe("Codex");
    expect(providerDisplayName("mystery")).toBe("mystery");
  });

  it("names Codex's vendor once in its sign-in", () => {
    expect(ownSignIn("codex")).toBe("Codex's own sign-in (OpenAI)");
    expect(ownSignIn("claude-code")).toBe("Claude Code's own sign-in");
  });

  it("ADR-0018: Claude Code and Codex setups can be used where they are; others are copied", () => {
    expect(canUseInPlace("claude-code")).toBe(true);
    expect(canUseInPlace("codex")).toBe(true);
    expect(canUseInPlace("zcode")).toBe(false);
  });

  it("says what 'the same' is the same as, per tool", () => {
    expect(sameAsWords(["claude-code"])).toBe("as your Claude app");
    expect(sameAsWords(["codex", "codex"])).toBe("as when you run Codex yourself");
    expect(sameAsWords(["claude-code", "codex"])).toBe("as wherever you use them now");
  });
});
