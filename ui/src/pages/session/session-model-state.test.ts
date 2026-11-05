/**
 * The P4 walk: a driven Codex session between turns showed "Working" in
 * its page header beside "Idle" on its rail row. The page says the rail's
 * one vocabulary (AJ-E2104).
 */
import { describe, expect, it } from "vitest";
import { MOCK_SESSIONS } from "../../lib/bridge";
import { fromInstance } from "./model";

describe("a session page's state", () => {
  it("is its rail row's: idle between turns, working in one", () => {
    const base = { ...MOCK_SESSIONS[0]!, status: "running", needs_attention: null };
    const idle = fromInstance({ ...base, runtime: { state: "idle" } } as never);
    expect(idle.state.words).toBe("Idle");
    expect(idle.state.working).toBe(false);
    const working = fromInstance({ ...base, runtime: { state: "working" } } as never);
    expect(working.state.words).toBe("Working");
    expect(working.state.working).toBe(true);
  });
});
