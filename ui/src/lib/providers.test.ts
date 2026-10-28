import { describe, expect, it } from "vitest";
import { inPlaceProviders, installWords, ownSignIn, providerDisplayName, sameAsWords, supportsInPlace, toolFound, versionNotAsked } from "./providers";

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
    ]);
    expect(supportsInPlace(ids, "codex")).toBe(true);
    expect(supportsInPlace(ids, "zcode")).toBe(false);
    // The call failed: every tool keeps the choice.
    expect(supportsInPlace("unknown", "zcode")).toBe(true);
  });

  it("says what 'the same' is the same as, per tool", () => {
    expect(sameAsWords(["claude-code"])).toBe("as your Claude app");
    expect(sameAsWords(["codex", "codex"])).toBe("as when you run Codex yourself");
    expect(sameAsWords(["claude-code", "codex"])).toBe("as wherever you use them now");
  });

  it("F19: a tool found from its files, its version not asked, is installed with its version not known", () => {
    const row = { provider: "codex", display_name: "Codex", install_url: "", status: "found", version: null, binary: "/usr/local/bin/codex", notes: [], verified_against: [], kind: "terminal" };
    expect(toolFound(row)).toBe(true);
    expect(versionNotAsked(row)).toBe(true);
    const words = installWords(row as never);
    expect(words).toContain("version isn't known");
    expect(words).not.toMatch(/couldn't check|not installed/i);
  });
});
