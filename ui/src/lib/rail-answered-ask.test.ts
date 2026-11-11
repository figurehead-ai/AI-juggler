/**
 * The rc29 walk: an answered ask read "Asks you" about 1.5 min beside the
 * session's own Idle (its tool's word). The tool's word that it doesn't
 * ask wins, as Needs you reads it (`notifications::needs::kind_of`).
 */
import { describe, expect, it } from "vitest";
import { sessionItem } from "./rail-items";
import { MOCK_SESSIONS } from "../preview/fixtures";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const base = MOCK_SESSIONS[0]!;
const row = (state: string | null): InstanceInfo => ({
  ...base,
  status: "waiting_user",
  needs_attention: "waiting_user",
  attention: { rule: "tool_asks", inputs: ["Bash: ls"], since_ms: 1 },
  runtime: state === null ? undefined : { kind: "terminal", label: null, beta: false, state, asks: [], can_steer: false, fell_back: null, error: null, limits: [], cut_off: null },
});

describe("an answered ask", () => {
  it("needs nothing once its tool says it's idle or working", () => {
    expect(sessionItem(row("idle"), () => null, false).need).toBeNull();
    expect(sessionItem(row("working"), () => null, false).need).toBeNull();
  });
  it("still asks while its tool says it asks, or says nothing", () => {
    expect(sessionItem(row("asks_you"), () => null, false).need).toBe("asks");
    expect(sessionItem(row(null), () => null, false).need).toBe("asks");
  });
});
