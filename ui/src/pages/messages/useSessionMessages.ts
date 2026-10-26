/**
 * E18 — whether a session has messages to or from it (its page shows a
 * Messages tab only then), and its pool. One small read when the page
 * opens, again when messages change.
 */
import { useEffect, useState } from "react";
import { messagesThreads, onMessagesChanged } from "../../lib/ipc/messages";
import { usePools } from "../../lib/pools";

/** The session's pool id when it has messages; `null` otherwise. */
export function useSessionMessages(sessionId: string | null): string | null {
  const pools = usePools();
  const poolId = sessionId ? (pools?.members.find((row) => row.session_id === sessionId)?.pool_id ?? null) : null;
  const [has, setHas] = useState(false);
  useEffect(() => {
    if (!sessionId) return;
    let alive = true;
    const read = () => void messagesThreads({ sessionId }).then((page) => alive && setHas(page.threads.length > 0), () => undefined);
    read();
    const off = onMessagesChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [sessionId]);
  return has ? poolId : null;
}
