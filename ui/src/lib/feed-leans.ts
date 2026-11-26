/**
 * AJ-E3713 R3 — More / Less like this: the leans a reader gave posts,
 * kept by the backend as `preference` memories through ADR-0037's write
 * gate (`feed_lean`, `feed_leans`, `feed_unlean`), one shared read for
 * every post, and what they do to the feed (`rank`):
 *
 * - **The lean behind a post** is the most specific one that matches it:
 *   its kind of event (group), then its App and its project.
 * - **Ranking.** Within a day, posts the reader wants more of rise above
 *   the rest.
 * - **What gets posted.** A quiet post (level 0–1) under a "less" folds
 *   into one line at its section's end ("2 posts shown less · Show"); a
 *   post that broke or needs the reader (level 2–3) is never folded or
 *   moved, so ranking never buries something urgent.
 * - A post the reader just leaned stays where it is until the feed is
 *   read again, so its "Why am I seeing this?" and Undo are at hand.
 *
 * In the preview (no backend) the leans live in memory, so the galleries
 * and tests exercise the same flow.
 */
import { useSyncExternalStore } from "react";
import { call, liveBackend as hasBackend } from "./ipc/transport";
import { groupOf, GROUPS } from "./feed-chips";
import { threadOf, type FeedCard } from "./feed-rollup";
import type { FeedItemInfo } from "./ipc/feed";
import type { FeedLeanAsk } from "../types/generated/FeedLeanAsk";
import type { FeedLeanInfo } from "../types/generated/FeedLeanInfo";

export type { FeedLeanAsk, FeedLeanInfo };

/** The preview's leans (`__AIJ_FEED_LEANS__` seeds them). */
let previewLeans: FeedLeanInfo[] | null = null;

function previewKept(): FeedLeanInfo[] {
  if (previewLeans === null) {
    const seeded: unknown = Reflect.get(window, "__AIJ_FEED_LEANS__");
    previewLeans = Array.isArray(seeded) ? seeded.filter((one): one is FeedLeanInfo => typeof one === "object" && one !== null && typeof Reflect.get(one, "key") === "string") : [];
  }
  return previewLeans;
}

/** The preview's subject key, as the backend's (one per scope). */
function previewKey(ask: FeedLeanAsk): string {
  return `feed:${ask.group}:${ask.folder ?? "-"}:${ask.project ?? "-"}`;
}

/** Keeps a lean; `null` when the gate refused it. */
async function feedLean(ask: FeedLeanAsk): Promise<FeedLeanInfo | null> {
  if (!hasBackend) {
    const key = previewKey(ask);
    const kept: FeedLeanInfo = { key, lean: ask.lean, group: ask.group, folder: ask.folder ?? null, project: ask.project ?? null, about: ask.about ?? null, at_ms: Date.now() };
    previewLeans = [kept, ...previewKept().filter((one) => one.key !== key)];
    return kept;
  }
  return call("feed_lean", { ask });
}

/** Every lean kept, newest first. */
export async function feedLeans(): Promise<FeedLeanInfo[]> {
  if (!hasBackend) return [...previewKept()];
  return call("feed_leans", {});
}

/** Undo: forgets a lean. */
async function feedUnlean(key: string): Promise<void> {
  if (!hasBackend) {
    previewLeans = previewKept().filter((one) => one.key !== key);
    return;
  }
  await call("feed_unlean", { key });
}

/** Test seam: forget the preview's leans. */
export function forgetFeedLeans(): void {
  previewLeans = null;
  leans = [];
  read = false;
  emit();
}

let leans: FeedLeanInfo[] = [];
let read = false;
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of listeners) listener();
}

async function readLeans(): Promise<void> {
  try {
    leans = await feedLeans();
  } catch {
    // No memory store (or it failed): the feed shows unranked.
    leans = [];
  }
  read = true;
  emit();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (!read) {
    read = true;
    void readLeans();
  }
  return () => listeners.delete(listener);
}

/** The leans kept, read once and shared by every post. */
export function useFeedLeans(): FeedLeanInfo[] {
  return useSyncExternalStore(subscribe, () => leans);
}

/** Keeps a lean and tells every post; the lean, or `null` when refused. */
export async function lean(ask: FeedLeanAsk): Promise<FeedLeanInfo | null> {
  const kept = await feedLean(ask);
  if (kept) leans = [kept, ...leans.filter((one) => one.key !== kept.key)];
  emit();
  return kept;
}

