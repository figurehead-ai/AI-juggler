/**
 * AJ-E2404 — the top bar's count of what is left to clean up: the
 * Processes view's own count (left behind or didn't stop, less what you
 * kept or snoozed), the same number This Mac shows. Read once, then kept
 * live by the backend's `processes://changed` (no timer).
 */
import { useEffect, useState } from "react";
import { onProcessesChanged, processesCounts } from "./ipc/processes";

export function useCleanupCount(): number {
  const [count, setCount] = useState(0);
  useEffect(() => {
    let alive = true;
    void processesCounts().then((counts) => alive && setCount(counts.to_clean), () => undefined);
    const stop = onProcessesChanged((change) => alive && setCount(change.counts.to_clean));
    return () => {
      alive = false;
      stop();
    };
  }, []);
  return count;
}
