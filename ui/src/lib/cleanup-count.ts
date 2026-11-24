/**
 * AJ-E2404 — the top bar's count of what is left to clean up: the
 * Processes view's own count (left behind or didn't stop, less what you
 * kept or snoozed), the same number This Mac shows. Read once, then kept
 * live by the backend's `processes://counts`, told when a count moves
 * (no timer). AJ-E2325: plus the move backups kept for Undo (the
 * Processes page says them under its tallies), read again when they
 * change.
 */
import { useEffect, useState } from "react";
import { onProcessCounts, processesCounts } from "./ipc/processes";
import { BACKUPS_CHANGED, switchMoveBackups } from "./ipc/switch-move";

export function useCleanupCount(): number {
  const [count, setCount] = useState(0);
  const [backups, setBackups] = useState(0);
  useEffect(() => {
    let alive = true;
    void processesCounts().then((counts) => alive && setCount(counts.to_clean), () => undefined);
    const stop = onProcessCounts((counts) => alive && setCount(counts.to_clean));
    const readBackups = () =>
      void switchMoveBackups().then(
        (info) => alive && setBackups(typeof info.count === "number" ? info.count : 0),
        () => undefined,
      );
    readBackups();
    window.addEventListener(BACKUPS_CHANGED, readBackups);
    return () => {
      alive = false;
      stop();
      window.removeEventListener(BACKUPS_CHANGED, readBackups);
    };
  }, []);
  return count + backups;
}
