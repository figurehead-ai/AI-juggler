/**
 * D-47 — Open now's order, rule by rule: in Your order a state change
 * never moves a row, a new row joins the end, pinned rows lead, a gone row
 * closes its gap and nothing else moves; drags and ⌥⌘↑/↓ moves; Reset;
 * Name and Latest activity; Needs you mirrored with each list counting a
 * session once; the record read guarded, kept and pruned.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { conversationItem, needsYou, openNow, sessionItem, type RailItem } from "./rail-items";
import { buildLines, type Line, type OrderInputs } from "./rail-lines";
import { groupItems } from "./rail-places";
import {
  arrangeGroup,
  arrangeGroups,
  dayOf,
  EMPTY_ORDER,
  FORGET_DAYS,
  LONG_GROUP,
  moveIn,
  movedWords,
  parseOrder,
  rankOf,
  reconcile,
  resetOrder,
  rowRange,
  type RailOrder,
} from "./rail-order";
import { moveGroup, moveRow, ORDER_KEY, railOrder, reconcileOrder, resetMyOrder, resetRailOrderCache, restoreOrder, setOrderMode, togglePin } from "./rail-order-store";
import { DEFAULT_VIEW, parseQuery } from "./rail-view";
import type { ConversationInfo, PlaceInfo } from "./ipc/conversations";
import type { InstanceInfo } from "./bridge";

const NOW = new Date(2026, 9, 27, 15, 0, 0).getTime();
const none = () => null;
const TODAY = dayOf(NOW);

function place(app: string, profile: string): PlaceInfo {
  return { app, app_id: app.toLowerCase(), profile, instance: profile, pid: 10, kind: "desktop", app_pid: 11, label: `${app} · ${profile}` };
}

function talk(id: string, over: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id, tool: "claude-code", account_id: "a1", account_label: "Work", project_id: null, cwd: "/w/app", title: `Talk ${id}`, git_branch: "main",
    started_ms: NOW - 3_600_000, last_activity_ms: NOW - 60_000, tokens: null, live: true, session_id: null, places: [place("Claude", "Work")],
    pid: 10, state: "idle", probably: false, viewed_ms: null, background: false, inactive: false, subtask_count: null, ...over,
  };
}

function own(id: string, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id, launch_recipe_id: "p", adopted: false, provider: "claude-code", account_id: "a1", account_label: "Work", model: null, launch_model: null,
    running_model: null, model_source: "launch", status: "running", quota: null, tokens: null, cpu_pct: null, rss_bytes: null, child_count: null,
    subtask_count: null, git_branch: "main", git_dirty_count: null, needs_attention: null, attention: null, started_at_ms: NOW - 60_000,
    last_activity_ms: NOW - 30_000, machine_id: "this-mac", cwd: "/w/app", kind: "terminal", isolation_label: null, project_id: null, conversation_id: null, ...over,
  } as InstanceInfo;
}

/** Four sessions in one place, started a minute apart (a first, d last). */
const four = (over: Record<string, Partial<ConversationInfo>> = {}, pins: string[] = []): RailItem[] =>
  ["a", "b", "c", "d"].map((id, n) => conversationItem(talk(id, { started_ms: NOW - (4 - n) * 60_000, ...over[id] }), none, null, pins.includes(`c:a1/${id}`)));

/** Open now's rows, top to bottom, as the rail would draw them. */
function drawn(items: RailItem[], order: Partial<OrderInputs> = {}, frozen: Map<string, string[]> | null = null): string[] {
  const inputs: OrderInputs = { mode: "yours", rows: new Map(), groups: [], folded: new Set(), ...order };
  return buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen, order: inputs })
    .lines.filter((line): line is Extract<Line, { kind: "row" }> => line.kind === "row")
    .map((line) => line.item.key.slice("c:a1/".length));
}

/** The record after the rail has seen `items` (as PlaceBoard does). */
function seenBy(items: RailItem[], order: RailOrder = EMPTY_ORDER, today = TODAY): RailOrder {
  return reconcile(order, { known: items.map((item) => item.key), open: items.filter(openNow), lens: "place", groups: groupItems(items.filter(openNow), "place").map((g) => g.key), today }).order;
}

const yours = (order: RailOrder): Partial<OrderInputs> => ({ rows: rankOf(order.rows), groups: order.groups.place });

