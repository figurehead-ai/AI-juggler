/**
 * AJ-E3014 item 9 (P7's rc3a follow-ups, re-checked on the beta): once
 * the "Client" pool was sealed, it held a session out of "web". web's
 * canvas then showed a lone "No lead for now" card, its setup panel said
 * "Who's in: No one yet" and its header "no session yet", beside the List
 * view's "in Client (sealed)" row. The page counts the held member now,
 * and says a sealed pool holds it. And a Pools card draws no held session
 * beside "0 sessions", and an App member reads as one, not as a session.
 */
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { heldOut, refreshPools } from "../../lib/pools";
import { pageOf } from "../../lib/paged";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import type { PoolMemberRow } from "../../lib/ipc/pools";
import type { PoolRulesInfo } from "../../lib/ipc/pool-model";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { miniTeam, PoolMini, withoutHeld } from "../../screens/pools/PoolMini";
import { PoolPage } from "../PoolPage";
import { resetCanvasTests, seams } from "./canvas.test-kit";

const POOL = "pool-web";
const kept = vi.hoisted((): { rows: PoolMemberRow[] } => ({ rows: [] }));

// The backend's member rows for web: one session, held by Client.
vi.mock("../../lib/ipc/pools", async (real) => {
  const actual = await real<typeof import("../../lib/ipc/pools")>();
  return { ...actual, poolMembers: async (...args: Parameters<typeof actual.poolMembers>) => (args[0] === POOL ? pageOf(kept.rows, args[1]) : actual.poolMembers(...args)) };
});

const client: InstanceInfo = { ...MOCK_SESSIONS[0]!, id: "client-1", label: "client · started 11:08 AM", status: "running" };
const card: PoolMemberRow["card"] = { session_id: client.id, name: "client", place: "client", tool: "Claude Code", account: null, state: "running", role: "member", task: { value: null, unavailable: null }, said: { value: null, unavailable: "it hasn't said" }, last_activity_ms: null, files: [], files_unavailable: null };

beforeEach(() => {
  resetCanvasTests();
  resetPreviewPoolModel();
  localStorage.clear();
});
afterEach(() => {
  cleanup();
  delete seams().__AIJ_POOL_RULES__;
  resetPreviewPoolModel();
  localStorage.clear();
});

describe("a pool whose session a sealed pool holds says so", () => {
  it("its header counts it, its canvas says where it is (no lone No lead), its setup's Who's in names it", async () => {
    kept.rows = [{ session: client, card, via: "session", held_by: "Client" }];
    seams().__AIJ_POOLS__ = { pools: [{ id: POOL, name: "web", rules: [] }], members: [] };
    seams().__AIJ_POOL_RULES__ = { [POOL]: { members: [{ member: `session:${client.id}`, kind: "session", role: "member", level: null, joined_at: 1, sessions: 0, session_ids: [] }], talk: [], lead_next: null } };
    await act(async () => {
      await refreshPools();
    });
    render(
      <ConversationsProvider want={null}>
        <ActionHostProvider host={{}}>
          <PoolPage poolId={POOL} sessions={[client]} projects={MOCK_PROJECTS} accounts={[]} now={Date.now()} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("pool-page-count").textContent).toContain("1 session in Client (sealed)"));
    expect((await screen.findByTestId("canvas-rules-only")).textContent).toContain("client · started 11:08 AM is in Client (sealed) for now");
    expect(screen.queryByTestId("canvas-lead-card")).toBeNull();
    const who = screen.getAllByTestId("pool-checklist-value").map((value) => value.textContent ?? "");
    expect(who).toContain("client (in Client, sealed)");
    expect(who).not.toContain("No one yet");
  });
});

describe("a Pools card", () => {
  const rule = (member: string, kind: string, sessions = 0) => ({ member, kind, role: "member", level: null, joined_at: 1, sessions, session_ids: [] });

  it("draws no session a sealed pool holds out of it", () => {
    const pools = {
      pools: [
        { id: POOL, sealed: false },
        { id: "pool-client", sealed: true },
      ],
      placements: [{ session_id: client.id, pool_id: "pool-client" }],
    };
    expect(heldOut(pools, client.id, POOL)).toBe(true);
    expect(heldOut(pools, client.id, "pool-client")).toBe(false);
    const rules: PoolRulesInfo = { pool_id: POOL, members: [rule(`session:${client.id}`, "session")], talk: [], lead_next: null };
    const team = miniTeam(withoutHeld(rules, (id) => heldOut(pools, id, POOL)), [], () => "member", { apps: [], sessions: [] });
    expect(team.members).toHaveLength(0);
    expect(team.lead).toBeNull();
  });

  it("an App member reads as one: its kind on its card, never as a third session", () => {
    const rules: PoolRulesInfo = { pool_id: "pool-client", members: [rule("app:cli:work", "app")], talk: [], lead_next: null };
    const team = miniTeam(rules, [], () => "member", { apps: [], sessions: [] }, () => "Claude Code · Work");
    render(<PoolMini lead={team.lead} members={team.members} />);
    expect(within(screen.getByTestId("pool-mini")).getByTestId("pool-mini-card").textContent).toBe("Claude Code · WorkApp · Member");
  });
});
