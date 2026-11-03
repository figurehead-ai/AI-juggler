/**
 * E31 (AJ-E3103) vitest — the related lists' order (needs you, working,
 * open, the last 7 days, earlier), their state filter, and the hold that
 * keeps a list still while it's read ("N new").
 */
import { describe, expect, it } from "vitest";
import { BANDS, RECENT_MS, bandOf, facetsOf, filtersOn, ordered, passes, stateQuery, NO_FILTERS, type Belongs } from "./order";
import { holdAgain, holdAll, waitingWords } from "./hold";
import { allPairs } from "../../test/pairwise";
import type { RailItem, RailState } from "../../lib/rail-items";

const NOW = 1_800_000_000_000;
const item = (key: string, state: RailState, lastMs: number): RailItem & { k: string } => ({ key, k: key, state, live: state !== "ended", lastMs }) as RailItem & { k: string };

describe("AJ-E3103 the order", () => {
  it("is needs you, working, open, the last 7 days, then earlier, each newest first", () => {
    const rows = [
      item("e-old", "ended", NOW - 30 * 86_400_000),
      item("idle", "idle", NOW - 5_000),
      item("e-new", "ended", NOW - 86_400_000),
      item("asks", "asks_you", NOW - 9_000),
      item("work-old", "working", NOW - 8_000),
      item("review", "done_review", NOW - 1_000),
      item("work-new", "working", NOW - 1_000),
      item("e-edge", "ended", NOW - RECENT_MS),
    ];
    const out = ordered(rows, (row) => row, (row) => row.key, NOW);
    expect(out.map((row) => row.key)).toEqual(["asks", "work-new", "work-old", "review", "idle", "e-new", "e-edge", "e-old"]);
    expect(out.map((row) => bandOf(row, NOW))).toEqual(["needs", "working", "working", "open", "open", "recent", "recent", "earlier"]);
    expect(BANDS).toEqual(["needs", "working", "open", "recent", "earlier"]);
  });

  it("ties fall to the key, so a page's end never repeats or skips a row", () => {
    const rows = [item("b", "ended", 5), item("a", "ended", 5)];
    expect(ordered(rows, (row) => row, (row) => row.key, NOW).map((row) => row.key)).toEqual(["a", "b"]);
  });

  it("the state filter asks the backend what the preview narrows by", () => {
    expect(stateQuery("all", NOW)).toEqual({ which: null, since_ms: null });
    expect(stateQuery("open", NOW)).toEqual({ which: "open", since_ms: null });
    expect(stateQuery("recent", NOW)).toEqual({ which: "ended", since_ms: NOW - RECENT_MS });
    expect(stateQuery("ended", NOW)).toEqual({ which: "ended", since_ms: null });
    const of: Belongs = { project: "p1", app: "cli:a", account: "acct:a", machine: "this-mac" };
    const recent = item("r", "ended", NOW - 86_400_000);
    const old = item("o", "ended", NOW - 30 * 86_400_000);
    const open = item("w", "working", NOW);
    expect([recent, old, open].filter((row) => passes(row, of, { ...NO_FILTERS, state: "recent" }, NOW)).map((row) => row.key)).toEqual(["r"]);
    expect([recent, old, open].filter((row) => passes(row, of, { ...NO_FILTERS, state: "ended" }, NOW)).map((row) => row.key)).toEqual(["r", "o"]);
    expect([recent, old, open].filter((row) => passes(row, of, { ...NO_FILTERS, state: "open" }, NOW)).map((row) => row.key)).toEqual(["w"]);
    expect(passes(open, of, { ...NO_FILTERS, project: "" }, NOW)).toBe(false);
    expect(passes(open, { ...of, project: "" }, { ...NO_FILTERS, project: "" }, NOW)).toBe(true);
    expect(filtersOn({ ...NO_FILTERS, state: "open", app: "cli:a" })).toBe(2);
  });

  it("a filter's values come with their counts, the most rows first", () => {
    const of = (project: string, app: string, account: string | null): Belongs => ({ project, app, account, machine: "this-mac" });
    const facets = facetsOf([of("p1", "cli:a", "acct:a"), of("", "cli:b", null), of("p1", "cli:b", "acct:a")]);
    expect(facets.project).toEqual([{ id: "p1", count: 2 }, { id: "", count: 1 }]);
    expect(facets.app).toEqual([{ id: "cli:b", count: 2 }, { id: "cli:a", count: 1 }]);
    expect(facets.account).toEqual([{ id: "acct:a", count: 2 }]);
    expect(facets.machine).toEqual([{ id: "this-mac", count: 3 }]);
  });
});

describe("AJ-E3103 the list holds still: new rows wait behind “N new”", () => {
  const band = (key: string) => (key.startsWith("w") ? "working" : "recent");

  it("a row that comes in above what's shown waits; the shown rows keep their places", () => {
    const first = holdAll(["w1", "e1", "e2"], band);
    const again = holdAgain(first, ["w2", "e1", "w1", "e2"], band);
    expect(again.keys).toEqual(["w1", "e1", "e2"]);
    expect([...again.waiting]).toEqual(["w2"]);
    expect(waitingWords(again)).toBe("1 new");
    // Another arrives: still waiting, counted once each.
    const third = holdAgain(again, ["w3", "w2", "e1", "w1", "e2"], band);
    expect(third.keys).toEqual(["w1", "e1", "e2"]);
    expect(waitingWords(third)).toBe("2 new");
  });

  it("a row that ends stays where it is, its section kept, and says the order changed", () => {
    const first = holdAll(["w1", "e1"], band);
    // w1 ended: the backend now orders it after e1.
    const ended = holdAgain(first, ["e1", "w1"], (key) => (key === "w1" ? "recent" : band(key)));
    expect(ended.keys).toEqual(["w1", "e1"]);
    expect(ended.bands.get("w1")).toBe("working");
    expect(waitingWords(ended)).toBe("Show in order");
  });

  it("a row that's gone leaves and the rest close up; the next page joins the end", () => {
    const first = holdAll(["w1", "e1", "e2"], band);
    const gone = holdAgain(first, ["w1", "e2"], band);
    expect(gone.keys).toEqual(["w1", "e2"]);
    const more = holdAgain(gone, ["w1", "e2", "e3", "e4"], band);
    expect(more.keys).toEqual(["w1", "e2", "e3", "e4"]);
    expect(waitingWords(more)).toBeNull();
  });
});

describe("the pairwise generator", () => {
  it("covers every pair of values the constraint allows, deterministically", () => {
    const params = { page: ["project", "app", "account"], size: ["0", "1", "9"], filter: ["none", "app", "project"] } as const;
    const allowed = (row: { page: string; filter: string }) => !(row.page === "project" && row.filter === "project") && !(row.page === "app" && row.filter === "app");
    const rows = allPairs(params, allowed);
    expect(allPairs(params, allowed)).toEqual(rows);
    for (const page of params.page)
      for (const size of params.size) expect(rows.some((row) => row.page === page && row.size === size)).toBe(true);
    expect(rows.every(allowed)).toBe(true);
    expect(rows.length).toBeLessThan(3 * 3 * 3);
  });
});
