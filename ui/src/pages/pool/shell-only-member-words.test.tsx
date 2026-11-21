/**
 * SK48-4 (the 0.4.8 soak, P8: a Kimi Code member, Shell only): its
 * inspector said "Messages: it reads messages on its next turn." under
 * "it can't read the pool, post or take a task"; its List row's reason
 * was cut ("…can't read the pool, p…"); its session page's Account was cut
 * ("Account unknown · this Provider exp…"). What it can't do is said
 * whole, and its messages line says what's true.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PoolCardRows } from "../../components/PoolCards";
import * as model from "../../lib/ipc/pool-model";
import type { ContextCard, PoolInfo, PoolMemberRow } from "../../lib/ipc/pools";
import type { PagedList } from "../../lib/paged";
import { instance } from "../../test/fixtures";
import { WhereItRuns } from "../session/WhereItRuns";
import { fromInstance } from "../session/model";
import { MemberLevel } from "./MemberLevel";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const pool: PoolInfo = { id: "scouts", name: "Scouts", kind: "user", project_id: null, access: { level: "work_together", actions: [], hop_limit: 5, delivery_stopped: false }, rules: [], spans_macs: false, scopes: 1, members: 1, scope_all: true, scope: [], sealed: false, skip_held: false };
const KIMI = instance({ id: "s-kimi", provider: "kimi-code", account_id: "kimi", account_label: "default", project_id: "research", cwd: "/Users/you/lab/research" });
const CLAUDE = instance({ id: "s-claude", provider: "claude-code", account_id: "work", project_id: "research", cwd: "/Users/you/lab/research" });
const placed = (sessionId: string) =>
  vi.spyOn(model, "poolsOfSession").mockResolvedValue({ session_id: sessionId, pools: [{ pool_id: "scouts", pool_name: "Scouts", role: "member", via: "everyone", member: "everyone", level: null, sealed: false }], skipped: [], account_unknown: false });

describe("a member that works through its shell (SK48-4)", () => {
  it("its messages line says it can't read them, never that it reads them on its next turn", async () => {
    placed("s-kimi");
    render(<MemberLevel pool={pool} sessionId="s-kimi" memberRole="member" session={KIMI} />);
    expect((await screen.findByTestId("inspect-node-wake")).textContent).toBe("Messages: it can't read the pool's messages: it works through its shell.");
  });

  it("one with the agent tools still says how it hears them", async () => {
    placed("s-claude");
    render(<MemberLevel pool={pool} sessionId="s-claude" memberRole="member" session={CLAUDE} />);
    expect((await screen.findByTestId("inspect-node-wake")).textContent).toBe("Messages: it reads messages on its next turn.");
  });

  it("its List row says why whole, never cut", () => {
    const card: ContextCard = { session_id: KIMI.id, name: "research", place: "research · default", tool: "Kimi Code", account: "default", state: "running", role: "member", task: { value: "None", unavailable: null }, said: { value: null, unavailable: "Kimi Code doesn't report what it's doing" }, last_activity_ms: null, files: [], files_unavailable: null };
    const row: PoolMemberRow = { session: KIMI, card, via: "everyone", held_by: null };
    const list: PagedList<PoolMemberRow> = { items: [row], total: 1, loading: false, stale: false, loadingMore: false, hasMore: false, error: null, loadMore: () => {}, refresh: () => {} };
    render(<PoolCardRows list={list} sessions={[KIMI]} now={Date.now()} />);
    const why = screen.getByTestId("pool-card-shell-why");
    expect(why.textContent).toMatch(/can't read the pool, post or take a task/);
    expect(why.className).not.toContain("truncate");
  });

  it("a session page's Account words wrap whole; only a folder's path gives way", () => {
    const words = "Account unknown · Kimi Code doesn't say which account it's signed in to";
    render(<WhereItRuns model={fromInstance(KIMI)} accountWords={words} outside={false} />);
    const account = screen.getByTestId("where-account-value");
    expect(account.textContent).toBe(words);
    expect(account.className).not.toContain("truncate");
    expect(screen.getByTestId("where-folder-value").className).toContain("truncate");
  });
});
