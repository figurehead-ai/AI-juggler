/**
 * AJ-E1529 — the one list of every account's conversations, for every
 * screen that shows sessions: the rail, a project's page, Accounts' and
 * the account page's running counts, and the go-to palette. Read once,
 * read again when the backend says it changed (and when asked).
 */
import { untitled } from "./project-words";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type JSX, type ReactNode } from "react";
import { conversationsGet, conversationsList, onConversationsChanged, type ConversationInfo } from "./ipc/conversations";

interface ConversationsState {
  /** Every conversation, newest first. */
  all: ConversationInfo[];
  /** Read the list again (after a launch, an import). */
  refresh: () => void;
  /** AJ-E2502: the first read has answered (Needs you is whole). */
  loaded: boolean;
}

const Context = createContext<ConversationsState>({ all: [], refresh: () => {}, loaded: true });

/**
 * The list is the backend's working set (open, in a sidebar, today's and
 * the week's newest), never every conversation ever had (0.1.5 review).
 * `want` names one that must be in it (the page open now): one past the
 * working set (an old one, opened from History) is read by its id and
 * joins the list while wanted.
 */
export function ConversationsProvider({ children, want = null }: { children: ReactNode; want?: { accountId: string; id: string } | null }): JSX.Element {
  const [working, setAll] = useState<ConversationInfo[]>([]);
  const [extra, setExtra] = useState<ConversationInfo | null>(null);
  const [loaded, setLoaded] = useState(false);
  const refresh = useCallback(() => {
    void conversationsList()
      .then(setAll, () => undefined)
      .finally(() => setLoaded(true));
  }, []);
  useEffect(() => {
    refresh();
    return onConversationsChanged(refresh);
  }, [refresh]);
  const missing = want !== null && !working.some((c) => c.account_id === want.accountId && c.id === want.id);
  useEffect(() => {
    if (!missing || want === null) return;
    let alive = true;
    void conversationsGet(want.accountId, want.id).then(
      (found) => alive && setExtra(found),
      () => null,
    );
    return () => {
      alive = false;
    };
    // Read again when the list is (its live state may have changed).
  }, [missing, want?.accountId, want?.id, working]);
  const all = useMemo(
    () => (missing && extra && want && extra.account_id === want.accountId && extra.id === want.id ? [...working, extra] : working),
    [missing, extra, want, working],
  );
  const value = useMemo(() => ({ all, refresh, loaded }), [all, refresh, loaded]);
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
  // Never a bare "main": a telling branch, else "Untitled session · <folder>".
  return conversation.title ?? untitled(conversation.git_branch, folder);
}
