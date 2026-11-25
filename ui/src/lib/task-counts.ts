/**
 * A pool's task counts, kept current (its Tasks tab, its live view).
 * AJ-E1552: a read like any other: `failed` once it fails (cleared by the
 * next answer), and `retry` reads again with a fresh deadline (`round`).
 */
import { useCallback, useEffect, useState } from "react";

import { onTasksChanged, tasksCounts } from "./ipc/tasks";
import type { TaskCounts } from "../types/generated/TaskCounts";

export function useTaskCounts(poolId: string): { counts: TaskCounts | null; failed: unknown; round: number; retry: () => void } {
  const [counts, setCounts] = useState<TaskCounts | null>(null);
  const [failed, setFailed] = useState<unknown>(undefined);
  const [round, setRound] = useState(0);
  useEffect(() => {
    let alive = true;
    const read = () =>
      void tasksCounts(poolId).then(
        (next) => alive && (setFailed(undefined), setCounts(next)),
        (cause: unknown) => alive && setFailed(cause),
      );
    read();
    const off = onTasksChanged(read);
    return () => {
      alive = false;
      off();
    };
    // `round` is a Retry: it reads again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [poolId, round]);
  const retry = useCallback(() => {
    setFailed(undefined);
    setRound((was) => was + 1);
  }, []);
  return { counts, failed, round, retry };
}
