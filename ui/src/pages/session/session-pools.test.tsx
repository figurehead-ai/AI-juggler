/**
 * D-62 (AJ-E3004): a session's Pools tab says how it got into each pool
 * and why the others left it out, from the resolver's own answer.
 */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as model from "../../lib/ipc/pool-model";
import { MOCK_SESSIONS } from "../../preview/fixtures";
import { SessionPools, viaWords } from "./SessionPools";

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
});
