/**
 * E18 — whether a session has messages to or from it (its page shows a
 * Messages tab only then), and its pool. One small read when the page
 * opens, again when messages change. 0.3.4: the last answer per session
 * is kept, so a page opened again has its tab in its first frame (it's
 * read again behind it; a thread is never taken back, so a kept "has
 * some" stays true).
 */
import { useEffect, useState } from "react";
import { messagesThreads, onMessagesChanged } from "../../lib/ipc/messages";
import { usePoolPlace } from "../../lib/pools";
import { keptFact, keepFact } from "../session/tab-facts";

const KEPT = "messages";

/** The session's pool id when it has messages (`null` otherwise), and
 * whether that's known yet (read, or kept from before). */
export function useSessionMessagesFact(sessionId: string | null): { poolId: string | null; known: boolean } {
  // Read alone: the session may be past the working set (opened from
  // History); the rail's Pool lens gets its place from this read too.
  const poolId = usePoolPlace(sessionId)?.pool_id ?? null;
  const [fact, setFact] = useState<{ sessionId: string | null; has: boolean | undefined }>(() => ({ sessionId, has: keptFact(KEPT, sessionId) }));
  // Another session: its own kept answer at once.
  const current = fact.sessionId === sessionId ? fact : { sessionId, has: keptFact(KEPT, sessionId) };
  if (current !== fact) setFact(current);
  useEffect(() => {
    if (!sessionId) return;
    let alive = true;
    const read = () =>
      void messagesThreads({ sessionId }).then(
        (page) => {
          keepFact(KEPT, sessionId, page.threads.length > 0);
          if (alive) setFact({ sessionId, has: page.threads.length > 0 });
        },
        () => alive && setFact((now) => (now.has === undefined ? { sessionId, has: false } : now)),
      );
    read();
    const off = onMessagesChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [sessionId]);
  // Known once the read (or the kept answer) is here, and its pool too
  // when it has some (the tab needs both).
  return { poolId: current.has ? poolId : null, known: sessionId === null || current.has === false || (current.has === true && poolId !== null) };
}
