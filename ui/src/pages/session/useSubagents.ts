/**
 * AJ-E1520 (0.2) — whether a session's page shows its Subagents tab:
 * only when its tool records subagents and it started some (progressive
 * disclosure). One small read when the page opens, again now and then
 * while the session runs (a first subagent may start). The same read
 * says how many it started and how many run: the session's quiet line to
 * the tab hears them from it, with no read of its own.
 */
import { useEffect, useState, useSyncExternalStore } from "react";
import { subagentsList } from "../../lib/ipc/subagents";

/** How often an open session is asked again. */
export const SUBAGENTS_CHECK_MS = 15_000;

/** How many subagents a session started, and how many run now. */
export type SubagentCounts = { total: number; running: number };

/** What was last heard, by session: a page shows the tab at once. */
const heard = new Map<string, boolean>();
/** The counts last read, by session. */
const counted = new Map<string, SubagentCounts>();
/** Who wants to hear new counts. */
const listeners = new Set<() => void>();

const keyOf = (profileId: string | null, conversationId: string | null): string | null =>
  profileId && conversationId ? `${profileId}\u0000${conversationId}` : null;

/** Something already knows this session started subagents (the rail's
 * count): its page shows the tab from its first render. */
export function markHasSubagents(profileId: string, conversationId: string): void {
  heard.set(`${profileId}\u0000${conversationId}`, true);
}

export function useHasSubagents(profileId: string | null, conversationId: string | null, live: boolean): boolean {
  const key = keyOf(profileId, conversationId);
  const [has, setHas] = useState<{ key: string | null; has: boolean }>(() => ({ key, has: key ? (heard.get(key) ?? false) : false }));
  useEffect(() => {
    if (!key || !profileId || !conversationId) return;
    let alive = true;
    const read = () =>
      void subagentsList(profileId, conversationId, { cursor: null, limit: 1 }).then(
        (info) => {
          const total = info.state === "listed" ? (info.page.total ?? info.page.items.length) : 0;
          heard.set(key, total > 0);
          const last = counted.get(key);
          if (!last || last.total !== total || last.running !== info.running) {
            counted.set(key, { total, running: info.running });
            listeners.forEach((listener) => listener());
          }
          if (alive) setHas({ key, has: total > 0 });
        },
        () => undefined,
      );
    read();
    const timer = live ? window.setInterval(read, SUBAGENTS_CHECK_MS) : undefined;
    return () => {
      alive = false;
      if (timer !== undefined) window.clearInterval(timer);
    };
  }, [key, profileId, conversationId, live]);
  if (has.key !== key) return key ? (heard.get(key) ?? false) : false;
  return has.has;
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

/** The counts the page's read last heard for a session (`null` before
 * it has read them); no read of its own. */
export function useHeardSubagentCounts(profileId: string | null, conversationId: string | null): SubagentCounts | null {
  const key = keyOf(profileId, conversationId);
  return useSyncExternalStore(subscribe, () => (key ? (counted.get(key) ?? null) : null));
}