/** Undoes a lean and tells every post. */
export async function unlean(key: string): Promise<void> {
  await feedUnlean(key);
  leans = leans.filter((one) => one.key !== key);
  emit();
}

/** A post's scope for a lean: its group, its App, its project. */
export function scopeOf(item: FeedItemInfo): { group: string; folder: string | null; project: string | null } {
  const folder = item.author.kind === "app" || item.author.kind === "account" ? item.author.folder : null;
  return { group: groupOf(item.kind), folder: folder ?? null, project: item.facts.project_id ?? item.target.project_id ?? null };
}

/** The item a card's post is about (its gesture bar leans by it): a
 * thread's first item, or a rollup's newest. */
function postOf(card: FeedCard): FeedItemInfo {
  return card.rolled ? card.head : threadOf(card.things[0]!).post;
}

/** How specific a lean is (a lean on the App and project wins). */
function specificity(one: FeedLeanInfo): number {
  return (one.folder ? 2 : 0) + (one.project ? 1 : 0);
}

/** The lean behind a post: the most specific that matches it. */
export function leanFor(item: FeedItemInfo, kept: readonly FeedLeanInfo[]): FeedLeanInfo | null {
  const scope = scopeOf(item);
  let best: FeedLeanInfo | null = null;
  for (const one of kept) {
    if (one.group !== scope.group) continue;
    if (one.folder && one.folder !== scope.folder) continue;
    if (one.project && one.project !== scope.project) continue;
    if (!best || specificity(one) > specificity(best)) best = one;
  }
  return best;
}

/** The words of a lean's group ("switch"). */
function groupWords(group: string): string {
  return GROUPS[group] ?? "these";
}

/** "You'll see fewer switch posts." and why. */
export function whyWords(one: FeedLeanInfo): { head: string; why: string } {
  const posts = `${groupWords(one.group)} posts`;
  const where = one.about ? ` ${one.project ? "in" : "from"} ${one.about}` : one.project ? " in this project" : one.folder ? " from this app" : "";
  const choice = one.lean === "more" ? "More like this" : "Less like this";
  return {
    head: one.lean === "more" ? `You'll see more ${posts} first.` : `You'll see fewer ${posts}.`,
    why: `You chose ${choice} for ${posts}${where}.`,
  };
}

/** The level from which a post is never folded or sunk. */
const URGENT = 2;

/** One run of posts that share a section (a day, "This morning"):
 * ranked, and the quiet ones a "less" folded out of it. */
export interface Section {
  key: string;
  cards: FeedCard[];
  folded: FeedCard[];
}

/** The day a card is in (its newest item's). */
function dayKey(card: FeedCard): string {
  return new Date(card.head.at_ms).toDateString();
}

/**
 * Ranks `cards` (newest first, as the stream has them) by the leans,
 * section by section (`keyOf`, the day by default): "more" first, then
 * the rest, each in time order; a quiet post (level 0–1) under a "less"
 * is folded out of its section, unless it is `pinned` (just leaned: it
 * stays in place with its Undo). A post at level 2 or 3 is never moved
 * or folded.
 */
export function rank(cards: readonly FeedCard[], kept: readonly FeedLeanInfo[], pinned: ReadonlySet<string> = new Set(), keyOf: (card: FeedCard) => string = dayKey): Section[] {
  const sections: Section[] = [];
  for (const card of cards) {
    const key = keyOf(card);
    const last = sections[sections.length - 1];
    if (last && last.key === key) last.cards.push(card);
    else sections.push({ key, cards: [card], folded: [] });
  }
  if (kept.length === 0) return sections;
  const tier = (card: FeedCard) => {
    if (pinned.has(card.key) || card.level >= URGENT) return 1;
    const one = leanFor(postOf(card), kept);
    return one?.lean === "more" ? 0 : one?.lean === "less" ? 2 : 1;
  };
  return sections.map((section) => {
    const tiers: FeedCard[][] = [[], [], []];
    for (const card of section.cards) tiers[tier(card)]!.push(card);
    return { key: section.key, cards: [...tiers[0]!, ...tiers[1]!], folded: tiers[2]! };
  });
}
