/**
 * D-126 §4d (the X2 walk: a Pools card's thumbnail was a near-empty
 * dotted box): a pool's card draws its team — who leads on top, the
 * members as their tools' logos with their open sessions counted.
 */
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { ActionHostProvider } from "../../components/ItemMenu";
import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import type { PoolRulesInfo } from "../../lib/ipc/pool-model";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools } from "../../lib/pools";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { miniTeam, MINI_ROW, PoolMini } from "./PoolMini";
import { PoolsScreen } from "./PoolsScreen";

const NO_SOURCES = { apps: [], sessions: [] };
const rule = (member: string, role: string, sessions: number) => ({ member, kind: member === "all" ? "everyone" : member.split(":")[0]!, role, level: null, joined_at: 1, sessions, session_ids: [] });

beforeEach(() => {
  resetPreviewPools();
  resetPreviewPoolModel();
  resetPools();
});
afterEach(cleanup);

describe("a Pools card's team in small", () => {
  it("from its members: the App whose next session leads on top, the rest with their open sessions counted", () => {
    const rules: PoolRulesInfo = { pool_id: "p", members: [rule("app:cli:work", "member", 0), rule("app:cli:codex", "reviewer", 2), rule("all", "member", 3)], talk: [], lead_next: "cli:work" };
    const team = miniTeam(rules, [], () => "member", NO_SOURCES);
    expect(team.lead?.ref).toBe("app:cli:work");
    expect(team.members.map((one) => [one.ref, one.role, one.count])).toEqual([
      ["app:cli:codex", "reviewer", 2],
      ["all", "member", 3],
    ]);
  });

  it("what everyone brings in now is drawn as those sessions' tools, alike ones together, beside the named members", () => {
    const base = MOCK_SESSIONS[0]!;
    const open: InstanceInfo[] = ["a", "b", "c"].map((id) => ({ ...base, id, provider: "claude-code", app_id: null }));
    const rules: PoolRulesInfo = { pool_id: "p", members: [{ ...rule("session:c", "reviewer", 1), session_ids: ["c"] }, rule("all", "member", 2)], talk: [], lead_next: null };
    const team = miniTeam(rules, open, () => "member", NO_SOURCES);
    expect(team.members.map((one) => [one.ref, one.role, one.count])).toEqual([
      ["session:c", "reviewer", 1],
      ["session:a", "member", 2],
    ]);
  });

  it("before its members are read, from its open sessions, alike tools together, the lead apart", () => {
    const base = MOCK_SESSIONS[0]!;
    const open: InstanceInfo[] = ["a", "b", "c"].map((id) => ({ ...base, id, provider: "claude-code", app_id: null }));
    const team = miniTeam(null, open, (id) => (id === "a" ? "lead" : "member"), NO_SOURCES);
    expect(team.lead?.ref).toBe("session:a");
    expect(team.members.map((one) => one.count)).toEqual([2]);
  });

  it("draws the lead on top with its crown, the members with counts, past four a +N; an empty one says so", () => {
    const tile = { kind: "tile" as const, icon: () => <svg /> };
    const members = ["a", "b", "c", "d", "e"].map((ref, at) => ({ ref, logo: tile, role: "member" as const, count: at + 1 }));
    const view = render(<PoolMini lead={{ ref: "l", logo: tile, role: "lead", count: 1 }} members={members} />);
    expect(screen.getByTestId("pool-mini").dataset.lead).toBe("l");
    expect(screen.getAllByTestId("pool-mini-member")).toHaveLength(MINI_ROW - 1);
    expect(screen.getByTestId("pool-mini-more").textContent).toBe("+2");
    expect(within(screen.getAllByTestId("pool-mini-member")[1]!).getByText("2")).toBeTruthy();
    view.rerender(<PoolMini lead={null} members={[]} />);
    expect(screen.getByTestId("pool-mini").textContent).toBe("No one in it yet");
    view.rerender(<PoolMini lead={null} members={[]} known={false} />);
    expect(screen.getByTestId("pool-mini").textContent).toBe("");
  });

  it("on the Pools screen, a card shows its team once read", async () => {
    Object.assign(window, { __AIJ_POOLS__: { pools: [{ id: "pool-team", name: "Team", rules: [] }], members: [{ session_id: MOCK_SESSIONS[0]!.id, pool_id: "pool-team", role: "lead" }] } });
    await act(async () => {
      await refreshPools();
    });
    render(
      <ActionHostProvider host={{}}>
        <PoolsScreen sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={[]} />
      </ActionHostProvider>,
    );
    const card = (await screen.findAllByTestId("pool-card-tile")).find((tile) => tile.dataset.pool === "pool-team")!;
    await waitFor(() => expect(within(card).getByTestId("pool-mini")).toBeTruthy());
    Reflect.deleteProperty(window, "__AIJ_POOLS__");
  });
});