describe("Your order: nothing moves by itself (D-47)", () => {
  it("a state change never moves a row: Working, Idle, Asks you, Done and a new event", () => {
    const start = four();
    const order = seenBy(start);
    expect(drawn(start, yours(order))).toEqual(["a", "b", "c", "d"]);
    const flipped = four({
      a: { state: "working", last_activity_ms: NOW },
      b: { state: "asks_you", last_activity_ms: NOW - 1 },
      c: { state: "done_review" },
      d: { state: "working", last_activity_ms: NOW + 5 },
    });
    expect(drawn(flipped, yours(seenBy(flipped, order)))).toEqual(["a", "b", "c", "d"]);
    // And back again, many times over.
    for (let round = 0; round < 5; round += 1) {
      const next = four({ [["a", "b", "c", "d"][round % 4]!]: { state: round % 2 ? "working" : "idle", last_activity_ms: NOW + round } });
      expect(drawn(next, yours(seenBy(next, order)))).toEqual(["a", "b", "c", "d"]);
    }
  });

  it("a new session joins the end of its group, in the order it was first seen", () => {
    const order = seenBy(four());
    const later = [...four(), conversationItem(talk("e", { started_ms: NOW - 9 * 3_600_000 }), none, null)];
    // Started long ago, but first seen now: at the end.
    expect(drawn(later, yours(seenBy(later, order)))).toEqual(["a", "b", "c", "d", "e"]);
  });

  it("an ended session frees its slot: the rows under it close up, nothing else moves", () => {
    const order = seenBy(four());
    const ended = four({ b: { live: false, places: [] } });
    expect(drawn(ended, yours(seenBy(ended, order)))).toEqual(["a", "c", "d"]);
  });

  it("pinned rows lead their group, in every order", () => {
    const pinnedD = four({ d: { title: "Zebra" } }, ["c:a1/d"]);
    const order = seenBy(pinnedD);
    expect(drawn(pinnedD, yours(order))).toEqual(["d", "a", "b", "c"]);
    expect(drawn(pinnedD, { mode: "name" })).toEqual(["d", "a", "b", "c"]);
    expect(drawn(pinnedD, { mode: "latest" })[0]).toBe("d");
    // An AI Juggler session's own pin counts the same.
    const mine = sessionItem(own("mine", { pinned: true, started_at_ms: NOW }), none, false);
    expect(mine.pinned).toBe(true);
  });

  it("a long group shows the first 8 by the user's order, then +n more, never re-picked by activity", () => {
    const many = Array.from({ length: 11 }, (_, n) => conversationItem(talk(`r${n}`, { started_ms: NOW - (20 - n) * 60_000 }), none, null));
    const order = seenBy(many);
    const group = groupItems(many, "place")[0]!;
    const fold = arrangeGroup(group, "yours", rankOf(order.rows), false);
    expect(fold.shown.map((item) => item.key)).toEqual(many.slice(0, LONG_GROUP).map((item) => item.key));
    expect([fold.hidden, fold.words]).toEqual([3, "more"]);
    // The last one starts working: the same 8.
    const busy = { ...group, items: group.items.map((item) => (item.key === "c:a1/r10" ? { ...item, state: "working" as const, lastMs: NOW + 1 } : item)) };
    expect(arrangeGroup(busy, "yours", rankOf(order.rows), false).shown.map((item) => item.key)).toEqual(fold.shown.map((item) => item.key));
    expect(arrangeGroup(group, "yours", rankOf(order.rows), true).hidden).toBe(0);
  });

  it("a place folds only when the user folds it, and stays folded", () => {
    const quiet = four();
    const lines = (folded: Set<string>) =>
      buildLines({ items: quiet, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, order: { mode: "yours", rows: new Map(), groups: [], folded } }).lines;
    expect(lines(new Set()).filter((line) => line.kind === "row")).toHaveLength(4);
    const folded = lines(new Set(["place:Claude|Work"]));
    expect(folded.filter((line) => line.kind === "row")).toHaveLength(0);
    expect(folded.find((line) => line.kind === "header")).toMatchObject({ folded: "Work · 4 idle" });
  });
});

