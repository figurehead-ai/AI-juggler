/**
 * D-118 (AJ-E3702 R4, R5) — the feed's cards: one thing one card, rollups
 * by key within their window, another kind of the same App restarting the
 * window, nothing broken ever folded into a quiet rollup, rollups across
 * page boundaries, the away divider and digest, day separators. Plus the
 * reader's place (R6) and Home's layout by width (AJ-E3701 R4).
 */
import { describe, expect, it } from "vitest";
import { AWAY_MS, awayOf, cardsOf, dayStarts, restartOf, stateOf } from "./feed-rollup";
import { heldBack, newWords } from "./use-feed-stream";
import { homeLayout } from "./home-layout";
import { feedItem } from "../preview/feed";
import type { FeedItemInfo } from "./ipc/feed";

const NOW = new Date(2026, 10, 14, 10, 0, 0).getTime();
const MIN = 60_000;
const HOUR = 3_600_000;

/** A finished session of App `folder` in `project`, `ago` ms ago. */
function finished(id: string, ago: number, { folder = "work", project = "web", level = 1 } = {}): FeedItemInfo {
  return feedItem({ id: `l:${id}`, at_ms: NOW - ago, kind: "finished", category: "finished", level, coalesce: `s:${id}`, rollup: `v:finished:${folder}:${project}:L${level}`, rollup_window_ms: 2 * HOUR, scope: `f:${folder}`, subject: `Session ${id}` });
}
function crashed(id: string, ago: number, folder = "work"): FeedItemInfo {
  // Even were its key a quiet one's, a broken thing never folds.
  return feedItem({ id: `l:${id}`, at_ms: NOW - ago, kind: "crashed", category: "needs", level: 3, coalesce: `s:${id}`, rollup: `v:finished:${folder}:web:L1`, rollup_window_ms: 2 * HOUR, scope: `f:${folder}`, subject: `Session ${id}` });
}

describe("D-118 R4 — one thing, one card; rollups by key", () => {
  it("coalesces a session's ending and its finished notice into one card, at the newest's place", () => {
    const notice = { ...finished("a", 1 * MIN), id: "n:7" };
    const cards = cardsOf([notice, finished("a", 2 * MIN), finished("b", 3 * HOUR)]);
    expect(cards).toHaveLength(2);
    expect(cards[0]!.things[0]!.items.map((one) => one.id)).toEqual(["n:7", "l:a"]);
    expect(cards[0]!.rolled).toBe(false);
  });

  it("rolls the same verb, App and project up within 2 h; past it, a new card", () => {
    const cards = cardsOf([finished("a", 10 * MIN), finished("b", 40 * MIN), finished("c", 90 * MIN), finished("d", 3 * HOUR)]);
    expect(cards.map((card) => card.things.length)).toEqual([3, 1]);
    expect(cards[0]!.rolled).toBe(true);
    expect(cards[0]!.head.id).toBe("l:a");
  });

  it("never rolls another App's or another project's finishes into it", () => {
    const cards = cardsOf([finished("a", 10 * MIN), finished("b", 20 * MIN, { folder: "home" }), finished("c", 30 * MIN, { project: "notes" }), finished("d", 40 * MIN)]);
    expect(cards.map((card) => card.things.map((thing) => thing.head.id))).toEqual([["l:a", "l:d"], ["l:b"], ["l:c"]]);
  });

  it("a crash of the same App starts the window again: nothing is hidden inside a rollup", () => {
    const cards = cardsOf([finished("a", 10 * MIN), crashed("x", 20 * MIN), finished("b", 30 * MIN)]);
    expect(cards.map((card) => card.head.kind)).toEqual(["finished", "crashed", "finished"]);
    expect(cards.every((card) => !card.rolled)).toBe(true);
  });

  it("never folds a broken (L3) thing, whatever its key; a level boundary is never crossed", () => {
    const cards = cardsOf([crashed("x", 10 * MIN), crashed("y", 11 * MIN, "home"), finished("a", 12 * MIN, { level: 0 }), finished("b", 13 * MIN, { level: 1 })]);
    for (const card of cards) {
      const levels = new Set(card.things.map((thing) => thing.head.level));
      expect(levels.size).toBe(1);
      if (card.level === 3) expect(card.rolled).toBe(false);
    }
    expect(cards).toHaveLength(4);
  });

  it("is one card across a page boundary: the cards are made over every page loaded", () => {
    const page1 = [finished("a", 10 * MIN), finished("b", 20 * MIN)];
    const page2 = [finished("c", 30 * MIN), finished("d", 5 * HOUR)];
    expect(cardsOf(page1)).toHaveLength(1);
    const both = cardsOf([...page1, ...page2]);
    expect(both.map((card) => card.things.length)).toEqual([3, 1]);
  });

  it("says a crash, not its restart, with the restart as its fact", () => {
    const restart = feedItem({ id: "n:9", at_ms: NOW - MIN, kind: "restarted", category: "needs", level: 3, coalesce: "s:x" });
    const [card] = cardsOf([restart, crashed("x", 2 * MIN)]);
    expect(stateOf(card!.things[0]!).kind).toBe("crashed");
    expect(restartOf(card!.things[0]!)?.id).toBe("n:9");
  });
});

