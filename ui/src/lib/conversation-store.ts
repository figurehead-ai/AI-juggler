/**
 * 0.3.5 (ADR-0024) — the conversations pages follow, one store per
 * conversation, read through `useSyncExternalStore` (U4). A page that
 * shows one connects; the store follows it on the backend (a snapshot,
 * then changes on its channel) while any page shows it, and unfollows
 * when none does. What it read stays in memory for the last few
 * conversations (`KEPT`), so going back to one shows it at once while
 * the follow catches up behind. Never on disk.
 *
 * Every half minute the store tells the backend which follows it still
 * has (a page that went away without saying so is let go there); a
 * follow the backend ended is started again. A change missed (a gap in
 * revisions) starts the follow again too; the snapshot merges with what
 * is shown. A follow that fails after turns were shown keeps them and
 * tries again, a little later each time.
 */
import { conversationsEarlier, conversationsKeep, conversationsUnfollow, type ConversationDeltaInfo, type ConversationSnapshotInfo } from "./ipc/conversations";
import { errorText } from "./bridge";
import { LAST_MESSAGES_EVENT } from "./last-messages";
import { applyDelta, applyEarlier, applySnapshot, INITIAL, type FeedState } from "./conversation-feed-state";

/** How a conversation is followed: its key, and the call that starts it. */
export type FeedSource = {
  key: string;
  follow: (max: number, onDelta: (delta: ConversationDeltaInfo) => void) => Promise<ConversationSnapshotInfo>;
};

/** Conversations whose turns stay in memory with no page showing them. */
export const KEPT = 8;
/** Turns a follow starts with, and a page of earlier ones. */
export const PAGE = 30;
/** How often the store says which follows it still has. */
export const KEEP_MS = 30_000;
/** Earlier pages read in a row past stretches with no turn. */
const FURTHER_MAX = 6;

type Entry = {
  state: FeedState;
  listeners: Set<() => void>;
  source: FeedSource | null;
  /** Pages showing it now. */
  shown: number;
  /** Bumped by every follow started: a stale answer is dropped. */
  attempt: number;
  retryMs: number;
  retry: ReturnType<typeof setTimeout> | null;
  used: number;
};

const entries = new Map<string, Entry>();
let keeper: ReturnType<typeof setInterval> | null = null;
let clock = 0;

function entry(key: string): Entry {
  let found = entries.get(key);
  if (!found) {
    found = { state: INITIAL, listeners: new Set(), source: null, shown: 0, attempt: 0, retryMs: 2_000, retry: null, used: 0 };
    entries.set(key, found);
  }
  return found;
}

function set(key: string, next: FeedState): void {
  const found = entry(key);
  if (found.state === next) return;
  found.state = next;
  for (const listener of found.listeners) listener();
}

/** Subscribes to `key`'s state (useSyncExternalStore). */
export function subscribe(key: string, listener: () => void): () => void {
  const found = entry(key);
  found.listeners.add(listener);
  return () => found.listeners.delete(listener);
}

/** `key`'s state now. */
export function stateOf(key: string): FeedState {
  return entries.get(key)?.state ?? INITIAL;
}

function unfollow(found: Entry): void {
  const subscription = found.state.subscription;
  if (subscription !== null) void conversationsUnfollow(subscription).catch(() => undefined);
}

