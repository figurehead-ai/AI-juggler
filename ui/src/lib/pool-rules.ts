/**
 * D-62 (AJ-E3008): one pool's member rules, talk settings and Watched
 * members, read when its page opens and again whenever the pools change
 * (a join, a leave, a talk rule, a seal). The canvas draws the broad
 * members and the Watched ones from it; the inspector changes them.
 */
import { useCallback, useEffect, useState } from "react";

import { poolRules, poolWatched, type PoolRulesInfo, type WatchedMemberInfo } from "./ipc/pool-model";
import { usePoolEvents } from "./pool-live";

export interface PoolRules {
  rules: PoolRulesInfo | null;
  watched: WatchedMemberInfo[];
  /** Reads them again now (after a change made here). */
  reload: () => void;
}

export function usePoolRules(poolId: string): PoolRules {
  const [rules, setRules] = useState<PoolRulesInfo | null>(null);
  const [watched, setWatched] = useState<WatchedMemberInfo[]>([]);
  const [nonce, setNonce] = useState(0);
  useEffect(() => {
    let alive = true;
    void Promise.all([poolRules(poolId), poolWatched(poolId)]).then(
      ([nextRules, nextWatched]) => {
        if (!alive) return;
        setRules(nextRules);
        setWatched(nextWatched);
      },
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [poolId, nonce]);
  const reload = useCallback(() => setNonce((value) => value + 1), []);
  // Round 7 #6: its counts follow the app live (a member changed, a rule
  // set here, a session started, ended or attested), as its desktop
  // members and sessions do: one place, lib/pool-live.
  usePoolEvents(poolId, reload);
  return { rules, watched, reload };
}
