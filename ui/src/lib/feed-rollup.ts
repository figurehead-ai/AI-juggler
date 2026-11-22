/**
 * D-118 (AJ-E3702 R4, R5) — the feed's items as cards, by the keys the
 * read model gives each item (`features/feed/items.rs`):
 *
 * 1. **One thing, one card.** Items with the same `coalesce` key (one
 *    session's ending and its finished notice; a crash and its restart)
 *    are one card, at the newest one's place, showing its newest state.
 * 2. **Rollups.** Things with the same `rollup` key within its window are
 *    one card ("finished 4 sessions in web-app"): the key's count, up to 3
 *    sample names and the newest time; it opens to the full list. Another
 *    kind of item of the same App or pool (`scope`) starts the window
 *    again, so a crash is never hidden inside a rollup.
 * 3. **Nothing broken is folded.** The level is part of every rollup key,
 *    and a level-3 thing never joins one, whatever its key says.
 *
 * Folded over every page loaded so far, so a rollup that straddles a page
 * boundary is still one card. Pure: the tests hold it to these rules.
 */
import type { FeedItemInfo } from "./ipc/feed";

/** One thing: its items, newest first (`head` is the newest). */
export interface FeedThing {
  key: string;
  head: FeedItemInfo;
  items: FeedItemInfo[];
}

/** One card: one thing, or a rollup of several. */
export interface FeedCard {
  key: string;
  /** The things, newest first (one, unless rolled up). */
  things: FeedThing[];
  /** The newest thing's newest item. */
  head: FeedItemInfo;
  /** The highest level inside. */
  level: number;
  /** A rollup (two things or more). */
  rolled: boolean;
}

/** The things, in the feed's order (each where its newest item is). */
function thingsOf(items: readonly FeedItemInfo[]): FeedThing[] {
  const things: FeedThing[] = [];
  const at = new Map<string, FeedThing>();
  for (const item of items) {
    const key = item.coalesce ?? `id:${item.id}`;
    const thing = at.get(key);
    if (thing) {
      if (!thing.items.some((one) => one.id === item.id)) thing.items.push(item);
      continue;
    }
    const made = { key, head: item, items: [item] };
    at.set(key, made);
    things.push(made);
  }
  return things;
}

/** A thing's state: its newest item, unless a restart follows a crash
 * (the crash is what the card says; the restart is its fact). */
export function stateOf(thing: FeedThing): FeedItemInfo {
  const crash = thing.items.find((item) => item.kind === "failed" || item.kind === "crashed");
  return crash && thing.head.kind === "restarted" ? crash : thing.head;
}

/** The restart that followed a thing's crash, when there was one. */
export function restartOf(thing: FeedThing): FeedItemInfo | null {
  return thing.items.find((item) => item.kind === "restarted") ?? null;
}

function levelOf(thing: FeedThing): number {
  return Math.max(...thing.items.map((item) => item.level));
}

/** The cards: things coalesced, then rolled up by key and window. */
export function cardsOf(items: readonly FeedItemInfo[]): FeedCard[] {
  const cards: FeedCard[] = [];
  // Rollup key → the card still open for it, and the scope it belongs to.
  const open = new Map<string, { card: FeedCard; scope: string | null }>();
  for (const thing of thingsOf(items)) {
    const state = stateOf(thing);
    const level = levelOf(thing);
    const key = level >= 3 ? null : state.rollup;
    // Another kind of item of the same App or pool (a crash among its
    // finishes) ends that App's open rollups of other kinds: their windows
    // start again after it.
    if (state.scope !== null) {
      for (const [rollup, entry] of open) if (entry.scope === state.scope && rollup !== key && entry.card.head.kind !== state.kind) open.delete(rollup);
    }
    const entry = key ? open.get(key) : undefined;
    if (entry && entry.card.head.at_ms - state.at_ms <= state.rollup_window_ms) {
      entry.card.things.push(thing);
      entry.card.rolled = true;
      entry.card.level = Math.max(entry.card.level, level);
      continue;
    }
    const card: FeedCard = { key: thing.key, things: [thing], head: state, level, rolled: false };
    cards.push(card);
    if (key) open.set(key, { card, scope: state.scope });
  }
  return cards;
}

/** The away boundary is shown when the last look is this long ago. */
export const AWAY_MS = 2 * 3_600_000;

/** When the user last looked (`sinceMs`, `summary_seen_at_ms`) and when
 * they came back (`backMs`, the read of it as Home opened). */
export interface LastLook {
  sinceMs: number;
  backMs: number;
}

/** The away divider and digest (R5): where the divider sits in the
 * stream, and what happened while away. */
export interface Away {
  /** The divider goes before this stream card (stream.length: after them all). */
  index: number;
  sinceMs: number;
  /** Each thing's kind while away (the digest counts them). */
  kinds: string[];
  /** The three most important, broken first, then the newest. */
  top: FeedThing[];
  /** Every thing while away (Show all). */
  all: FeedThing[];
}

/**
 * The feed split at the last look (R5, proposal §3.6): what happened
 * while away (after the last look, up to the return) is rolled into the
 * digest under the divider and is not a stream card as well; above the
 * divider only what came since the return; under it, what is older than
 * the last look. A card goes whole by its newest thing, so no item is
 * ever both a card and a digest line. No away (`away: null`, every card
 * in the stream) when the last look is under 2 h before the return, was
 * never kept, nothing happened while away, or its boundary isn't loaded
 * yet (more pages follow; the auto pages reach it).
 */
export function splitAway(cards: readonly FeedCard[], look: LastLook | null, more: boolean): { stream: FeedCard[]; away: Away | null } {
  const none = { stream: [...cards], away: null };
  if (look === null || look.backMs - look.sinceMs < AWAY_MS) return none;
  if (more && !cards.some((card) => card.head.at_ms <= look.sinceMs)) return none;
  const since = cards.filter((card) => card.head.at_ms > look.backMs);
  const gone = cards.filter((card) => card.head.at_ms <= look.backMs && card.head.at_ms > look.sinceMs);
  const older = cards.filter((card) => card.head.at_ms <= look.sinceMs);
  const all = gone.flatMap((card) => card.things);
  if (all.length === 0) return none;
  const top = [...all].sort((a, b) => levelOf(b) - levelOf(a) || stateOf(b).at_ms - stateOf(a).at_ms).slice(0, 3);
  return {
    stream: [...since, ...older],
    away: { index: since.length, sinceMs: look.sinceMs, kinds: all.map((thing) => stateOf(thing).kind), top, all },
  };
}

/** Day separators: the index of each card that starts a new day. */
export function dayStarts(cards: readonly FeedCard[]): Set<number> {
  const starts = new Set<number>();
  let last = "";
  cards.forEach((card, index) => {
    const day = new Date(card.head.at_ms).toDateString();
    if (day !== last) starts.add(index);
    last = day;
  });
  return starts;
}
