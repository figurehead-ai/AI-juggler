/**
 * 0.3.5 (ADR-0024) vitest — the pure steps of a followed conversation:
 * a change replaces only the turns it names (the others keep their
 * objects), new turns go by offset, a stale change is dropped, a missed
 * one asks to follow again, a reset swaps all, earlier pages merge by
 * id, a bound lets old turns go, and nothing shown is ever replaced by
 * an empty list.
 */
import { describe, expect, it } from "vitest";
import type { ConversationDeltaInfo } from "../types/generated/ConversationDeltaInfo";
import type { ConversationSnapshotInfo } from "../types/generated/ConversationSnapshotInfo";
import type { TurnInfo } from "../types/generated/TurnInfo";
import { applyDelta, applyEarlier, applySnapshot, INITIAL, TURNS_KEPT, upsert } from "./conversation-feed-state";

const turn = (offset: number, words = `turn ${offset}`, revision = 1, generation = 3): TurnInfo => ({
  id: `${generation}:${offset}`,
  revision,
  offset,
  open: false,
  speaker: offset % 2 ? "assistant" : "you",
  blocks: [{ kind: "text", text: words, steps: [], hidden: 0, calls: [], thoughts: [], title: null, title_running: null, after_thinking: false, message_id: null, at_ms: null, final_answer: false }],
  blocks_hidden: 0,
  at_ms: null,
  model: null,
  redacted: 0,
});

const snapshot = (turns: TurnInfo[], extra: Partial<ConversationSnapshotInfo> = {}): ConversationSnapshotInfo => ({
  state: "shown",
  subscription: 1,
  generation: 3,
  revision: 5,
  layout: "stretches",
  turns,
  earlier: turns[0]?.offset ?? null,
  status: null,
  skipped: 0,
  ...extra,
});

const delta = (extra: Partial<ConversationDeltaInfo>): ConversationDeltaInfo => ({
  subscription: 1,
  generation: 3,
  revision: 6,
  kind: "append",
  upserts: [],
  dropped_before: null,
  earlier: null,
  layout: "stretches",
  status: null,
  skipped: 0,
  ...extra,
});

const shown = applySnapshot(INITIAL, snapshot([turn(10), turn(20), turn(30)]));

describe("0.3.5 a followed conversation's state", () => {
  it("a change replaces only the turns it names; the others keep their objects", () => {
    const grown = turn(30, "turn 30, grown", 2);
    const { state, resync } = applyDelta(shown, delta({ upserts: [grown, turn(40)] }));
    expect(resync).toBe(false);
    expect(state.revision).toBe(6);
    expect(state.turns.map((one) => one.offset)).toEqual([10, 20, 30, 40]);
    expect(state.turns[0]).toBe(shown.turns[0]);
    expect(state.turns[1]).toBe(shown.turns[1]);
    expect(state.turns[2]).toBe(grown);
  });

  it("a turn new by offset goes in its place", () => {
    expect(upsert([turn(10), turn(30)], [turn(20)]).map((one) => one.offset)).toEqual([10, 20, 30]);
  });

  it("a stale change is dropped; a missed one, or another file's, asks to follow again", () => {
    expect(applyDelta(shown, delta({ revision: 5, upserts: [turn(99)] }))).toEqual({ state: shown, resync: false });
    expect(applyDelta(shown, delta({ revision: 8 })).resync).toBe(true);
    expect(applyDelta(shown, delta({ generation: 4 })).resync).toBe(true);
    expect(applyDelta(shown, delta({ subscription: 2, upserts: [turn(99)] })).state).toBe(shown);
  });

  it("a reset swaps everything at once", () => {
    const { state } = applyDelta(shown, delta({ kind: "reset", generation: 9, upserts: [turn(5, "new file", 1, 9)], earlier: null }));
    expect(state.generation).toBe(9);
    expect(state.turns.map((one) => one.id)).toEqual(["9:5"]);
    expect(state.earlier).toBeNull();
  });

  it("turns let go by the feed's bound go here too, and earlier ones are a page away", () => {
    const { state } = applyDelta(shown, delta({ dropped_before: 20 }));
    expect(state.turns.map((one) => one.offset)).toEqual([20, 30]);
    expect(state.earlier).toBe(20);
  });

  it("a status is a line over the turns, which stay", () => {
    const { state } = applyDelta(shown, delta({ kind: "state", status: "gone" }));
    expect(state.banner).toBe("gone");
    expect(state.turns).toBe(shown.turns);
    const failed = applySnapshot(shown, { ...snapshot([]), state: "unreadable", subscription: null });
    expect(failed.turns).toBe(shown.turns);
    expect(failed.banner).toBe("unreadable");
  });

  it("only the setting turned off takes the turns away", () => {
    expect(applyDelta(shown, delta({ kind: "state", status: "off" })).state.status).toBe("off");
    expect(applySnapshot(shown, { ...snapshot([]), state: "off", subscription: null }).turns).toEqual([]);
  });

  it("a follow started again merges with what is shown when they meet, else replaces", () => {
    const withOlder = applyEarlier(shown, { state: "shown", generation: 3, turns: [turn(1), turn(2)], earlier: null });
    const again = applySnapshot(withOlder, snapshot([turn(30, "turn 30", 2), turn(40)], { subscription: 2, revision: 9 }));
    expect(again.turns.map((one) => one.offset)).toEqual([1, 2, 10, 20, 30, 40]);
    expect(again.earlier).toBeNull();
    expect(again.subscription).toBe(2);
    const apart = applySnapshot(withOlder, snapshot([turn(500), turn(600)], { subscription: 2 }));
    expect(apart.turns.map((one) => one.offset)).toEqual([500, 600]);
  });

  it("an earlier page merges by id; one of another file is dropped; an empty one with a cursor reads on", () => {
    const page = applyEarlier(shown, { state: "shown", generation: 3, turns: [turn(5), turn(10)], earlier: 2 });
    expect(page.turns.map((one) => one.offset)).toEqual([5, 10, 20, 30]);
    expect(page.turns[1]).toBe(shown.turns[0]);
    expect(page.earlier).toBe(2);
    expect(applyEarlier(shown, { state: "shown", generation: 4, turns: [turn(5, "x", 1, 4)], earlier: null }).turns).toBe(shown.turns);
    const further = applyEarlier(shown, { state: "shown", generation: 3, turns: [], earlier: 4 });
    expect(further.lookingFurther).toBe(true);
    expect(further.turns).toBe(shown.turns);
  });

  it("keeps at most its bound, the latest", () => {
    const many = Array.from({ length: TURNS_KEPT + 50 }, (_, index) => turn(index + 1));
    const state = applySnapshot(shown, snapshot(many, { revision: 9 }));
    const { state: next } = applyDelta(state, delta({ revision: 10, upserts: [turn(5_000)] }));
    expect(next.turns.length).toBe(TURNS_KEPT);
    expect(next.turns[next.turns.length - 1]!.offset).toBe(5_000);
    expect(next.earlier).toBe(next.turns[0]!.offset);
  });
});
