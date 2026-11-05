/**
 * The P3 walk: an App member brings in every session the App runs, and the
 * ended ones piled up on the canvas. The canvas draws the open work and
 * the sessions named on their own; the rest stay in the List view.
 */
import { describe, expect, it } from "vitest";
import { drawnMembers } from "./useCanvas";
import { MOCK_SESSIONS, type InstanceInfo } from "../../lib/bridge";
import type { PoolMemberRow } from "../../lib/ipc/pools";

const base = MOCK_SESSIONS[0] as InstanceInfo;
const row = (id: string, status: string, via: string): PoolMemberRow => ({ session: { ...base, id, status }, card: { session_id: id, role: "member" } as PoolMemberRow["card"], via });

describe("the canvas's members", () => {
  it("draws open sessions and named ones, never an ended one a broad member brought", () => {
    const items = [row("open-by-app", "running", "app"), row("ended-by-app", "finished", "app"), row("ended-named", "stopped", "session"), row("ended-watched", "finished", "watched")];
    expect(drawnMembers(items, new Map()).map((m) => m.id)).toEqual(["open-by-app", "ended-named", "ended-watched"]);
  });

  it("judges by the session as it is now", () => {
    const live = new Map([["ended-by-app", { ...base, id: "ended-by-app", status: "running" }]]);
    expect(drawnMembers([row("ended-by-app", "finished", "app")], live).map((m) => m.id)).toEqual(["ended-by-app"]);
  });
});
