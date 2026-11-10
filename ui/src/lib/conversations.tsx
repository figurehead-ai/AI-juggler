/**
 * AJ-E1529 — the one list of every account's conversations, for every
 * screen that shows sessions: the rail, a project's page, Accounts' and
 * the account page's running counts, and the go-to palette. Read once,
 * read again when the backend says it changed (and when asked).
 */
import { keepIfSame } from "./same";
import { untitled } from "./project-words";
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type JSX, type ReactNode } from "react";
import { conversationsBuildAgain, conversationsBuilding, conversationsGet, conversationsList, onConversationsChanged, type ConversationInfo } from "./ipc/conversations";
import type { ListBuildInfo } from "../types/generated/ListBuildInfo";

interface ConversationsState {
  /** Every conversation, newest first. */
  all: ConversationInfo[];
  /** Read the list again (after a launch, an import). */
  refresh: () => void;
  /** AJ-E2502: the first read has answered (Needs you is whole). */
  loaded: boolean;
  /** A first start is still building the list (Home and the rail say so,
   * and how far it is); null once it's built. */
  build: ListBuildInfo | null;
  /** The failed build's Try again. */
  buildAgain: () => void;
}

const Context = createContext<ConversationsState>({ all: [], refresh: () => {}, loaded: true, build: null, buildAgain: () => {} });

/** While the build line shows, it asks again this often by itself: its end
 * is told, but a told end that's missed must not leave it on for good. */
const BUILD_ASK_MS = 2_000;

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
  const [build, setBuild] = useState<ListBuildInfo | null>(null);
  // Asked only until it's built: it never starts again in a run.
  const mayBuild = useRef(true);
  // Only the newest question's answer counts: under heavy swapping an
  // older "still building" came back after "built" and stayed (rc24).
  const asked = useRef(0);
  const askBuild = useCallback(() => {
    if (!mayBuild.current) return;
    const ask = ++asked.current;
    void conversationsBuilding().then(
      (now) => {
        if (ask !== asked.current || !mayBuild.current) return;
        mayBuild.current = now.building;
        setBuild((was) => (now.building ? keepIfSame(was, now) : null));
      },
      () => undefined,
    );
  }, []);
  const refresh = useCallback(() => {
    void conversationsList()
      // an equal list keeps its identity: nothing that reads it renders again
      .then((next) => setAll((was) => keepIfSame(was, next)), () => undefined)
      .finally(() => setLoaded(true));
    askBuild();
  }, [askBuild]);
  useEffect(() => {
    refresh();
    return onConversationsChanged(refresh);
  }, [refresh]);
  const building = build !== null;
  useEffect(() => {
    if (!building) return undefined;
    // poll: only while the build line shows (its end is told; this is the
    // net under a missed one), and it stops for good once it's built.
    const timer = window.setInterval(askBuild, BUILD_ASK_MS);
    return () => window.clearInterval(timer);
  }, [building, askBuild]);
  const buildAgain = useCallback(() => {
    void conversationsBuildAgain().then(askBuild, () => undefined);
  }, [askBuild]);
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
  const value = useMemo(() => ({ all, refresh, loaded, build, buildAgain }), [all, refresh, loaded, build, buildAgain]);
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
