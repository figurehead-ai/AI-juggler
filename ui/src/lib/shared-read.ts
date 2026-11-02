/**
 * 0.3.4 — one backend read that several screens show (every account's
 * runway, the monthly caps, a project's checkout): its last answer kept
 * in memory, so a screen opened again paints it in its first frame and
 * reads again behind it (stale while it revalidates, the page-switching
 * report: "shouldn't our app cache certain things … with proper
 * invalidation"). Reads asked for at once share one call, and the
 * screens that keep it current share one timer.
 *
 * What makes an answer old stays each caller's, as before the store: a
 * screen reads again when it opens (`maxAgeMs`, 0 by default: always),
 * on its poll (one timer for all, the shortest asked), when its own
 * events say so, and after a change the user made (`refresh`). A failed
 * read keeps the last answer. `forgetSharedReads` drops every one.
 */
import { useEffect, useSyncExternalStore } from "react";

export type SharedRead<T> = {
  /** The last answer, if any. */
  peek: () => T | undefined;
  /** Reads, unless an answer younger than `maxAgeMs` is kept or a read
   * is under way (it waits on that one). */
  load: (maxAgeMs?: number) => Promise<T | undefined>;
  /** Reads now, whatever is kept or under way (something changed). */
  refresh: () => Promise<T | undefined>;
  subscribe: (listener: () => void) => () => void;
  /** Joins the shared timer (read every `everyMs` while anyone is in it). */
  poll: (everyMs: number) => () => void;
  /** Forgets the answer and stops the timer. */
  reset: () => void;
};

const every = new Set<{ reset: () => void }>();

/** Forgets every kept answer, these stores' and the other keepers' that
 * asked (`whenForgotten`): tests start from nothing. */
export function forgetSharedReads(): void {
  for (const store of every) store.reset();
}

/** Runs `reset` with `forgetSharedReads` (another module's own memory). */
export function whenForgotten(reset: () => void): void {
  every.add({ reset });
}

function store<T>(read: () => Promise<T>): SharedRead<T> & { idle: () => boolean } {
  let value: T | undefined;
  let at = 0;
  let started = 0;
  let pending: Promise<T | undefined> | null = null;
  const listeners = new Set<() => void>();
  const pollers = new Map<number, number>();
  let nextPoller = 0;
  let timer: ReturnType<typeof setInterval> | null = null;
  let timerMs = 0;

  const start = (): Promise<T | undefined> => {
    // Only the newest read's answer is kept (a refresh after a change
    // wins over a read that began before it).
    const mine = ++started;
    const run: Promise<T | undefined> = read()
      .then(
        (next) => {
          if (mine === started) {
            value = next;
            at = Date.now();
            listeners.forEach((listener) => listener());
          }
          return value;
        },
        () => value,
      )
      .finally(() => {
        if (pending === run) pending = null;
      });
    pending = run;
    return run;
  };
  const load = (maxAgeMs = 0): Promise<T | undefined> => {
    if (pending) return pending;
    if (value !== undefined && maxAgeMs > 0 && Date.now() - at < maxAgeMs) return Promise.resolve(value);
    return start();
  };
  const retime = () => {
    const shortest = pollers.size > 0 ? Math.min(...pollers.values()) : 0;
    if (shortest === timerMs) return;
    if (timer !== null) clearInterval(timer);
    timer = shortest > 0 ? setInterval(() => void load(), shortest) : null;
    timerMs = shortest;
  };
  const self = {
    peek: () => value,
    load,
    refresh: start,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
    poll: (everyMs: number) => {
      const id = ++nextPoller;
      pollers.set(id, everyMs);
      retime();
      return () => {
        pollers.delete(id);
        retime();
      };
    },
    reset: () => {
      value = undefined;
      at = 0;
      started++;
      pending = null;
      pollers.clear();
      retime();
    },
    idle: () => listeners.size === 0 && pollers.size === 0,
  };
  return self;
}

/** One read shared by every screen that shows it. */
export function sharedRead<T>(read: () => Promise<T>): SharedRead<T> {
  const one = store(read);
  every.add(one);
  return one;
}

/** One read per key (a project, a session): at most `keep` answers are
 * kept, the least recently asked for dropped first (never one shown). */
export function sharedReads<T>(read: (key: string) => Promise<T>, keep = 32): (key: string) => SharedRead<T> {
  const byKey = new Map<string, ReturnType<typeof store<T>>>();
  every.add({ reset: () => byKey.clear() });
  return (key) => {
    let one = byKey.get(key);
    if (one) byKey.delete(key);
    else one = store(() => read(key));
    byKey.set(key, one);
    for (const [old, entry] of byKey) {
      if (byKey.size <= keep) break;
      if (old !== key && entry.idle()) {
        entry.reset();
        byKey.delete(old);
      }
    }
    return one;
  };
}

/** The kept answer of `source` (`undefined` before the first), read
 * again when this mounts and when `version` changes (unless younger than
 * `maxAgeMs`), and every `pollMs` while shown. `wanted` false reads
 * nothing (a view that doesn't show it). */
export function useSharedRead<T>(
  source: SharedRead<T> | null,
  { maxAgeMs = 0, pollMs = 0, wanted = true, version = 0 }: { maxAgeMs?: number; pollMs?: number; wanted?: boolean; version?: unknown } = {},
): T | undefined {
  const value = useSyncExternalStore(source?.subscribe ?? noSubscribe, source?.peek ?? nothing);
  useEffect(() => {
    if (source && wanted) void source.load(maxAgeMs);
  }, [source, wanted, maxAgeMs, version]);
  useEffect(() => (source && wanted && pollMs > 0 ? source.poll(pollMs) : undefined), [source, wanted, pollMs]);
  return value;
}

const noSubscribe = () => () => undefined;
const nothing = () => undefined;