function start(key: string): void {
  const found = entry(key);
  const source = found.source;
  if (!source || found.shown === 0) return;
  if (found.retry) clearTimeout(found.retry);
  found.retry = null;
  const attempt = ++found.attempt;
  const onDelta = (delta: ConversationDeltaInfo) => {
    if (found.attempt !== attempt) return;
    const applied = applyDelta(found.state, delta);
    set(key, applied.state);
    if (applied.resync) {
      unfollow(found);
      start(key);
    }
  };
  source.follow(PAGE, onDelta).then(
    (snapshot) => {
      if (found.attempt !== attempt) {
        if (snapshot.subscription !== null) void conversationsUnfollow(snapshot.subscription).catch(() => undefined);
        return;
      }
      found.retryMs = 2_000;
      set(key, applySnapshot(found.state, snapshot));
      watchKept();
    },
    (cause) => {
      if (found.attempt !== attempt) return;
      const now = found.state;
      set(key, now.turns.length > 0 ? { ...now, banner: "retrying", subscription: null } : { ...INITIAL, status: "failed", why: errorText(cause) });
      if (now.turns.length > 0 && found.shown > 0) {
        found.retry = setTimeout(() => start(key), found.retryMs);
        found.retryMs = Math.min(found.retryMs * 2, 30_000);
      }
    },
  );
}

/** A page shows `source`'s conversation: followed while any page does. */
export function connect(source: FeedSource): void {
  const found = entry(source.key);
  found.source = source;
  found.used = ++clock;
  found.shown += 1;
  if (found.shown === 1) start(source.key);
  evict();
}

/** A page stopped showing `key`'s conversation. */
export function disconnect(key: string): void {
  const found = entries.get(key);
  if (!found || found.shown === 0) return;
  found.shown -= 1;
  if (found.shown > 0) return;
  found.attempt += 1;
  if (found.retry) clearTimeout(found.retry);
  found.retry = null;
  unfollow(found);
  set(key, { ...found.state, subscription: null, loadingEarlier: false, lookingFurther: false });
}

/** Loads the turns before the oldest shown; reads on past stretches
 * with none (a long run of tool output), a few pages at most. */
export async function showEarlier(key: string): Promise<void> {
  const found = entries.get(key);
  if (!found) return;
  for (let page = 0; page < FURTHER_MAX; page += 1) {
    const now = found.state;
    if (now.earlier === null || now.subscription === null || (page === 0 && now.loadingEarlier)) return;
    set(key, { ...now, loadingEarlier: true });
    try {
      const answer = await conversationsEarlier(now.subscription, now.earlier, PAGE);
      set(key, applyEarlier(found.state, answer));
    } catch {
      set(key, { ...found.state, loadingEarlier: false, lookingFurther: false, banner: "earlier" });
      return;
    }
    if (!found.state.lookingFurther) return;
  }
  set(key, { ...found.state, lookingFurther: false });
}

/** The follows the store has, renewed; those the backend ended, again. */
async function keepFollows(): Promise<void> {
  const held = [...entries.entries()].filter(([, found]) => found.shown > 0 && found.state.subscription !== null);
  if (held.length === 0) {
    if (keeper) clearInterval(keeper);
    keeper = null;
    return;
  }
  try {
    const gone = new Set(await conversationsKeep(held.map(([, found]) => found.state.subscription as number)));
    for (const [key, found] of held) if (gone.has(found.state.subscription as number)) start(key);
  } catch {
    // Next time.
  }
}

function watchKept(): void {
  if (keeper) return;
  keeper = setInterval(() => void keepFollows(), KEEP_MS);
}

/** Past `KEPT`, the least recently shown conversations no page shows go. */
function evict(): void {
  const idle = [...entries.entries()].filter(([, found]) => found.shown === 0 && found.listeners.size === 0);
  if (entries.size <= KEPT) return;
  idle.sort((a, b) => a[1].used - b[1].used);
  for (const [key] of idle.slice(0, entries.size - KEPT)) entries.delete(key);
}

// Turned on or off in Settings: every conversation read again (or off).
if (typeof window !== "undefined") {
  window.addEventListener(LAST_MESSAGES_EVENT, () => {
    for (const [key, found] of entries) {
      unfollow(found);
      set(key, INITIAL);
      start(key);
    }
  });
}

/** Tests: forget everything. */
export function resetConversationStore(): void {
  for (const found of entries.values()) if (found.retry) clearTimeout(found.retry);
  entries.clear();
  if (keeper) clearInterval(keeper);
  keeper = null;
}
