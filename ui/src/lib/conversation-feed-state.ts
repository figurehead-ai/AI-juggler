/**
 * 0.3.5 (ADR-0024) — a followed conversation's state on the page, and
 * the pure steps that change it: a snapshot when a follow starts, each
 * change the backend sends, a page of earlier turns. Turns are kept by
 * their id (`generation:offset`); a change replaces only the turns it
 * names, so every other turn keeps its object (React keeps its row and
 * whatever the reader opened in it). Nothing is ever replaced by an
 * empty list once turns were shown: problems become a `banner`.
 */
import type { ConversationDeltaInfo } from "../types/generated/ConversationDeltaInfo";
import type { ConversationSnapshotInfo } from "../types/generated/ConversationSnapshotInfo";
import type { EarlierPageInfo } from "../types/generated/EarlierPageInfo";
import type { TurnInfo } from "../types/generated/TurnInfo";

/** Before the first turn: why there's nothing (yet). */
export type FeedStatus = "loading" | "shown" | "off" | "none" | "compressed" | "unreadable" | "failed";

export type FeedState = {
  status: FeedStatus;
  /** Why it failed (`failed` only). */
  why: string | null;
  generation: number;
  revision: number;
  /** `stretches` (the Claude app's way) or `folded` (ChatGPT's Codex). */
  layout: string;
  /** Oldest first, by offset. */
  turns: TurnInfo[];
  /** Where the oldest shown turn starts: more before it; null at the start. */
  earlier: number | null;
  loadingEarlier: boolean;
  /** An earlier page passed a long stretch with no turn: still reading back. */
  lookingFurther: boolean;
  /** Said over the turns: `ended`, `gone`, `unreadable`, `retrying`, `earlier`. */
  banner: string | null;
  subscription: number | null;
};

export const INITIAL: FeedState = {
  status: "loading",
  why: null,
  generation: 0,
  revision: 0,
  layout: "stretches",
  turns: [],
  earlier: null,
  loadingEarlier: false,
  lookingFurther: false,
  banner: null,
  subscription: null,
};

/** The most turns a page keeps (older ones go; "Show earlier" brings them). */
export const TURNS_KEPT = 400;

function byOffset(a: TurnInfo, b: TurnInfo): number {
  return a.offset - b.offset;
}

/** `turns` with `upserts` put in: replaced by id, new ones placed by
 * offset (normally at the end); untouched turns keep their objects. */
export function upsert(turns: TurnInfo[], upserts: TurnInfo[]): TurnInfo[] {
  if (upserts.length === 0) return turns;
  const index = new Map(turns.map((turn, at) => [turn.id, at]));
  const next = turns.slice();
  let sorted = true;
  for (const turn of upserts) {
    const at = index.get(turn.id);
    if (at !== undefined) next[at] = turn;
    else {
      const last = next[next.length - 1];
      if (last && turn.offset < last.offset) sorted = false;
      index.set(turn.id, next.length);
      next.push(turn);
    }
  }
  return sorted ? next : next.sort(byOffset);
}

/** At most `TURNS_KEPT`, the latest; `earlier` moves with the cut. */
function bounded(state: FeedState): FeedState {
  if (state.turns.length <= TURNS_KEPT) return state;
  const turns = state.turns.slice(state.turns.length - TURNS_KEPT);
  return { ...state, turns, earlier: turns[0]?.offset ?? state.earlier };
}

/** A follow started (or started again). Same file and the turns meet the
 * ones shown: merged (earlier pages stay); else the snapshot replaces. */
export function applySnapshot(state: FeedState, snapshot: ConversationSnapshotInfo): FeedState {
  if (snapshot.state !== "shown") {
    // Turns shown stay shown; only the setting turned off takes them away.
    if (state.turns.length > 0 && snapshot.state !== "off") return { ...state, banner: snapshot.state, subscription: null };
    return { ...INITIAL, status: snapshot.state as FeedStatus };
  }
  const base = {
    status: "shown" as const,
    why: null,
    generation: snapshot.generation,
    revision: Number(snapshot.revision),
    layout: snapshot.layout,
    subscription: snapshot.subscription,
    banner: snapshot.status,
    loadingEarlier: false,
    lookingFurther: false,
  };
  const last = state.turns[state.turns.length - 1];
  const first = snapshot.turns[0];
  const meets = state.status === "shown" && state.generation === snapshot.generation && last !== undefined && (first === undefined || first.offset <= last.offset);
  if (meets) {
    return bounded({ ...state, ...base, turns: upsert(state.turns, snapshot.turns) });
  }
  return { ...INITIAL, ...base, turns: snapshot.turns, earlier: snapshot.earlier };
}

/** What a change did: the state after it, and whether a change was
 * missed (follow again to catch up). */
export type Applied = { state: FeedState; resync: boolean };

/** One change from the backend. */
export function applyDelta(state: FeedState, delta: ConversationDeltaInfo): Applied {
  if (delta.subscription !== state.subscription) return { state, resync: false };
  const revision = Number(delta.revision);
  if (revision <= state.revision) return { state, resync: false };
  if (delta.status === "off") return { state: { ...INITIAL, status: "off" }, resync: false };
  if (delta.status === "closed") return { state: { ...state, subscription: null }, resync: true };
  if (delta.kind === "reset") {
    return {
      state: { ...state, status: "shown", generation: delta.generation, revision, layout: delta.layout, turns: delta.upserts, earlier: delta.earlier, banner: delta.status, loadingEarlier: false, lookingFurther: false },
      resync: false,
    };
  }
  if (delta.generation !== state.generation || revision !== state.revision + 1) return { state, resync: true };
  let turns = upsert(state.turns, delta.upserts);
  let earlier = state.earlier;
  if (delta.dropped_before != null) {
    const cut = delta.dropped_before;
    const kept = turns.filter((turn) => turn.offset >= cut);
    if (kept.length > 0 && kept.length < turns.length) {
      turns = kept;
      earlier = cut;
    }
  }
  return { state: bounded({ ...state, revision, layout: delta.layout, turns, earlier, banner: delta.status }), resync: false };
}

/** A page of earlier turns. */
export function applyEarlier(state: FeedState, page: EarlierPageInfo): FeedState {
  if (page.state !== "shown") return { ...state, loadingEarlier: false, lookingFurther: false, banner: page.state === "gone" ? state.banner : "earlier" };
  if (page.generation !== state.generation) return { ...state, loadingEarlier: false, lookingFurther: false };
  const known = new Set(state.turns.map((turn) => turn.id));
  const older = page.turns.filter((turn) => !known.has(turn.id));
  return {
    ...state,
    turns: older.length > 0 ? [...older, ...state.turns].sort(byOffset) : state.turns,
    earlier: page.earlier,
    loadingEarlier: false,
    lookingFurther: page.turns.length === 0 && page.earlier !== null,
  };
}
