import { describe, expect, it } from "vitest";
import { MOST_USED, suggestedGroups } from "./suggested";
import type { RelationCountsInfo } from "./ipc/relations";

const row = (id: string, all: number, open: number): RelationCountsInfo => ({
  kind: "folder",
  id,
  sessions: { all, open, ended: all - open, archived: 0 },
  projects: 0,
  folders: 0,
  apps: 0,
  subscriptions: 0,
  tools: 0,
  machines: 0,
  launch_settings: 0,
  pools: 0,
});

describe("AJ-E3105 a picker of things: open now, then the most used", () => {
  it("lists the open ones first, the most used of the rest next, and never one twice", () => {
    const groups = suggestedGroups([row("a", 3, 0), row("b", 9, 2), row("c", 0, 0), row("d", 7, 0), row("e", 7, 1)]);
    expect(groups).toEqual([
      { label: "Open now", values: ["b", "e"] },
      { label: "Most used", values: ["d", "a"] },
    ]);
  });

  it("keeps Most used short, and has no group where nothing was used", () => {
    const many = Array.from({ length: 12 }, (_, n) => row(`f${n}`, n + 1, 0));
    expect(suggestedGroups(many)[0]!.values).toHaveLength(MOST_USED);
    expect(suggestedGroups([row("a", 0, 0)])).toEqual([]);
    expect(suggestedGroups([])).toEqual([]);
  });
});
