/**
 * 0.3 — a project's tasks by status (every pool of the project's), read
 * across pools and again when tasks change: its page's Tasks count and
 * its Overview's tile.
 */
import { useEffect, useState } from "react";
import { onTasksChanged, type TaskCounts } from "../../lib/ipc/tasks";
import { tasksAcrossCounts } from "../../lib/ipc/tasks-across";

/** A project's tasks by status (its pools'), kept current. */
export function useProjectTasks(projectId: string | null): TaskCounts | null {
  const [counts, setCounts] = useState<{ id: string; counts: TaskCounts } | null>(null);
  useEffect(() => {
    if (!projectId) return;
    let alive = true;
    const read = () => void tasksAcrossCounts({ project_id: projectId }).then((next) => alive && setCounts({ id: projectId, counts: next }), () => undefined);
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [projectId]);
  return counts && counts.id === projectId ? counts.counts : null;
}
