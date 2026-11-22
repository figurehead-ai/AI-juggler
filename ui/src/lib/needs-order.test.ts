import { describe, expect, it } from "vitest";
import { mergeByWait } from "./needs-order";

type Row = { id: string; level: number; since: number };
const row = (id: string, level: number, since: number): Row => ({ id, level, since });
const ids = (rows: Row[]) => rows.map((one) => one.id);

describe("one order for what needs you (D-118's mock, Home round 3)", () => {
  it("the one waiting longest first: a session's need asked 12 min ago before an agent's question of 5 min", () => {
    const sessions = [row("billing-webhooks", 2, 1_000 - 12 * 60_000)];
    const messages = [row("builder-1", 2, 1_000 - 5 * 60_000)];
    expect(ids(mergeByWait([messages, sessions], (one) => one))).toEqual(["billing-webhooks", "builder-1"]);
    expect(ids(mergeByWait([sessions, messages], (one) => one))).toEqual(["billing-webhooks", "builder-1"]);
  });

  it("what broke (L3) before what waits (L2), however long that waited", () => {
    const merged = mergeByWait([[row("asks", 2, 1)], [row("failed", 3, 900)]], (one) => one);
    expect(ids(merged)).toEqual(["failed", "asks"]);
  });

  it("each list keeps its own order; a tie goes to the earlier list", () => {
    const own = [row("a1", 2, 50), row("a2", 2, 10)]; // its own order, not by time
    const other = [row("b1", 2, 50), row("b2", 2, 20)];
    // a1 and b1 tie: a1 (the earlier list). a2 stays after a1, though older.
    expect(ids(mergeByWait([own, other], (one) => one))).toEqual(["a1", "a2", "b1", "b2"]);
  });

  it("empty lists merge to the rest", () => {
    expect(ids(mergeByWait<Row>([[], [row("x", 2, 1)], []], (one) => one))).toEqual(["x"]);
  });
});
