/**
 * E19 — whether a session holds or worked a task (its page shows a Tasks
 * tab only then). One small read (a page of one row) when the page
 * opens, again when tasks change.
 */
import { useEffect, useState } from "react";
import { onTasksChanged, tasksOfSession } from "../../lib/ipc/tasks";

export function useSessionTasks(sessionId: string | null): boolean {
  const [has, setHas] = useState(false);
  useEffect(() => {
    setHas(false);
    if (!sessionId) return;
    let alive = true;
    const read = () => void tasksOfSession(sessionId, { cursor: null, limit: 1 }).then((page) => alive && setHas(page.items.length > 0), () => undefined);
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [sessionId]);
  return has;
}