describe("arranging: drags and the keyboard twin", () => {
  it("moves one row, and only that one, keeping the other groups' rows where they were", () => {
    expect(moveIn(["a", "x", "b", "c", "y", "d"], ["a", "b", "c", "d"], 3, 1)).toEqual(["a", "x", "d", "b", "y", "c"]);
    expect(moveIn(["a", "b", "c"], ["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"]);
    // A key the record lacks joins first; a move to where it is changes nothing.
    expect(moveIn(["a"], ["a", "b"], 1, 1)).toEqual(["a", "b"]);
  });

  it("a pinned row moves among the pinned ones; the others below them", () => {
    const rows = [{ pinned: true }, { pinned: true }, { pinned: false }, { pinned: false }, { pinned: false }];
    expect(rowRange(rows, 0)).toEqual([0, 1]);
    expect(rowRange(rows, 3)).toEqual([2, 4]);
    expect(movedWords(1, 5)).toBe("Moved to position 2 of 5");
  });

  it("the drawn order follows a move; ⌘1–9 follow a moved place", () => {
    const items = [...four(), conversationItem(talk("t", { places: [place("Terminal", "Work")] }), none, null)];
    const order = seenBy(items);
    const moved: RailOrder = { ...order, rows: moveIn(order.rows, ["c:a1/a", "c:a1/b", "c:a1/c", "c:a1/d"], 3, 0) };
    expect(drawn(items, yours(moved)).slice(0, 4)).toEqual(["d", "a", "b", "c"]);
    const groups = groupItems(items, "place");
    expect(arrangeGroups(groups, "yours", order.groups.place).map((g) => g.key)).toEqual(["Claude|Work", "Terminal|Work"]);
    const swapped = moveIn(order.groups.place, ["Claude|Work", "Terminal|Work"], 1, 0);
    const built = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null, order: { mode: "yours", rows: rankOf(moved.rows), groups: swapped, folded: new Set() } });
    expect(built.groups.map((g) => g.key)).toEqual(["Terminal|Work", "Claude|Work"]);
    expect(built.lines.filter((line) => line.kind === "header").map((line) => (line as { digit: number | null }).digit)).toEqual([1, 2]);
    // Latest activity and Name keep the usual place order.
    expect(arrangeGroups(groups, "latest", swapped).map((g) => g.key)).toEqual(["Claude|Work", "Terminal|Work"]);
  });

  it("Reset my order forgets the arrangement, keeping pins, folds and the order picked", () => {
    const order: RailOrder = { ...EMPTY_ORDER, mode: "name", rows: ["b", "a"], groups: { place: ["x"], account: ["y"], pool: [] }, pins: ["c:a1/a"], folded: ["place:x"] };
    expect(resetOrder(order)).toEqual({ ...order, rows: [], groups: { place: [], account: [], pool: [] } });
    // Re-seen afterwards: by when each started.
    const items = four();
    expect(seenBy(items, resetOrder(order)).rows.slice(-4)).toEqual(items.map((item) => item.key));
  });
});

describe("the other orders", () => {
  it("Name: A to Z, numbers in order", () => {
    const named = four({ a: { title: "fix 10" }, b: { title: "Alpha" }, c: { title: "fix 9" }, d: { title: "beta" } });
    expect(drawn(named, { mode: "name" })).toEqual(["b", "d", "c", "a"]);
  });

  it("Latest activity: today's order, held still under the pointer", () => {
    const items = four({ a: { last_activity_ms: NOW - 9_000 }, b: { last_activity_ms: NOW - 1_000 }, c: { state: "working" }, d: { last_activity_ms: NOW - 5_000 } });
    // Working first, then the newest event first.
    expect(drawn(items, { mode: "latest" })).toEqual(["c", "b", "d", "a"]);
    const frozen = new Map([["group:Claude|Work", ["d", "c", "b", "a"].map((id) => `c:a1/${id}`)]]);
    expect(drawn(items, { mode: "latest" }, frozen)).toEqual(["d", "c", "b", "a"]);
    // Your order isn't frozen: it never re-sorts, and a drag shows at once.
    expect(drawn(items, { mode: "yours" }, frozen)).toEqual(["a", "b", "c", "d"]);
  });
});

describe("Needs you mirrors Open now; each list counts a session once", () => {
  it("one asking is in Needs you and in its place, amber, counted once in each", () => {
    const items = four({ b: { state: "asks_you" } });
    const built = buildLines({ items, view: DEFAULT_VIEW, query: parseQuery(""), now: NOW, toggled: new Set(), frozen: null });
    const count = (kind: Line["kind"], key: string) => built.lines.filter((line) => line.kind === kind && "item" in line && line.item.key === key).length;
    expect(count("need", "c:a1/b")).toBe(1);
    expect(count("row", "c:a1/b")).toBe(1);
    expect(built.needs).toBe(1);
    expect(needsYou(items)).toHaveLength(1);
    const sections = built.lines.filter((line): line is Extract<Line, { kind: "section" }> => line.kind === "section");
    expect(sections.map((line) => [line.id, line.count])).toEqual([["needs", 1], ["open", 4]]);
    // The place says how many of its rows need you.
    expect(built.lines.find((line) => line.kind === "header")).toMatchObject({ needs: 1 });
  });
});

