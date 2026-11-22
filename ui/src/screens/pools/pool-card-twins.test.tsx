/**
 * R6 (the 0.5.0 re-walk, P11): pool A's card drew "web · started 1…"
 * twice: two untitled sessions in one folder, cut where they differ. The
 * card titles its sessions as the pool's diagram does (the name, then when
 * each started on the second line, first, where it isn't cut), so twins
 * read apart.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { PoolRulesInfo } from "../../lib/ipc/pool-model";
import { MOCK_SESSIONS } from "../../preview/fixtures";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { miniTeam, PoolMini } from "./PoolMini";
import { cardTitles } from "./PoolsScreen";

afterEach(cleanup);

const at = (hour: number, minute: number) => new Date(2026, 10, 21, hour, minute).getTime();
const twin = (id: string, started: number): InstanceInfo => ({ ...MOCK_SESSIONS[0]!, id, label: null, title: null, git_branch: null, cwd: "/lab/repos/web", started_at_ms: started });
const rule = (id: string) => ({ member: `session:${id}`, kind: "session", role: "member", level: null, joined_at: 1, sessions: 1, session_ids: [id] });

describe("R6 twins on a Pools card", () => {
  it("read apart: the same name, each with when it started on its line", () => {
    const sessions = [twin("s-a", at(22, 20)), twin("s-b", at(22, 22))];
    const titles = cardTitles(sessions, new Map(), at(23, 0));
    const rules: PoolRulesInfo = { pool_id: "a", members: [rule("s-a"), rule("s-b")], talk: [], lead_next: null };
    const team = miniTeam(
      rules,
      [],
      () => "member",
      { apps: [], sessions },
      (ref) => titles.get(ref.slice("session:".length))?.title ?? "",
      (ref) => titles.get(ref.slice("session:".length))?.when ?? null,
    );
    render(<PoolMini lead={team.lead} members={team.members} />);
    const cards = screen.getAllByTestId("pool-mini-card").map((card) => card.textContent ?? "");
    expect(cards).toHaveLength(2);
    expect(cards[0]).not.toBe(cards[1]);
    expect(screen.getAllByTestId("pool-mini-card-name").map((name) => name.textContent)).toEqual(["web", "web"]);
    // When each started leads the second line, so the cut never takes it.
    expect(cards[0]).toMatch(/^web\s*started 10:20\sPM · /);
    expect(cards[1]).toMatch(/^web\s*started 10:22\sPM · /);
  });
});
