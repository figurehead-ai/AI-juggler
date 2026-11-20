/**
 * D-126: New pool's own words — the steps' questions and groups, and the
 * values only the wizard has (How to start's, Team tools'); the setup's
 * six values are the shared ones (pool-values.test.ts).
 */
import { describe, expect, it } from "vitest";

import type { PoolShape } from "./pool-sentences";
import { eyebrowOf, stepValue } from "./pool-wizard-words";

const shape: PoolShape = {
  members: [
    { ref: "app:claude", label: "Claude · Work", role: "member", level: null, later: true },
    { ref: "app:codex", label: "Codex · Personal", role: "reviewer", level: null, later: true },
    { ref: "all", label: "Everyone in web", role: "member", level: null, later: false },
  ],
  leadNext: "app:claude",
  talk: {},
  level: "work_together",
  sealed: false,
};

describe("the wizard's words (D-126)", () => {
  it("each step says where it is: its number of all and its group", () => {
    const steps = ["template", "where", "who", "lead", "everyone", "talk", "own", "tools", "review"] as const;
    expect(eyebrowOf(steps, "who")).toBe("Step 3 of 9 · The basics");
    expect(eyebrowOf(steps, "talk")).toBe("Step 6 of 9 · How they work");
    expect(eyebrowOf(steps, "review")).toBe("Step 9 of 9 · Finish");
  });

  it("each step's value under its name: the wizard's own two, the setup's six shared, none for Review", () => {
    const draft = { shape, where: "web", template: "From scratch", tools: "Asked once" };
    expect(stepValue("template", draft)).toBe("From scratch");
    expect(stepValue("tools", draft)).toBe("Asked once");
    expect(stepValue("review", draft)).toBeNull();
    expect(stepValue("who", draft)).toBe("Claude, Codex, everyone in web");
    expect(stepValue("lead", draft)).toBe("Claude's next session");
    expect(stepValue("everyone", draft)).toBe("Codex checks the work");
    expect(stepValue("talk", draft)).toBe("Everyone, freely");
    expect(stepValue("own", draft)).toBe("Work together");
  });
});
