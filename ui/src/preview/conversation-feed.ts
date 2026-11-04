/**
 * 0.3.5 (ADR-0024) — the preview's conversation feed: a fixture's turns
 * (`__AIJ_FIXTURE__.last_messages`, by conversation id, or a subagent's
 * seeded turns) answered as the backend's follow does, paged back by
 * turn index, and a seam that plays a live feed for the gallery and the
 * specs: `window.dispatchEvent(new CustomEvent(PREVIEW_FEED, { detail }))`
 * with `{ conversationId, turns }` appends those turns (or grows the
 * last one when its first turn has the same speaker and `grow` is set),
 * and `{ conversationId, status }` says a status. Preview only.
 */
import type { ConversationDeltaInfo } from "../types/generated/ConversationDeltaInfo";
import type { ConversationSnapshotInfo } from "../types/generated/ConversationSnapshotInfo";
import type { EarlierPageInfo } from "../types/generated/EarlierPageInfo";
import type { TurnBlockInfo } from "../types/generated/TurnBlockInfo";
import type { TurnInfo } from "../types/generated/TurnInfo";

/** The event a spec sends to play the feed. */
export const PREVIEW_FEED = "aij:preview-feed";

/** A fixture's block: the backend's shape, or part of it. */
export type SeededBlock = Partial<TurnBlockInfo> & { kind: string };
/** A fixture's turn: the backend's shape, or speaker and text only. */
export type SeededTurn = Partial<Omit<TurnInfo, "blocks">> & { speaker: string; text?: string; blocks?: SeededBlock[] };

/** A block with every field the wire has. */
export function blockOf(block: SeededBlock): TurnBlockInfo {
  return {
    text: "",
    steps: [],
    hidden: 0,
    calls: [],
    thoughts: [],
    title: null,
    title_running: null,
    after_thinking: false,
    message_id: null,
    at_ms: null,
    ...block,
    final_answer: block.final_answer ?? false,
  };
}

/** A seeded turn as the wire has it, at `index` (its id and offset). */
export function seedTurn(turn: SeededTurn, index: number, open = false): TurnInfo {
  const { text, blocks, ...rest } = turn;
  return {
    id: `p:${index}`,
    revision: 1,
    offset: index,
    open,
    blocks_hidden: 0,
    at_ms: null,
    model: null,
    redacted: 0,
    ...rest,
    blocks: (blocks ?? [{ kind: "text", text: text ?? "" }]).map(blockOf),
  };
}

type Live = { turns: TurnInfo[]; revision: number; layout: string; listeners: Map<number, (delta: ConversationDeltaInfo) => void> };

const live = new Map<string, Live>();
let nextSubscription = 0;
const subscriptions = new Map<number, string>();

/** Nothing to follow, and why. */
export function previewWithout(state: string): ConversationSnapshotInfo {
  return { state, subscription: null, generation: 0, revision: 0, layout: "stretches", turns: [], earlier: null, status: null, skipped: 0 };
}

/** Follows `key`'s seeded turns: the latest `max` now, then what a spec plays. */
export function previewFollow(key: string, seeded: SeededTurn[], layout: string, max: number, onDelta: (delta: ConversationDeltaInfo) => void): ConversationSnapshotInfo {
  let feed = live.get(key);
  if (!feed) {
    feed = { turns: seeded.map((turn, index) => seedTurn(turn, index)), revision: 1, layout, listeners: new Map() };
    live.set(key, feed);
  }
  const id = ++nextSubscription;
  subscriptions.set(id, key);
  feed.listeners.set(id, onDelta);
  const start = Math.max(0, feed.turns.length - Math.min(Math.max(max, 1), 60));
  return { state: "shown", subscription: id, generation: 1, revision: feed.revision, layout: feed.layout, turns: feed.turns.slice(start), earlier: start > 0 ? start : null, status: null, skipped: 0 };
}

export function previewUnfollow(subscription: number): void {
  const key = subscriptions.get(subscription);
  subscriptions.delete(subscription);
  if (key) live.get(key)?.listeners.delete(subscription);
}

export function previewKeep(asked: number[]): number[] {
  return asked.filter((id) => !subscriptions.has(id));
}

export function previewEarlier(subscription: number, before: number, max: number): EarlierPageInfo {
  const key = subscriptions.get(subscription);
  const feed = key ? live.get(key) : undefined;
  if (!feed) return { state: "gone", generation: 0, turns: [], earlier: null };
  const end = Math.min(before, feed.turns.length);
  const start = Math.max(0, end - Math.min(Math.max(max, 1), 60));
  return { state: "shown", generation: 1, turns: feed.turns.slice(start, end), earlier: start > 0 ? start : null };
}

type Play = { conversationId: string; turns?: SeededTurn[]; grow?: boolean; status?: string | null };

function play(detail: Play): void {
  for (const [key, feed] of live) {
    if (!key.endsWith(`\u0000${detail.conversationId}`)) continue;
    const upserts: TurnInfo[] = [];
    for (const seeded of detail.turns ?? []) {
      const last = feed.turns[feed.turns.length - 1];
      if (detail.grow && last && last.speaker === seeded.speaker) {
        const grown: TurnInfo = { ...last, revision: last.revision + 1, blocks: [...last.blocks, ...seedTurn(seeded, 0).blocks] };
        feed.turns[feed.turns.length - 1] = grown;
        upserts.push(grown);
      } else {
        const turn = seedTurn(seeded, feed.turns.length, true);
        if (last?.open) {
          const closed = { ...last, open: false };
          feed.turns[feed.turns.length - 1] = closed;
          upserts.push(closed);
        }
        feed.turns.push(turn);
        upserts.push(turn);
      }
    }
    feed.revision += 1;
    const delta: Omit<ConversationDeltaInfo, "subscription"> = {
      generation: 1,
      revision: feed.revision,
      kind: detail.turns ? "append" : "state",
      upserts,
      dropped_before: null,
      earlier: null,
      layout: feed.layout,
      status: detail.status ?? null,
      skipped: 0,
    };
    for (const [subscription, listener] of feed.listeners) listener({ ...delta, subscription });
  }
}

if (typeof window !== "undefined") {
  window.addEventListener(PREVIEW_FEED, (event) => play((event as CustomEvent<Play>).detail));
}
