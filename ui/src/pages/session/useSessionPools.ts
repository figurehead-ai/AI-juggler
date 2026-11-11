/**
 * AJ-E3104: a session's pools, read once for its Pools tab and the tab's
 * count. Apart from the tab's code, which loads only when it's opened.
 */
import { useEffect, useState } from "react";

import { poolsOfSession, type SessionPoolsInfo } from "../../lib/ipc/pool-model";
import { usePools } from "../../lib/pools";

/** One read per session and pools version, shared by the tab and its count. */
const reads = new Map<string, { version: unknown; read: Promise<SessionPoolsInfo> }>();

function readPoolsOf(sessionId: string, version: unknown): Promise<SessionPoolsInfo> {
  const kept = reads.get(sessionId);
  if (kept && kept.version === version) return kept.read;
  const read = poolsOfSession(sessionId).catch(() => ({ session_id: sessionId, pools: [], skipped: [], account_unknown: false }));
  reads.set(sessionId, { version, read });
  return read;
}

/** AJ-E3104: what a session's Pools tab lists, read once for the tab and
 * its count (the count is the list's length, never a second guess), read
 * again when the pools change. */
export function useSessionPools(sessionId: string | null): SessionPoolsInfo | null {
  const pools = usePools();
  const [info, setInfo] = useState<SessionPoolsInfo | null>(null);
  useEffect(() => {
    if (sessionId === null) return undefined;
    let alive = true;
    void readPoolsOf(sessionId, pools).then((next) => alive && setInfo(next));
    return () => {
      alive = false;
    };
  }, [sessionId, pools]);
  return info !== null && info.session_id === sessionId ? info : null;
}
