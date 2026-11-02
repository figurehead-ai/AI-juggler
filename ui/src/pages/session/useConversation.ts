/**
 * 0.3.5 (ADR-0024) — a conversation as its page shows it: followed while
 * the page (and its window) is shown, through the conversation store;
 * no timer of its own. Its turns come once and then only what changes.
 * Kept in memory only, never on disk.
 */
import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import { conversationsFollow } from "../../lib/ipc/conversations";
import { connect, disconnect, showEarlier, stateOf, subscribe, type FeedSource } from "../../lib/conversation-store";
import type { FeedState } from "../../lib/conversation-feed-state";

export type { FeedSource };

function documentHidden(): boolean {
  return typeof document !== "undefined" && document.visibilityState === "hidden";
}

/** The session's own conversation (`profileId`'s `conversationId`). */
export function sessionSource(profileId: string, conversationId: string): FeedSource {
  return {
    key: `${profileId}\u0000${conversationId}`,
    follow: (max, onDelta) => conversationsFollow(profileId, conversationId, max, onDelta),
  };
}

export function useConversation({ source, visible }: { source: FeedSource | null; visible: boolean }): { state: FeedState; showEarlier: () => void } {
  const key = source?.key ?? "";
  const [pageHidden, setPageHidden] = useState(documentHidden);
  // The window hidden (another app in front, minimised): not followed.
  useEffect(() => {
    const changed = () => setPageHidden(documentHidden());
    document.addEventListener("visibilitychange", changed);
    return () => document.removeEventListener("visibilitychange", changed);
  }, []);
  const active = visible && !pageHidden && source !== null;
  useEffect(() => {
    if (!active || !source) return;
    connect(source);
    return () => disconnect(source.key);
    // `source` is the same conversation while its key is.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, key]);
  const state = useSyncExternalStore(
    useCallback((listener: () => void) => subscribe(key, listener), [key]),
    () => stateOf(key),
  );
  const earlier = useCallback(() => void showEarlier(key), [key]);
  return { state, showEarlier: earlier };
}
