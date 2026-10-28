/**
 * wf09: whether a session has any history to show (its History tab shows
 * only then): one row read, kept a minute. Apart from the tab itself,
 * which loads when opened.
 */
import { useCached } from "../../lib/usage/cached";
import { sessionsHistory } from "../../lib/ipc/sessions";

export function useSessionHasHistory(sessionId: string | null): boolean {
  const result = useCached(sessionId ? `session-history-any:${sessionId}` : null, () => sessionsHistory(sessionId!, { cursor: null, limit: 1 }), 60_000);
  return sessionId !== null && (result.data?.items.length ?? 0) > 0;
}
