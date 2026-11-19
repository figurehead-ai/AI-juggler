/**
 * Round 7 #5, #6 (aij-test runs 2 and 3): what a pool page shows of its
 * members follows the app live, from one place. The page was open before
 * the aij-test session started and never showed it (no node, the To list
 * only "Everyone") until it was left and opened again; after aij-test
 * quit, its App still said "1 open now"; and after an Add, a node read
 * words from a list read before it.
 *
 * Every read of a pool's members (its rules and counts, its desktop
 * members and sessions) reads again when:
 * - its members change here (Add, Take out, Undo, a move: each lands in
 *   the pools store) or its rules do (`POOL_RULES_CHANGED`);
 * - a connector is added or removed (`CONNECTOR_CHANGED`);
 * - a session starts, ends or is attested: the app's own events, the
 *   sessions' (`session://updated`) and the conversations' (a desktop
 *   session's open state, `conversations://changed`).
 * Never on a timer.
 */
import { useEffect, useRef } from "react";

import { CONNECTOR_CHANGED } from "./ipc/connector";
import { onConversationsChanged } from "./ipc/conversations";
import { onSessionsUpdated } from "./ipc/instances";
import { POOL_RULES_CHANGED } from "./ipc/pool-model";
import { usePools } from "./pools";

export function usePoolEvents(poolId: string | undefined, refresh: () => void): void {
  const pools = usePools();
  const seen = useRef(pools);
  useEffect(() => {
    // Not on open: the read that opened it is fresh.
    if (seen.current !== null && seen.current !== pools) refresh();
    seen.current = pools;
  }, [pools, refresh]);
  useEffect(() => {
    if (!poolId) return undefined;
    const mine = (event: Event) => {
      if (!(event instanceof CustomEvent) || event.type === CONNECTOR_CHANGED || event.detail === poolId || event.detail === null) refresh();
    };
    window.addEventListener(CONNECTOR_CHANGED, mine);
    window.addEventListener(POOL_RULES_CHANGED, mine);
    const offSessions = onSessionsUpdated(refresh);
    const offConversations = onConversationsChanged(refresh);
    return () => {
      window.removeEventListener(CONNECTOR_CHANGED, mine);
      window.removeEventListener(POOL_RULES_CHANGED, mine);
      offSessions();
      offConversations();
    };
  }, [poolId, refresh]);
}
