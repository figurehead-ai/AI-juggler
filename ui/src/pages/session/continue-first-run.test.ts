/**
 * 0.3.5: a Claude app profile's folder never ran Claude Code's terminal
 * first run; 0.3.4 held every such target back. Now the row offers it and
 * says the switch finishes that step.
 */
import { describe, expect, it } from "vitest";
import { targetsOf, withFirstRun } from "./ContinueOn";
import type { SwitchTargetData } from "../../lib/ipc/switch-targets";

describe("0.3.5 a target whose tool hasn't done its first run", () => {
  it("is offered, and its line says the switch does it", () => {
    const row = { recipe_id: "r", account_id: "west", account_label: "acme-west", filtered_reason: null, held_back: null, first_run: true, badge: "", validated: true, cross_provider: false } as unknown as SwitchTargetData;
    const [target] = targetsOf([row], "north");
    expect(target!.blocked).toBeNull();
    expect(target!.setupNeeded).toBe(false);
    expect(withFirstRun(null, target!.firstRun === true)).toBe("Its first-run setup is done as it switches");
    expect(withFirstRun("Needs 1 thing, set up as it switches", true)).toBe("Needs 1 thing, set up as it switches · its first-run setup too");
    expect(withFirstRun("Ready: same tools set up", false)).toBe("Ready: same tools set up");
  });
});
