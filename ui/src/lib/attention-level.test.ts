/**
 * 0.5 (D-117) — attentionLevel(), the one rule: every state and every
 * Needs-you kind to its level (the table the backend's mirror reads too,
 * `fixtures/attention-levels.json`), and the shared count (L2 + L3).
 */
import { describe, expect, it } from "vitest";
import table from "./fixtures/attention-levels.json";
import { attentionLevel, counted, highest, levelTone, noticeLevel, type AttentionSubject } from "./attention";
import { instance } from "../test/fixtures";
import { sessionItem, type RailState } from "./rail-items";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

type Case = AttentionSubject & { level: number };
const CASES: readonly Case[] = table.cases;

describe("attentionLevel()", () => {
  it.each(CASES.map((one) => [JSON.stringify({ ...one, level: undefined }), one] as const))("%s", (_, { level, ...subject }) => {
    expect(attentionLevel(subject)).toBe(level);
  });

  it("places every kind the backend lists (centre.rs NEED_KINDS) and every rail need", () => {
    const kinds = ["asks", "stuck", "limit", "failed", "review", "signin", "room", "heavy", "leftovers", "cap", "message", "task", "quota", "cut_off"];
    const placed = new Set(CASES.map((one) => one.need).filter(Boolean));
    for (const kind of kinds) expect(placed.has(kind), kind).toBe(true);
  });

  it("every rail state has a level", () => {
    const states: RailState[] = ["asks_you", "working", "idle", "done_review", "ended"];
    expect(states.map((state) => attentionLevel({ state }))).toEqual([2, 0, 0, 1, 0]);
  });

  it("a need outranks the state it shows (a failed session that reads Ended is broken)", () => {
    expect(attentionLevel({ need: "failed", state: "ended" })).toBe(3);
    expect(attentionLevel({ need: "review", state: "working" })).toBe(1);
  });
});

describe("the shared count: L2 and L3 only", () => {
  it("counts what waits on you and what broke, never a note", () => {
    expect(([0, 1, 2, 3] as const).map((level) => counted(level))).toEqual([false, false, true, true]);
  });

  it("a list of needs counts its asks and failures, not its reviews", () => {
    const rows = [{ kind: "asks" }, { kind: "review" }, { kind: "failed" }, { kind: "leftovers" }, { kind: "quota", meter: 20, facts: { sessions: 1 } }, { kind: "quota", meter: 4, facts: { sessions: 1 } }];
    expect(rows.filter((row) => counted(noticeLevel(row))).map((row) => row.kind)).toEqual(["asks", "failed", "quota"]);
  });

  it("a count takes the colour of the highest level it holds", () => {
    expect(levelTone(highest([1, 2, 1])).text).toBe("text-warning");
    expect(levelTone(highest([2, 3])).text).toBe("text-danger");
    expect(levelTone(highest([1, 1])).text).toBe("text-muted");
    expect(highest([])).toBe(0);
  });
});

describe("a session's rail item, by its level", () => {
  const level = (over: Partial<InstanceInfo>) => {
    const item = sessionItem(instance(over), () => null, false);
    return attentionLevel({ need: item.need, state: item.state });
  };
  it("working is calm, waiting needs you, an error is broken", () => {
    expect(level({ status: "running" })).toBe(0);
    expect(level({ status: "waiting_user", needs_attention: "waiting_user" })).toBe(2);
    expect(level({ status: "stuck", needs_attention: "stuck" })).toBe(2);
    expect(level({ status: "quota_blocked", needs_attention: "quota_blocked" })).toBe(2);
    expect(level({ status: "failed", needs_attention: "failed" })).toBe(3);
    expect(level({ status: "finished" })).toBe(0);
  });
});

describe("attentionLevel()'s budget (AJ-E3601: 0.05 ms an item)", () => {
  it("places 10,000 items in 500 ms or less", () => {
    const subjects = Array.from({ length: 10_000 }, (_, at) => CASES[at % CASES.length]!);
    const started = performance.now();
    let sum = 0;
    for (const { level: _level, ...subject } of subjects) sum += attentionLevel(subject);
    expect(sum).toBeGreaterThan(0);
    expect(performance.now() - started).toBeLessThanOrEqual(500);
  });
});
