/**
 * D-62 (AJ-E3004): a session's Pools tab says how it got into each pool
 * and why the others left it out, from the resolver's own answer.
 */
import { cleanup, render, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as model from "../../lib/ipc/pool-model";
import { MOCK_SESSIONS } from "../../preview/fixtures";
import { SessionPools, viaWords } from "./SessionPools";
import { useSessionTabs } from "./session-tabs";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("D-62 a session's pools", () => {
  it("says how it got in, for every kind of member", () => {
    expect(viaWords("session")).toMatch(/You put it here/);
    expect(viaWords("app")).toMatch(/Through its App/);
    expect(viaWords("account")).toMatch(/Through its Account/);
    expect(viaWords("everyone")).toMatch(/Every session of its projects/);
  });

  it("lists each pool with its role, then what left it out and why", async () => {
    vi.spyOn(model, "poolsOfSession").mockResolvedValue({
      session_id: MOCK_SESSIONS[0]!.id,
      pools: [
        { pool_id: "p-reviews", pool_name: "Reviews", role: "reviewer", via: "app", member: "app:cli:work", level: "read", sealed: false },
        { pool_id: "pool-default:shop", pool_name: "shop · all sessions", role: "member", via: "everyone", member: "all", level: null, sealed: false },
      ],
      skipped: [{ pool_id: "p-web", pool_name: "Web", reason: "out_of_scope", by_pool: null, by_pool_name: null, words: "«Web» doesn't take its project. Add the project to its scope to let it in." }],
      account_unknown: true,
    });
    render(<SessionPools session={MOCK_SESSIONS[0]!} />);
    await waitFor(() => expect(screen.getByTestId("session-pools-list")).toBeTruthy());
    expect(screen.getAllByTestId("related-pool")).toHaveLength(2);
    expect(screen.getAllByTestId("session-pools-how")[0]!.textContent).toMatch(/Through its App/);
    expect(screen.getByText("its own level: Read")).toBeTruthy();
    expect(screen.getByTestId("session-pools-skipped").textContent).toContain("Add the project to its scope");
    expect(screen.getByTestId("session-pools-account-unknown")).toBeTruthy();
  });

  it("its Pools tab counts the pools the tab lists, from the same read (AJ-E3104 R1)", async () => {
    const session = { ...MOCK_SESSIONS[1]!, id: "s-count-pools" };
    const read = vi.spyOn(model, "poolsOfSession").mockResolvedValue({
      session_id: session.id,
      pools: [
        { pool_id: "p-a", pool_name: "A", role: "member", via: "session", member: `session:${session.id}`, level: null, sealed: false },
        { pool_id: "p-b", pool_name: "B", role: "lead", via: "account", member: "account:work", level: null, sealed: false },
        { pool_id: "p-c", pool_name: "C", role: "member", via: "everyone", member: "all", level: null, sealed: false },
      ],
      skipped: [{ pool_id: "p-web", pool_name: "Web", reason: "out_of_scope", by_pool: null, by_pool_name: null, words: "left out" }],
      account_unknown: false,
    });
    const facts = renderHook(() => useSessionTabs(session));
    // The pools it's in, not the ones that left it out.
    await waitFor(() => expect(facts.result.current.counts).toEqual({ pools: 3 }));
    render(<SessionPools session={session} />);
    await waitFor(() => expect(screen.getAllByTestId("related-pool")).toHaveLength(3));
    expect(read.mock.calls.filter(([id]) => id === session.id)).toHaveLength(1);
  });
});
