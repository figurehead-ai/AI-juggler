import { describe, expect, it } from "vitest";
import { inPlaceProviders, ownSignIn, providerDisplayName, sameAsWords } from "./providers";

describe("providers — copy helpers", () => {
  it("names tools by id, unknown ones as themselves", () => {
    expect(providerDisplayName("codex")).toBe("Codex");
    expect(providerDisplayName("mystery")).toBe("mystery");
  });

  it("names Codex's vendor once in its sign-in", () => {
    expect(ownSignIn("codex")).toBe("Codex's own sign-in (OpenAI)");
    expect(ownSignIn("claude-code")).toBe("Claude Code's own sign-in");
  });

  it("ADR-0018: the tools that can be used where they are come from the backend's rows", () => {
    const ids = inPlaceProviders([
      { provider: "claude-code", can_use_in_place: true },
      { provider: "codex", can_use_in_place: true },
      { provider: "zcode", can_use_in_place: false },
      { provider: "old-backend" },
    ]);
    expect([...ids].sort()).toEqual(["claude-code", "codex"]);
  });

  it("says what 'the same' is the same as, per tool", () => {
    expect(sameAsWords(["claude-code"])).toBe("as your Claude app");
    expect(sameAsWords(["codex", "codex"])).toBe("as when you run Codex yourself");
    expect(sameAsWords(["claude-code", "codex"])).toBe("as wherever you use them now");
  });
});
