/**
 * AJ-E2108 — a session's conversation as its page shows it: read from
 * the end of its transcript once the page is shown (never before), up to
 * 30 turns; earlier ones a page at a time on request; and, while it runs
 * and the page is in view, followed along: a cheap ask every few seconds
 * that the backend answers "unchanged" from the file's stamp alone. Kept
 * in this page's memory only, gone when it closes.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { conversationsLastMessages, type LastTurnInfo } from "../../lib/ipc/conversations";
import { errorText } from "../../lib/bridge";
import { LAST_MESSAGES_EVENT } from "../../lib/last-messages";

/** How often a working session is asked again; any other open one. */
export const FOLLOW_WORKING_MS = 4_000;
export const FOLLOW_OPEN_MS = 10_000;

export type Chat =
  | { state: "loading" }
  | { state: "failed"; why: string }
  | { state: "off" | "none" | "compressed" | "unreadable" }
  | {
      state: "shown";
      /** Oldest first: the earlier pages, then the latest. */
      turns: LastTurnInfo[];
      /** Keys or tokens taken out of what is shown. */
      redacted: number;
      /** Where the oldest shown turn starts: more before it; null at the
       * transcript's start. */
      earlier: number | null;
      loadingEarlier: boolean;
    };

type Latest = { from: number | null; stamp: string | null; turns: LastTurnInfo[]; redacted: number };
type Older = { turns: LastTurnInfo[]; earlier: number | null; redacted: number };

function documentHidden(): boolean {
  return typeof document !== "undefined" && document.visibilityState === "hidden";
}

export function useLastMessages({
  profileId,
  conversationId,
  visible,
  live,
  working,
}: {
  profileId: string;
  conversationId: string | null;
  /** While the page (its tab) is shown. */
  visible: boolean;
  /** Open now: followed while shown. */
  live: boolean;
  /** Working now: followed more often. */
  working: boolean;
}): { chat: Chat; showEarlier: () => void } {
  const key = `${profileId}\u0000${conversationId ?? ""}`;
  const [status, setStatus] = useState<{ key: string; chat: Exclude<Chat, { state: "shown" }> } | null>(null);
  const [latest, setLatest] = useState<Latest | null>(null);
  const [older, setOlder] = useState<Older | null>(null);
  const [loadingEarlier, setLoadingEarlier] = useState(false);
  const readFor = useRef<string | null>(null);
  const latestRef = useRef<Latest | null>(null);
  useEffect(() => {
    latestRef.current = latest;
  }, [latest]);
  const [choice, setChoice] = useState(0);
  const [pageHidden, setPageHidden] = useState(documentHidden);

  // Turned on or off in Settings: read again (or say it's off).
  useEffect(() => {
    const changed = () => {
      readFor.current = null;
      setChoice((n) => n + 1);
    };
    window.addEventListener(LAST_MESSAGES_EVENT, changed);
    return () => window.removeEventListener(LAST_MESSAGES_EVENT, changed);
  }, []);

  // The window hidden (another app in front, minimised): no asking.
  useEffect(() => {
    const changed = () => setPageHidden(documentHidden());
    document.addEventListener("visibilitychange", changed);
    return () => document.removeEventListener("visibilitychange", changed);
  }, []);

  // The first read: once the page is shown, once per session (again only
  // when a read was cut short by leaving it, or the setting changed).
  useEffect(() => {
    if (!visible || conversationId === null || readFor.current === key) return;
    readFor.current = key;
    setStatus({ key, chat: { state: "loading" } });
    setLatest(null);
    setOlder(null);
    let alive = true;
    let answered = false;
    conversationsLastMessages(profileId, conversationId).then(
      (info) => {
        answered = true;
        if (!alive) return;
        if (info.state === "shown") {
          setStatus(null);
          setLatest({ from: info.earlier ?? 0, stamp: info.stamp, turns: info.turns, redacted: info.redacted });
          setOlder({ turns: [], earlier: info.earlier, redacted: 0 });
        } else {
          setStatus({ key, chat: { state: info.state as "off" } });
        }
      },
      (cause) => {
        answered = true;
        if (alive) setStatus({ key, chat: { state: "failed", why: errorText(cause) } });
      },
    );
    return () => {
      alive = false;
      if (!answered) readFor.current = null;
    };
  }, [visible, key, profileId, conversationId, choice]);

  // Following along while it runs and the page is in view.
  const following = visible && live && !pageHidden && latest !== null && conversationId !== null;
  useEffect(() => {
    if (!following || conversationId === null) return;
    let alive = true;
    let busy = false;
    const tick = () => {
      const now = latestRef.current;
      if (busy || !now) return;
      busy = true;
      conversationsLastMessages(profileId, conversationId, { known: now.stamp, from: now.from }).then(
        (info) => {
          busy = false;
          if (!alive || info.state === "unchanged") return;
          if (info.state !== "shown") {
            setStatus({ key, chat: { state: info.state as "off" } });
            setLatest(null);
            return;
          }
          const from = info.earlier ?? 0;
          // Too much new to read from the page's start: the latest only.
          if (from !== now.from) setOlder({ turns: [], earlier: info.earlier, redacted: 0 });
          setLatest({ from, stamp: info.stamp, turns: info.turns, redacted: info.redacted });
        },
        () => {
          busy = false;
        },
      );
    };
    // poll: only while it runs, its page is shown and the window in view;
    // an unchanged transcript answers from its stamp without a read.
    const timer = window.setInterval(tick, working ? FOLLOW_WORKING_MS : FOLLOW_OPEN_MS);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [following, working, key, profileId, conversationId]);

  const showEarlier = useCallback(() => {
    const cursor = older?.earlier;
    if (cursor == null || conversationId === null || loadingEarlier) return;
    setLoadingEarlier(true);
    conversationsLastMessages(profileId, conversationId, { before: cursor }).then(
      (info) => {
        setLoadingEarlier(false);
        if (info.state !== "shown") return;
        setOlder((now) => ({ turns: [...info.turns, ...(now?.turns ?? [])], earlier: info.earlier, redacted: (now?.redacted ?? 0) + info.redacted }));
      },
      () => setLoadingEarlier(false),
    );
  }, [older, conversationId, profileId, loadingEarlier]);

  let chat: Chat;
  if (status && status.key === key) chat = status.chat;
  else if (latest && older) {
    chat = {
      state: "shown",
      turns: [...older.turns, ...latest.turns],
      redacted: older.redacted + latest.redacted,
      earlier: older.earlier,
      loadingEarlier,
    };
  } else chat = { state: "loading" };
  return { chat, showEarlier };
}
