/**
 * E19 — whether a session holds or worked a task (its page shows a Tasks
 * tab only then). One small read (a page of one row) when the page
 * opens, again when tasks change. 0.3.4: the last answer per session is
 * kept, so a page opened again has its tab in its first frame (it's read
 * again behind it; a task a session worked stays in its record).
 */
import { useEffect, useState } from "react";
import { onTasksChanged, tasksOfSession } from "../../lib/ipc/tasks";
import { keptFact, keepFact } from "../session/tab-facts";

const KEPT = "tasks";

/** Whether the session has a task, and whether that's known yet (read,
 * or kept from before). */
export function useSessionTasksFact(sessionId: string | null): { has: boolean; known: boolean } {
  const [fact, setFact] = useState<{ sessionId: string | null; has: boolean | undefined }>(() => ({ sessionId, has: keptFact(KEPT, sessionId) }));
  const current = fact.sessionId === sessionId ? fact : { sessionId, has: keptFact(KEPT, sessionId) };
  if (current !== fact) setFact(current);
  useEffect(() => {
    if (!sessionId) return;
    let alive = true;
    const read = () =>
      void tasksOfSession(sessionId, { cursor: null, limit: 1 }).then(
        (page) => {
          keepFact(KEPT, sessionId, page.items.length > 0);
          if (alive) setFact({ sessionId, has: page.items.length > 0 });
        },
        () => alive && setFact((now) => (now.has === undefined ? { sessionId, has: false } : now)),
      );
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [sessionId]);
  return { has: current.has === true, known: sessionId === null || current.has !== undefined };
}
