/**
 * D-62 decision 12 (pools-model §4.2: "One session named on its own can
 * override that"): a session brought in by its Account at Read gets its
 * own level from the canvas inspector, as a session member, so it wins
 * inside the pool (the rc33 walk, P5 step 3, found no control for it).
 */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import * as model from "../../lib/ipc/pool-model";
import type { PoolInfo } from "../../lib/ipc/pools";
import { pick } from "../../test/pick";
import { MemberLevel, keptLevelWords } from "./MemberLevel";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const pool: PoolInfo = { id: "careful", name: "Careful", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 2, scope_all: true, scope: [], sealed: false, skip_held: false };

describe("D-62 a session's own level in a pool", () => {
  it("names a session on its own at the level picked, over its Account's Read", async () => {
    vi.spyOn(model, "poolsOfSession").mockResolvedValue({
      session_id: "p2",
      pools: [{ pool_id: "careful", pool_name: "Careful", role: "member", via: "account", member: "account:personal", level: "read", sealed: false }],
      skipped: [],
      account_unknown: false,
    });
    const join = vi.spyOn(model, "poolJoin").mockResolvedValue({ pools: [], placements: [] });
    render(<MemberLevel pool={pool} sessionId="p2" memberRole="member" />);
    const picker = await screen.findByTestId("inspect-node-level-picker");
    // What it has now: its Account's level, said as such.
    expect(picker.textContent).toContain("As its Account (Read)");
    pick(picker, "Work together");
    await waitFor(() => expect(join).toHaveBeenCalledWith("careful", "session:p2", "member", "work_together"));
  });

  // Another id: the hook keeps one read per session.
  it("a session named on its own changes its own rule, and can go back to the pool's", async () => {
    vi.spyOn(model, "poolsOfSession").mockResolvedValue({
      session_id: "p3",
      pools: [{ pool_id: "careful", pool_name: "Careful", role: "reviewer", via: "session", member: "work:w2", level: "work_together", sealed: false }],
      skipped: [],
      account_unknown: false,
    });
    const join = vi.spyOn(model, "poolJoin").mockResolvedValue({ pools: [], placements: [] });
    render(<MemberLevel pool={pool} sessionId="p3" memberRole="member" />);
    const picker = await screen.findByTestId("inspect-node-level-picker");
    expect(picker.textContent).toContain("Work together");
    pick(picker, "The pool's (Work together)");
    // Its stored ref (a switch keeps its place) and its role there.
    await waitFor(() => expect(join).toHaveBeenCalledWith("careful", "work:w2", "reviewer", null));
  });

  it("says whose level it keeps", () => {
    expect(keptLevelWords("account", "read", "full")).toBe("As its Account (Read)");
    expect(keptLevelWords("app", null, "full")).toBe("As its App (Full)");
    expect(keptLevelWords("session", null, "read")).toBe("The pool's (Read)");
  });
});
