/**
 * AJ-E3008 R4 and D-95: a line's talk rule, and the one thing it can't
 * hold — Claude's own messages between sessions.
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("../../lib/ipc/pool-model", () => ({
  poolEdgeTalk: vi.fn(async () => ({ rule: "ask", pair_rule: "ask", own: false, from_role: "member", to_role: "member" })),
  poolSetTalk: vi.fn(async () => undefined),
}));

import { EdgeTalk } from "./EdgeTalk";

describe("EdgeTalk", () => {
  it("says its rule follows the pair, and that Claude's own messages aren't held by it (D-95)", async () => {
    render(<EdgeTalk poolId="p1" from="s1" to="s2" fromName="checkout" toName="review" />);
    expect((await screen.findByTestId("edge-talk")).textContent).toContain("The role pair Member → Member says Ask first.");
    expect(screen.getByTestId("edge-talk-claude").textContent).toBe("Claude's own messages between sessions aren't held by this.");
  });
});
