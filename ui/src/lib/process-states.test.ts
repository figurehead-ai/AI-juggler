import { describe, expect, it } from "vitest";
import { actorWords, ageWords, cpuWords, outcomeWords, portWords, processState, roleWords, STATE_FILTERS, ZOMBIE_WORDS } from "./process-words";
import { treeKey, windowRows, type TreeRowLike } from "./tree-grid";

describe("the Processes view's words", () => {
  it("names every state with a word and an icon, never colour alone", () => {
    for (const word of [...STATE_FILTERS, "ended"]) {
      const state = processState(word);
      expect(state.label).not.toBe("Unknown");
      expect(state.Icon).toBeTypeOf("function");
      expect(state.hint.length).toBeGreaterThan(10);
    }
    expect(processState("nonsense").label).toBe("Unknown");
  });

  it("explains a zombie in plain words, never the code", () => {
    expect(processState("zombie").label).toBe("Waiting for its parent");
    expect(ZOMBIE_WORDS).toMatch(/only its parent can clear it/);
    expect(ZOMBIE_WORDS.toLowerCase()).not.toMatch(/zombie|t1|reap/);
  });

  it("says a role is a guess unless it is known", () => {
    expect(roleWords("dev_server", "dev server", false)).toBe("probably a dev server");
    expect(roleWords("mcp_server", "MCP server", false)).toBe("probably an MCP server");
    expect(roleWords("hook", "AI Juggler hook", true)).toBe("AI Juggler hook");
    expect(roleWords("tool", "", true)).toBe("the session itself");
  });

  it("reads numbers and outcomes in short plain words", () => {
    expect(portWords([5173, 8000])).toBe(":5173, :8000");
    expect(ageWords(40_000)).toBe("40 s");
    expect(ageWords(12 * 60_000)).toBe("12 min");
    expect(ageWords(3 * 3_600_000)).toBe("3 h");
    expect(ageWords(3 * 86_400_000)).toBe("3 d");
    expect(cpuWords(null)).toBe("—");
    expect(cpuWords(0.34)).toBe("0.3 %");
    expect(cpuWords(4.4)).toBe("4 %");
    expect(outcomeWords("pid 8123: exited on TERM")).toBe("Stopped when asked");
    expect(outcomeWords("pid 1: needed KILL")).toBe("Needed a force stop");
    expect(outcomeWords("pid 1: refused")).toBe("Didn't stop");
    expect(actorWords("user:observed")).toBe("You (another app's session)");
    expect(actorWords("auto:tier-1")).toMatch(/^Automatic/);
  });
});

describe("the tree table's mechanics", () => {
  it("keeps at most 60 rows in the page whatever the list's length", () => {
    expect(windowRows(2_000, 44, 0, 800)).toEqual([0, 35]);
    const [start, end] = windowRows(2_000, 44, 44 * 1_000, 3_000);
    expect(end - start).toBeLessThanOrEqual(60);
    expect(start).toBe(992);
    expect(windowRows(0, 44, 0, 800)).toEqual([0, 0]);
  });

  const rows: TreeRowLike[] = [
    { key: "a", depth: 0, parent_key: null, has_children: true },
    { key: "s", depth: 1, parent_key: "a", has_children: true },
    { key: "p1", depth: 2, parent_key: "s", has_children: true },
    { key: "p2", depth: 3, parent_key: "s", has_children: false },
  ];
  const open = () => true;

  it("moves like a treegrid: arrows, Home and End, open and close", () => {
    expect(treeKey(rows, 0, "ArrowDown", open)).toEqual({ to: 1 });
    expect(treeKey(rows, 0, "ArrowUp", open)).toEqual({ to: 0 });
    expect(treeKey(rows, 2, "End", open)).toEqual({ to: 3 });
    expect(treeKey(rows, 3, "Home", open)).toEqual({ to: 0 });
    // → on an open row goes to its first child; on a closed one opens it.
    expect(treeKey(rows, 1, "ArrowRight", open)).toEqual({ to: 2 });
    expect(treeKey(rows, 1, "ArrowRight", () => false)).toEqual({ toggle: "open", at: 1 });
    // ← closes an open row, else goes to its parent (a subprocess's is
    // the nearest shallower row).
    expect(treeKey(rows, 1, "ArrowLeft", open)).toEqual({ toggle: "close", at: 1 });
    expect(treeKey(rows, 3, "ArrowLeft", open)).toEqual({ to: 1 });
    expect(treeKey(rows, 0, "x", open)).toBeNull();
  });
});
