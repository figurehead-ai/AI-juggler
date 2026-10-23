/**
 * AJ-E1529 — the one list of every account's conversations, for every
 * screen that shows sessions: the rail, a project's page, Accounts' and
 * the account page's running counts, and the go-to palette. Read once,
 * read again when the backend says it changed (and when asked).
 */
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type JSX, type ReactNode } from "react";
import { conversationsList, onConversationsChanged, type ConversationInfo } from "./ipc/conversations";

interface ConversationsState {
  /** Every conversation, newest first. */
  all: ConversationInfo[];
  /** Read the list again (after a launch, an import). */
  refresh: () => void;
}

const Context = createContext<ConversationsState>({ all: [], refresh: () => {} });

export function ConversationsProvider({ children }: { children: ReactNode }): JSX.Element {
  const [all, setAll] = useState<ConversationInfo[]>([]);
  const refresh = useCallback(() => {
    void conversationsList().then(setAll, () => undefined);
  }, []);
  useEffect(() => {
    refresh();
    return onConversationsChanged(refresh);
  }, [refresh]);
  const value = useMemo(() => ({ all, refresh }), [all, refresh]);
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useConversations(): ConversationsState {
  return useContext(Context);
}

/** Conversations AI Juggler didn't start (its own show as its sessions). */
export function outside(all: ConversationInfo[]): ConversationInfo[] {
  return all.filter((conversation) => conversation.session_id === null);
}

/** Outside conversations a running session is on now, optionally on one
 * account. */
export function liveOutside(all: ConversationInfo[], accountId?: string): ConversationInfo[] {
  return outside(all).filter((conversation) => conversation.live && (accountId === undefined || conversation.account_id === accountId));
}

/** A conversation's title on screen: its own, else its branch, else its
 * folder's name. */
export function conversationTitle(conversation: ConversationInfo): string {
  const folder = (conversation.cwd ?? "").replace(/\/+$/, "").split("/").pop() ?? "";
  return conversation.title ?? conversation.git_branch ?? (folder || conversation.id.slice(0, 8));
}
