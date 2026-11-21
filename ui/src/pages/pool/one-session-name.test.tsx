/**
 * SK48-3 (the 0.4.8 soak, lab P7): three untitled sessions in client,
 * started the same minute, read "client · started 8:25 AM (1)/(2)/(3)" in
 * the rail, the seal's ask and Who's in, but the one also in web read
 * "client · started 8:25 AM" in web's List view and its canvas line: each
 * surface told names apart within its own few. Now one function decides
 * (`sessionNames`, over every session the app holds) and every surface
 * reads it.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PoolCardRows } from "../../components/PoolCards";
import type { PagedList } from "../../lib/paged";
import type { ContextCard, PoolMemberRow } from "../../lib/ipc/pools";
import { sessionNames } from "../../lib/session-titles";
import { instance } from "../../test/fixtures";
import { heldOf } from "./pool-count";
import { drawnMembers } from "./useCanvas";

afterEach(cleanup);

const AT = new Date(2026, 10, 21, 8, 25).getTime();
const client = (id: string, ms: number) => instance({ id, label: undefined, title: undefined, git_branch: null, cwd: "/Users/you/lab/client", project_id: "client", status: "running", started_at_ms: ms });
const C1 = client("c1", AT);
const C2 = client("c2", AT + 10_000);
const C3 = client("c3", AT + 20_000);
const EVERY = [C1, C2, C3];
const card = (id: string): ContextCard => ({ session_id: id, name: "client", place: "client · Work", tool: "Claude Code", account: "Work", state: "running", role: "member", task: { value: "None", unavailable: null }, said: { value: null, unavailable: null }, last_activity_ms: null, files: [], files_unavailable: null });
// web's pool holds only c1 (the other two are only in Client).
const row = (session: typeof C1, held_by: string | null = null): PoolMemberRow => ({ session, card: card(session.id), via: "everyone", held_by });
const listOf = (rows: PoolMemberRow[]): PagedList<PoolMemberRow> => ({ items: rows, total: rows.length, loading: false, stale: false, loadingMore: false, hasMore: false, error: null, loadMore: () => {}, refresh: () => {} });

describe("one name for a session, on every surface (SK48-3)", () => {
  it("is decided once, over every session the app holds", () => {
    const names = sessionNames(EVERY);
    expect([...names.values()].map((name) => name.replace(/^client · started .+ \((\d)\)$/, "client ($1)"))).toEqual(["client (1)", "client (2)", "client (3)"]);
    // Asked again for the same list, the same answer.
    expect(sessionNames(EVERY)).toBe(names);
  });

  it("a pool holding one of them names it as the rail does: its canvas, its sealed line and its List view", () => {
    const names = sessionNames(EVERY);
    const one = names.get("c1")!;
    expect(one).toMatch(/ \(1\)$/);
    // The canvas's member (its inspector, the seal's ask, its lines).
    expect(drawnMembers([row(C1)], new Map(EVERY.map((session) => [session.id, session])), 0, names)[0]!.name).toBe(one);
    // The line a sealed pool's hold says ("… is in Client (sealed) for now").
    expect(heldOf([row(C1, "Client")], names)[0]!.name).toBe(one);
    // The List view's row.
    render(<PoolCardRows list={listOf([row(C1)])} sessions={EVERY} now={AT + 60_000} />);
    expect(screen.getByTestId("pool-card-name").textContent).toContain(one);
  });
});
