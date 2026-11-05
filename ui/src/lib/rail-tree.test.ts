/**
 * 0.3.9 (the maintainer's walk of 2026-11-04): a Claude app side session
 * ("AWS WS-A", started from "AWS migration planning") showed in Open now
 * as a sibling of the session that started it. It sits under it, as the
 * app's own sidebar draws it.
 */
import { describe, expect, it } from "vitest";
import { conversationItem } from "./rail-items";
import { buildLines, type Line } from "./rail-lines";
import { MAX_DEPTH, nestRows } from "./rail-tree";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import type { ConversationInfo } from "./ipc/conversations";

const NOW = new Date(2026, 10, 4, 15, 0, 0).getTime();
const none = () => null;

function talk(id: string, from: string | null, ago: number): ConversationInfo {
  return {
    id, tool: "claude-code", account_id: "a1", account_label: "Default", project_id: null, cwd: "/w/infra", title: `Talk ${id}`, git_branch: "main",
    started_ms: NOW - 3_600_000, last_activity_ms: NOW - ago, tokens: null, live: true, session_id: null,
    places: [{ app: "Claude", shown: "Claude app", app_id: "claude", profile: "Default", instance: "Default", pid: 10, kind: "desktop", app_pid: 11, label: "Claude · Default" }],
    pid: 10, state: "idle", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, started_from: from,
  };
}

const rows = (lines: Line[]) => lines.flatMap((line) => (line.kind === "row" ? [{ id: line.item.conversation?.id, depth: line.depth ?? 0, parent: line.parent ?? null }] : []));

describe("a session started from another sits under it (0.3.9)", () => {
  it("in Open now, under its app profile, as the app's sidebar draws it", () => {
    const items = [
      talk("worldbuilder", null, 1_000),
      talk("plan", null, 2_000),
      talk("ws-a", "plan", 3_000),
      talk("build", null, 4_000),
      talk("ws-a-1", "ws-a", 5_000),
    ].map((c) => conversationItem(c, none, null));
    const { lines } = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null });
    const drawn = rows(lines);
    // Whatever order the top level is in, the planning session's own run
    // straight under it, each a level deeper.
    const at = drawn.findIndex((row) => row.id === "plan");
    expect(drawn.slice(at, at + 3)).toEqual([
      { id: "plan", depth: 0, parent: null },
      { id: "ws-a", depth: 1, parent: "Talk plan" },
      { id: "ws-a-1", depth: 2, parent: "Talk ws-a" },
    ]);
    expect(drawn.filter((row) => row.depth === 0).map((row) => row.id).sort()).toEqual(["build", "plan", "worldbuilder"]);
  });

  it("one whose parent isn't here keeps its place; siblings keep their order", () => {
    const items = [talk("b", "gone", 1), talk("p", null, 2), talk("c2", "p", 3), talk("c1", "p", 4)].map((c) => conversationItem(c, none, null));
    expect(nestRows(items).map(({ item, depth }) => [item.conversation?.id, depth])).toEqual([
      ["b", 0],
      ["p", 0],
      ["c2", 1],
      ["c1", 1],
    ]);
  });

  it("a ring of parents, or one deeper than the rail draws, never loses a row", () => {
    const ring = [talk("x", "y", 1), talk("y", "x", 2)].map((c) => conversationItem(c, none, null));
    expect(nestRows(ring).map(({ item }) => item.conversation?.id).sort()).toEqual(["x", "y"]);
    const deep = Array.from({ length: 7 }, (_, n) => conversationItem(talk(`d${n}`, n === 0 ? null : `d${n - 1}`, n), none, null));
    const nested = nestRows(deep);
    expect(nested).toHaveLength(7);
    expect(Math.max(...nested.map((row) => row.depth))).toBe(MAX_DEPTH);
  });
});
