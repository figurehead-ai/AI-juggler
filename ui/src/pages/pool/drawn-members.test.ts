/**
 * The P3 walk: an App member brings in every session the App runs, and the
 * ended ones piled up on the canvas. The canvas draws the open work and
 * the sessions named on their own; the rest stay in the List view.
 */
import { describe, expect, it } from "vitest";
import { distinctNames, drawnMembers } from "./useCanvas";
import { MOCK_SESSIONS } from "../../preview/fixtures";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { PoolMemberRow } from "../../lib/ipc/pools";

const base = MOCK_SESSIONS[0] as InstanceInfo;
const row = (id: string, status: string, via: string, held_by: string | null = null): PoolMemberRow => ({ session: { ...base, id, status }, card: { session_id: id, role: "member" } as PoolMemberRow["card"], via, held_by });

describe("the canvas's members", () => {
  it("draws open sessions and named ones, never an ended one a broad member brought", () => {
    const items = [row("open-by-app", "running", "app"), row("ended-by-app", "finished", "app"), row("ended-named", "stopped", "session"), row("ended-watched", "finished", "watched")];
    expect(drawnMembers(items, new Map()).map((m) => m.id)).toEqual(["open-by-app", "ended-named", "ended-watched"]);
  });

  it("leaves a session a sealed pool holds out of this one to the List view", () => {
    expect(drawnMembers([row("held", "running", "session", "Client"), row("own", "running", "session")], new Map()).map((m) => m.id)).toEqual(["own"]);
  });

  it("judges by the session as it is now", () => {
    const live = new Map([["ended-by-app", { ...base, id: "ended-by-app", status: "running" }]]);
    expect(drawnMembers([row("ended-by-app", "finished", "app")], live).map((m) => m.id)).toEqual(["ended-by-app"]);
  });

  it("tells two sessions of one name apart by when each started (the P7 and P11 walks)", () => {
    const at = (h: number) => new Date(2026, 10, 5, h, 7).getTime();
    const one = { session: { ...base, id: "a", label: "web", started_at_ms: at(3) } };
    const two = { session: { ...base, id: "b", label: "web", started_at_ms: at(4) } };
    const other = { session: { ...base, id: "c", label: "docs", started_at_ms: at(5) } };
    const names = distinctNames([one, two, other]).map((row) => row.name);
    expect(names[0]).toMatch(/^web · started 0?3:07/);
    expect(names[1]).toMatch(/^web · started 0?4:07/);
    expect(names[2]).toBe("docs");
  });
});

describe("a member's name (rc7 walk)", () => {
  it("is the label, else the title's first words, never the whole prompt nor a branch that names no work", async () => {
    const { nameOf } = await import("./actions");
    const plain: InstanceInfo = { ...base, label: undefined, title: undefined, git_branch: null, cwd: "/w/web" };
    const one = (over: Partial<InstanceInfo>) => nameOf({ ...plain, ...over });
    expect(one({ label: "Writer", title: "Write the notes" })).toBe("Writer");
    expect(one({ title: "Good. Write the release notes for 0.4.1 and check every link in them twice" })).toBe("Write the release notes for 0.4.1 and check…");
    expect(one({ git_branch: "main" })).not.toContain("main");
    expect(one({ git_branch: "fix/limits" })).toBe("fix/limits");
  });
});
