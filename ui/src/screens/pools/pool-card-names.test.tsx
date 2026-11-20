/**
 * SK-9 (the soak's P5): a Pools card named an Account member "An Account"
 * where the pool's canvas named it "Personal": the card looked up only
 * sessions and Apps. It names every member as the canvas does now, by the
 * one label the canvas uses (`useMemberLabel`).
 */
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ActionHostProvider } from "../../components/ItemMenu";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools } from "../../lib/pools";
import { PoolsScreen } from "./PoolsScreen";

// The canvas's names, as its label gives them.
vi.mock("../../pages/pool/pool-page-reads", () => ({
  useMemberLabel: () => (member: string) => (member === "account:acct-personal" ? "Personal" : "Someone else"),
}));

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_POOLS__");
  Reflect.deleteProperty(window, "__AIJ_POOL_RULES__");
  resetPreviewPools();
  resetPreviewPoolModel();
  resetPools();
});

describe("a Pools card names its members as the canvas does", () => {
  it("an Account member by its name, never \"An Account\"", async () => {
    Object.assign(window, {
      __AIJ_POOLS__: { pools: [{ id: "pool-careful", name: "Careful web", rules: [] }] },
      __AIJ_POOL_RULES__: { "pool-careful": { members: [{ member: "account:acct-personal", kind: "account", role: "member", level: null, joined_at: 1, sessions: 0, session_ids: [] }], talk: [], lead_next: null } },
    });
    resetPreviewPools();
    resetPreviewPoolModel();
    await act(async () => {
      await refreshPools();
    });
    render(
      <ActionHostProvider host={{}}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    const card = (await screen.findAllByTestId("pool-card-tile")).find((tile) => tile.dataset.pool === "pool-careful")!;
    await waitFor(() => expect(within(card).getByTestId("pool-mini-card").textContent).toBe("PersonalAccount · Member"));
    expect(card.textContent).not.toContain("An Account");
  });
});
