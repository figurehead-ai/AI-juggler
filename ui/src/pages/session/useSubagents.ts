/**
 * AJ-E1520 (0.2) — whether a session's page shows its Subagents tab:
 * only when its tool records subagents and it started some (progressive
 * disclosure). One small read when the page opens, again now and then
 * while the session runs (a first subagent may start).
 */
import { useEffect, useState } from "react";
import { subagentsList } from "../../lib/ipc/subagents";

/** How often an open session is asked again. */
export const SUBAGENTS_CHECK_MS = 15_000;

/** What was last heard, by session: a page shows the tab at once. */
const heard = new Map<string, boolean>();

/** Something already knows this session started subagents (the rail's
 * count): its page shows the tab from its first render. */
export function markHasSubagents(profileId: string, conversationId: string): void {
  heard.set(`${profileId}\u0000${conversationId}`, true);
}

export function useHasSubagents(profileId: string | null, conversationId: string | null, live: boolean): boolean {
  const key = profileId && conversationId ? `${profileId}\u0000${conversationId}` : null;
  const [has, setHas] = useState<{ key: string | null; has: boolean }>(() => ({ key, has: key ? (heard.get(key) ?? false) : false }));
  useEffect(() => {
    if (!key || !profileId || !conversationId) return;
    let alive = true;
    const read = () =>
      void subagentsList(profileId, conversationId, { cursor: null, limit: 1 }).then(
        (info) => {
          const now = info.state === "listed" && (info.page.total ?? info.page.items.length) > 0;
          heard.set(key, now);
          if (alive) setHas({ key, has: now });
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
