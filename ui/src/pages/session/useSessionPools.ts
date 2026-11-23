/**
 * AJ-E3104: a session's pools, read once for its Pools tab and the tab's
 * count. Apart from the tab's code, which loads only when it's opened.
 */
import { useEffect, useState } from "react";

import { poolsOfSession, type SessionPoolsInfo } from "../../lib/ipc/pool-model";
import { usePools } from "../../lib/pools";
import { logLoadFailure, withDeadline } from "../../lib/loading";

/** One read per session and pools version, shared by the tab and its count. */
const reads = new Map<string, { version: unknown; read: Promise<SessionPoolsInfo> }>();

/** The read, shared while the pools are the same; a failed one isn't kept. */
function rawPoolsOf(sessionId: string, version: unknown): Promise<SessionPoolsInfo> {
  const kept = reads.get(sessionId);
  if (kept && kept.version === version) return kept.read;
  const read = withDeadline("its pools", poolsOfSession(sessionId));
  read.catch(() => reads.get(sessionId)?.read === read && reads.delete(sessionId));
  reads.set(sessionId, { version, read });
  return read;
}

function readPoolsOf(sessionId: string, version: unknown): Promise<SessionPoolsInfo> {
  return rawPoolsOf(sessionId, version).catch(() => ({ session_id: sessionId, pools: [], skipped: [], account_unknown: false }));
}

/** AJ-E1552: the session's pools with the read's failure and a Retry,
 * for the Pools tab (the doors and levels take a failure as "none"). */
export function useSessionPoolsRead(sessionId: string): { info: SessionPoolsInfo | null; error: unknown; round: number; retry: () => void } {
  const pools = usePools();
  const [round, setRound] = useState(0);
  const [got, setGot] = useState<{ asked: string; info: SessionPoolsInfo | null; error: unknown } | null>(null);
  const asked = `${sessionId}#${round}`;
  useEffect(() => {
    let alive = true;
    void rawPoolsOf(sessionId, pools).then(
      (info) => alive && setGot({ asked, info, error: undefined }),
      (error: unknown) => {
        if (!alive) return;
        logLoadFailure("its pools", error);
        setGot({ asked, info: null, error });
      },
    );
    return () => {
      alive = false;
    };
  }, [sessionId, asked, pools]);
  const mine = got !== null && got.asked === asked ? got : null;
  return { info: mine?.info ?? null, error: mine?.error, round, retry: () => setRound((was) => was + 1) };
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