describe("D-118 R5 — the away divider and digest", () => {
  const items = [finished("a", 10 * MIN), crashed("x", 3 * HOUR), finished("b", 5 * HOUR, { folder: "home" }), finished("old", 10 * HOUR)];

  it("shows nothing when the last look is under 2 h ago, or never kept", () => {
    expect(awayOf(cardsOf(items), NOW - AWAY_MS + MIN, NOW, false)).toBeNull();
    expect(awayOf(cardsOf(items), null, NOW, false)).toBeNull();
  });

  it("sits before the first card at or before the last look, with what happened since: counts by kind, the broken first", () => {
    const away = awayOf(cardsOf(items), NOW - 8 * HOUR, NOW, false)!;
    expect(away.index).toBe(3);
    expect(away.kinds).toEqual(["finished", "crashed", "finished"]);
    expect(away.top.map((thing) => thing.head.id)).toEqual(["l:x", "l:a", "l:b"]);
  });

  it("waits for its boundary to load while more pages follow; at the end of the feed it closes the list", () => {
    const recent = items.slice(0, 3);
    expect(awayOf(cardsOf(recent), NOW - 8 * HOUR, NOW, true)).toBeNull();
    expect(awayOf(cardsOf(recent), NOW - 8 * HOUR, NOW, false)!.index).toBe(3);
  });

  it("starts a day separator at each new day", () => {
    const cards = cardsOf([finished("a", MIN), finished("y", 26 * HOUR, { folder: "home" }), finished("z", 27 * HOUR, { project: "x" })]);
    expect([...dayStarts(cards)]).toEqual([0, 1]);
  });
});

describe("D-118 R6 — the reader's place", () => {
  const ids = (list: FeedItemInfo[]) => list.map((one) => one.id);
  const page = [finished("c", 3 * MIN), finished("b", 2 * HOUR + 1), finished("a", 5 * HOUR)];

  it("at the top, everything shows: nothing waits", () => {
    expect(heldBack(page, null)).toEqual({ shown: page, waiting: 0 });
  });

  it("scrolled down, new items wait behind the pill and what's shown never moves", () => {
    const newer = [finished("e", MIN), finished("d", 2 * MIN), ...page];
    const held = heldBack(newer, "l:c");
    expect(held.waiting).toBe(2);
    expect(ids(held.shown)).toEqual(ids(page));
    expect(newWords(held.waiting)).toBe("2 new updates");
    expect(newWords(1)).toBe("1 new update");
  });

  it("a top that left the list holds nothing back", () => {
    expect(heldBack(page, "l:gone").waiting).toBe(0);
  });
});

describe("AJ-E3701 R4 — Home's layout by its main area's width", () => {
  it("is two columns from about 1,000 px (960: a 1280 px window with the rail open), stacked under it, a strip of App icons under 640", () => {
    expect(homeLayout(1280)).toBe("columns");
    expect(homeLayout(990)).toBe("columns");
    expect(homeLayout(959)).toBe("stacked");
    expect(homeLayout(740)).toBe("stacked");
    expect(homeLayout(900)).toBe("stacked");
    expect(homeLayout(639)).toBe("strip");
    // Not measured yet (a test's DOM): the columns.
    expect(homeLayout(0)).toBe("columns");
  });
});