describe("the record: read guarded, kept, pruned", () => {
  it("round-trips, and junk is the defaults", () => {
    const order: RailOrder = { mode: "latest", rows: ["s:1", "c:acct/2"], groups: { place: ["ai-juggler"], account: [], pool: ["pool:p"] }, folded: ["place:x"], pins: ["c:acct/2"], seen: { "s:1": TODAY, "c:acct/2": TODAY } };
    expect(parseOrder(JSON.stringify(order))).toEqual(order);
    expect(parseOrder(null)).toEqual(EMPTY_ORDER);
    expect(parseOrder("{nope")).toEqual(EMPTY_ORDER);
    expect(parseOrder("[1]")).toEqual(EMPTY_ORDER);
    expect(parseOrder(JSON.stringify({ mode: "random", rows: ["a", 3, "a", "x".repeat(500)], seen: { a: "today", b: 2.5, c: 4 } }))).toEqual({ ...EMPTY_ORDER, rows: ["a"], seen: { c: 4 } });
  });

  it("is kept by the rail's own keys, also for sessions other apps run", () => {
    const order = seenBy([...four(), sessionItem(own("s1"), none, false)]);
    expect(order.rows).toContain("s:s1");
    expect(order.rows).toContain("c:a1/a");
  });

  it("a session gone for good is pruned; a known one is kept, marked once a day", () => {
    const old = { ...seenBy(four()), pins: ["c:a1/gone"], seen: {} as Record<string, number> };
    const marked = reconcile(old, { known: ["c:a1/a"], open: [], lens: "place", groups: [], today: TODAY });
    expect(marked.changed).toBe(true);
    expect(reconcile(marked.order, { known: ["c:a1/a"], open: [], lens: "place", groups: [], today: TODAY }).changed).toBe(false);
    const later = reconcile(marked.order, { known: ["c:a1/a"], open: [], lens: "place", groups: [], today: TODAY + FORGET_DAYS + 1 }).order;
    expect(later.rows).toEqual(["c:a1/a"]);
    expect(later.pins).toEqual([]);
  });
});

describe("the store: kept in AI Juggler's settings", () => {
  type Seams = { __AIJ_APP_SETTINGS__?: Record<string, string> };
  const seams = () => window as unknown as Seams;
  beforeEach(() => {
    vi.useFakeTimers();
    resetRailOrderCache();
    localStorage.clear();
  });
  afterEach(() => {
    resetRailOrderCache();
    delete seams().__AIJ_APP_SETTINGS__;
    vi.useRealTimers();
  });

  it("writes the order, pins and the mode, and reads them back after a restart", async () => {
    railOrder();
    await vi.runAllTimersAsync();
    reconcileOrder({ known: ["c:a1/a", "c:a1/b"], open: [{ key: "c:a1/a", startMs: 1 }, { key: "c:a1/b", startMs: 2 }], lens: "place", groups: ["Claude|Work"], today: TODAY });
    moveRow(["c:a1/a", "c:a1/b"], 1, 0);
    moveGroup("place", ["Claude|Work", "Terminal|Work"], 1, 0);
    togglePin("c:a1/b");
    setOrderMode("name");
    await vi.runAllTimersAsync();
    const kept = seams().__AIJ_APP_SETTINGS__?.[ORDER_KEY] ?? null;
    expect(kept).not.toBeNull();
    // A restart: nothing in memory, this window's copy gone; the settings stay.
    resetRailOrderCache();
    localStorage.clear();
    railOrder();
    await vi.runAllTimersAsync();
    expect(railOrder()).toMatchObject({ mode: "name", rows: ["c:a1/b", "c:a1/a"], pins: ["c:a1/b"], groups: { place: ["Terminal|Work", "Claude|Work"] } });
  });

  it("Reset my order, and Undo puts it back", async () => {
    railOrder();
    await vi.runAllTimersAsync();
    moveRow(["x", "y"], 1, 0);
    const before = resetMyOrder();
    expect(railOrder().rows).toEqual([]);
    restoreOrder(before);
    expect(railOrder().rows).toEqual(["y", "x"]);
  });
});
