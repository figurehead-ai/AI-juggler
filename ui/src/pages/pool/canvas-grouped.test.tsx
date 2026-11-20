/**
 * F-rc3g-8 (0.4.6 rc3g, the P4 walk): right after Make, a pool of 88
 * sessions (85 long ended, 3 open, all in it through "everyone in web and
 * team") said "Nothing open now" and "Your team is drawn. No one has
 * joined yet." while its header said "3 open in AI Juggler · 88 in all".
 * Not a stale read: past 50 members the canvas drew a broad member's
 * sessions as its card, counting the ended ones it never draws, and Now
 * and the planned team read what the canvas drew. The canvas draws the
 * open ones, Now counts what the header counts, and the planned team is
 * only for a pool no session has joined.
 */
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { refreshPools } from "../../lib/pools";
import { pageOf } from "../../lib/paged";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { ConversationsProvider } from "../../lib/conversations";
import { ActionHostProvider } from "../../components/ItemMenu";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { PoolMemberRow } from "../../lib/ipc/pools";
import { PoolPage } from "../PoolPage";
import { resetCanvasTests, seams } from "./canvas.test-kit";

const POOL = "pool-big";
const big = vi.hoisted((): { rows: PoolMemberRow[]; members: number; open: string[] } => ({ rows: [], members: 0, open: [] }));

// The backend's answers for a big pool: its members a page at a time, its
// count and its open sessions' places.
vi.mock("../../lib/ipc/pools", async (real) => {
  const actual = await real<typeof import("../../lib/ipc/pools")>();
  return {
    ...actual,
    poolMembers: async (...args: Parameters<typeof actual.poolMembers>) => (args[0] === POOL ? pageOf(big.rows, args[1]) : actual.poolMembers(...args)),
    poolsList: async () => {
      const info = await actual.poolsList();
      return {
        pools: info.pools.map((pool) => (pool.id === POOL ? { ...pool, members: big.members } : pool)),
        placements: [...info.placements, ...big.open.map((session_id) => ({ session_id, pool_id: POOL, role: "member" }))],
      };
    },
  };
});

const base: InstanceInfo = MOCK_SESSIONS[0]!;
const session = (id: string, status: string): InstanceInfo => ({ ...base, id, label: id, status, ...(status === "finished" ? { runtime: undefined } : {}) });
const card = (one: InstanceInfo): PoolMemberRow["card"] => ({ session_id: one.id, name: one.id, place: "web", tool: "Claude Code", account: null, state: one.status, role: "member", task: { value: null, unavailable: null }, said: { value: null, unavailable: "it hasn't said" }, last_activity_ms: null, files: [], files_unavailable: null });
const row = (one: InstanceInfo): PoolMemberRow => ({ session: one, card: card(one), via: "everyone", held_by: null });

/** A pool of `open` open and `ended` ended sessions, all through everyone
 * in its projects, its setup hidden (the calm view with Now). */
async function openBig(open: number, ended: number) {
  const opened = Array.from({ length: open }, (_, n) => session(`big-open-${n}`, "running"));
  const gone = Array.from({ length: ended }, (_, n) => session(`big-ended-${n}`, "finished"));
  Object.assign(big, { rows: [...opened, ...gone].map(row), members: open + ended, open: opened.map((one) => one.id) });
  seams().__AIJ_POOLS__ = { pools: [{ id: POOL, name: "A reviewer for everything", rules: [] }], members: [] };
  seams().__AIJ_POOL_RULES__ = { [POOL]: { members: [{ member: "all", kind: "everyone", role: "member", level: null, joined_at: 1, sessions: open, session_ids: [] }], talk: [], lead_next: null } };
  localStorage.setItem(`aij.pool.setup-hidden.${POOL}`, "1");
  await act(async () => {
    await refreshPools();
  });
  render(
    <ConversationsProvider want={null}>
      <ActionHostProvider host={{}}>
        <PoolPage poolId={POOL} sessions={opened} projects={MOCK_PROJECTS} accounts={[]} now={Date.now()} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
  await waitFor(() => expect(screen.getByTestId("pool-page-count").textContent).toContain(`${open + ended}`));
}

const now = () => screen.getByTestId("pool-running").querySelector("[data-fact='now'] dd")?.textContent ?? "";

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

describe("a big pool's canvas and Now agree with its header (F-rc3g-8)", () => {
  it("88 sessions, 85 ended: its 3 open ones are drawn and Now counts them; no planned team", async () => {
    await openBig(3, 85);
    expect(screen.getByTestId("pool-page-count").textContent).toBe("3 open in AI Juggler · 88 in all");
    await waitFor(() => expect(screen.getAllByTestId("canvas-node")).toHaveLength(3));
    expect(screen.queryByTestId("planned-team")).toBeNull();
    expect(screen.queryByText("Your team is drawn. No one has joined yet.")).toBeNull();
    await waitFor(() => expect(now()).toMatch(/^3 /));
  });

  it("88 sessions, none open: it says they show as they start, never that no one has joined", async () => {
    await openBig(0, 88);
    await waitFor(() => expect(screen.getByTestId("canvas-rules-only").textContent).toContain("Its sessions show here as they start"));
    expect(screen.queryByTestId("planned-team")).toBeNull();
    expect(now()).toBe("Nothing open now");
  });

  it("past 50 open, each member's are its card: the canvas and Now say how many are open", async () => {
    await openBig(51, 10);
    await waitFor(() => expect(screen.getByTestId("canvas-rules-only").textContent).toContain("51 open now, each on its member's card"));
    expect(screen.queryAllByTestId("canvas-node")).toHaveLength(0);
    expect(screen.queryByTestId("planned-team")).toBeNull();
    expect(now()).toMatch(/^51 /);
  });
});
