/**
 * AJ-E402/E401 — the read-model ticks: the coalesced 5 s resource and
 * 30 s quota deltas merge into the rows by id (no full refetch per tick).
 * This drives the I-07 sparkline history and the quota countdowns. Each
 * delta also refreshes its sampled-at time per id (the stale rule).
 */
import { useEffect, type Dispatch, type SetStateAction } from "react";
import { onSessionsQuota, onSessionsResources, type InstanceInfo } from "./bridge";

/** When each value class of a session was last sampled, by id. */
export type Freshness = Record<string, { resources: number; quota: number }>;

export function useLiveDeltas(
  setSessions: Dispatch<SetStateAction<InstanceInfo[]>>,
  setFreshness: Dispatch<SetStateAction<Freshness>>,
): void {
  useEffect(() => {
    const stopResources = onSessionsResources((deltas) => {
      const at = Date.now();
      setSessions((current) =>
        current.map((row) => {
          const delta = deltas.find((d) => d.id === row.id);
          return delta
            ? {
                ...row,
                cpu_pct: delta.cpu_pct,
                rss_bytes: delta.rss_bytes,
                child_count: delta.child_count,
              }
            : row;
        }),
      );
      setFreshness((current) => {
        const next = { ...current };
        for (const delta of deltas) {
          if (next[delta.id]) next[delta.id] = { ...next[delta.id]!, resources: at };
        }
        return next;
      });
    });
    const stopQuota = onSessionsQuota((deltas) => {
      const at = Date.now();
      setSessions((current) =>
        current.map((row) => {
          const delta = deltas.find((d) => d.id === row.id);
          return delta ? { ...row, quota: delta.quota } : row;
        }),
      );
      setFreshness((current) => {
        const next = { ...current };
        for (const delta of deltas) {
          if (next[delta.id]) next[delta.id] = { ...next[delta.id]!, quota: at };
        }
        return next;
      });
    });
    return () => {
      stopResources();
      stopQuota();
    };
  }, [setSessions, setFreshness]);
}

/**
 * One run at a time: a call while `run` is in flight doesn't start
 * another, it asks for exactly one more run once this one ends. A burst
 * of session events (eight live sessions send many) then reads the
 * lists twice at most, not once per event (0.1.1's spinning wheel).
 */
export function singleFlight(run: () => Promise<void>): () => Promise<void> {
  let running: Promise<void> | null = null;
  let again = false;
  const start = (): Promise<void> => {
    running = (async () => {
      try {
        do {
          again = false;
          await run();
        } while (again);
      } finally {
        running = null;
      }
    })();
    return running;
  };
  return () => {
    if (running) {
      again = true;
      return running;
    }
    return start();
  };
}
