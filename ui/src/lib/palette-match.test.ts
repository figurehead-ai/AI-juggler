/**
 * AJ-E2901 vitest — the loose match ⌘K and the rail share: letters in
 * order inside one word, ranked prefix › word start › inside › loose ›
 * recency; the title's matched letters as ranges; the rail's words.
 */
import { describe, expect, it } from "vitest";
import { fuzzyMatch, highlight, matchWord, rank } from "./palette-match";
import { parseQuery } from "./rail-view";
import { loadRecents, MAX_RECENTS, rememberRecent, keepRecents, clearRecents } from "./palette-recents";

describe("AJ-E2901 palette-match", () => {
  it("matches a word at the start, a word's start, inside, or its letters within one word", () => {
    expect(matchWord("bil", "Billing export")).toEqual({ score: 4, ranges: [[0, 3]] });
    expect(matchWord("exp", "Billing export")).toEqual({ score: 3, ranges: [[8, 11]] });
    expect(matchWord("ill", "Billing export")).toEqual({ score: 2, ranges: [[1, 4]] });
    expect(matchWord("wrk", "Work Max")).toEqual({ score: 1, ranges: [[0, 1], [2, 4]] });
    // Letters never jump across words: "wm" isn't "Work Max".
    expect(matchWord("wm", "Work Max")).toBeNull();
    expect(matchWord("zz", "Work Max")).toBeNull();
  });

  it("every typed word must match the title or another field", () => {
    expect(fuzzyMatch("wrk bil", "Fix billing export", ["Work Max", "Claude Code"])).not.toBeNull();
    expect(fuzzyMatch("wrk bil", "Fix billing export", ["Personal"])).toBeNull();
    expect(fuzzyMatch("", "anything")).toEqual({ score: 0, ranges: [] });
  });

  it("ranks the title starting with what's typed first, then word starts, then the rest, then recency", () => {
    const items = [
      { title: "Fix the billing export", recency: 5 },
      { title: "Billing export", recency: 1 },
      { title: "Rebill old invoices", recency: 9 },
      { title: "Build in lisp", recency: 50 },
      { title: "Fix billing totals", recency: 7 },
    ];
    expect(rank(items, "bil", (item) => item).map(({ item }) => item.title)).toEqual(["Billing export", "Fix billing totals", "Fix the billing export", "Rebill old invoices", "Build in lisp"]);
  });

  it("the work-billing story: wrk bil finds the billing session on Work first", () => {
    const sessions = [
      { title: "Billing dashboard", other: ["Personal"], recency: 10 },
      { title: "Fix billing export", other: ["Work Max"], recency: 1 },
      { title: "Write release notes", other: ["Work Max"], recency: 20 },
    ];
    expect(rank(sessions, "wrk bil", (item) => item).map(({ item }) => item.title)).toEqual(["Fix billing export"]);
  });

  it("returns the title's ranges for bold, merged", () => {
    const found = fuzzyMatch("fix bil", "Fix billing export")!;
    expect(found.ranges).toEqual([[0, 3], [4, 7]]);
    expect(highlight("Fix billing export", found.ranges)).toEqual([
      { text: "Fix", hit: true },
      { text: " ", hit: false },
      { text: "bil", hit: true },
      { text: "ling export", hit: false },
    ]);
  });

  it("the rail's words still parse out before matching", () => {
    const query = parseQuery("is:ended app:claude wrk");
    expect(query.text).toBe("wrk");
    expect(query.is.has("ended")).toBe(true);
    expect(query.only.app).toBe("claude");
  });
});

describe("AJ-E2901 palette-recents", () => {
  it("keeps ids only, newest first, at most 8, drops gone ones and clears", () => {
    localStorage.clear();
    for (let n = 0; n < 10; n++) rememberRecent({ kind: "session", id: `s${n}` });
    rememberRecent({ kind: "session", id: "s3" });
    const recents = loadRecents();
    expect(recents).toHaveLength(MAX_RECENTS);
    expect(recents[0]).toEqual({ kind: "session", id: "s3" });
    expect(Object.keys(recents[0]!)).toEqual(["kind", "id"]);
    keepRecents((recent) => recent.id !== "s9");
    expect(loadRecents().some((recent) => recent.id === "s9")).toBe(false);
    clearRecents();
    expect(loadRecents()).toEqual([]);
    localStorage.setItem("aij.palette.recents", "{broken");
    expect(loadRecents()).toEqual([]);
  });
});
