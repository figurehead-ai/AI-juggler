/**
 * AJ-E2404 — the top bar's count of what is left to clean up: the
 * Processes view's own count (left behind or didn't stop, less what you
 * kept or snoozed), the same number This Mac shows. Read once, then kept
 * live by the backend's `processes://counts`, told when a count moves
 * (no timer). AJ-E2325: plus the move backups kept for Undo (the
 * Processes page says them under its tallies); 0.5.1 F93: the one read
 * Settings › Moves shows too (`lib/move-backups`), so both say the same.
 */
import { useEffect, useState } from "react";
import { onProcessCounts, processesCounts } from "./ipc/processes";
import { useMoveBackups } from "./move-backups";

export function useCleanupCount(): number {
  const [count, setCount] = useState(0);
  const backups = useMoveBackups();
  useEffect(() => {
    let alive = true;
    void processesCounts().then((counts) => alive && setCount(counts.to_clean), () => undefined);
    const stop = onProcessCounts((counts) => alive && setCount(counts.to_clean));
    return () => {
      alive = false;
      stop();
    };
  }, []);
  return count + (typeof backups?.count === "number" ? backups.count : 0);
}
