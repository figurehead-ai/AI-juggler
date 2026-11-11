import { afterEach, describe, expect, it, vi } from "vitest";

/** 0.3.5: the switch is told where it continues, Desktop or CLI. */
const invoke = vi.fn(async () => ({ session_id: "s-2", pid: 1, edge_id: "e-1", source_session_id: "s-1", continued_in: "desktop", conversation_key: "conversation:a:b" }));
vi.mock("./transport", () => ({ call: (...args: unknown[]) => invoke(...(args as [])) }));
vi.mock("../bridge", () => ({ hasLiveBackend: () => true }));

afterEach(() => invoke.mockClear());

describe("switchExecute", () => {
  it("sends the destination with the switch", async () => {
    const { switchExecute } = await import("./switchExecute");
    const result = await switchExecute("s-1", "r-1", "finish-turn", null, [], ["notes"], "desktop");
    expect(invoke).toHaveBeenCalledWith("switch_execute", expect.objectContaining({ sessionId: "s-1", targetProfileId: "r-1", approved: ["notes"], destination: "desktop" }));
    expect(result.conversation_key).toBe("conversation:a:b");
  });

  it("sends CLI when no destination is given", async () => {
    const { switchExecute } = await import("./switchExecute");
    await switchExecute("s-1", "r-1", "finish-turn");
    expect(invoke).toHaveBeenCalledWith("switch_execute", expect.objectContaining({ destination: "cli" }));
  });
});
