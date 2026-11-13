/**
 * "Continue in another app" for the session whose page is open: ⌘⇧H,
 * the row menu, the header's Switch account…, an out-of-room banner and
 * Home's "Continue in another app…" all ask through here (the 0.2
 * walk, F8 and F16: ⌘⇧H did nothing on another app's session, and the
 * banner's Hand over… opened the account page).
 *
 * The page's switch list takes the keyboard: the switch card of a session
 * AI Juggler runs in its terminal, or Continue on another account on any
 * other session's page (one another app runs included). A page opened a
 * moment ago takes it when it mounts. When no session's page is open,
 * the asker hears so (`focusHandover` returns false) and says why.
 */
import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ConversationInfo } from "./ipc/conversations";

/** A page that opens now may take a request for its session this long. */
const PENDING_MS = 3_000;

/** The switch lists on screen, oldest first; the last one is the page
 * showing now. A request names the session it is for (the switch source
 * key, `switchSourceOf`, or AI Juggler's session id), and only that
 * session's list may take it (0.3.9: "Continue on another account…" on a
 * row while another session's page was still mounted opened the switch
 * for the page's session, and the wrong conversation moved). */
interface Listener {
  key: string;
  take: () => void;
}
const listeners: Listener[] = [];
/** The session pages on screen whose switch list is on another of their
 * tabs: asked, they show the body, where the list mounts and takes the
 * request. */
const pages: Listener[] = [];
let pending: { key: string; at: number } | null = null;

function takeFor(key: string): boolean {
  const found = listeners.find((one) => one.key === key);
  if (!found) return false;
  pending = null;
  // Its page shows the body too, where the list's dialog belongs.
  pages.find((one) => one.key === key)?.take();
  found.take();
  return true;
}

/** Asks a session's switch list for the keyboard: `key`'s, else (⌘⇧H, a
 * page's own button) the page showing now. True when a list took it; a
 * keyed request no list has yet waits for its page to open. */
export function focusHandover(key?: string): boolean {
  if (key !== undefined) {
    if (takeFor(key)) return true;
    pending = { key, at: Date.now() };
    const page = pages.find((one) => one.key === key);
    page?.take();
    return page !== undefined;
  }
  const shown = listeners[listeners.length - 1];
  if (shown) return takeFor(shown.key);
  const page = pages[pages.length - 1];
  if (!page) return false;
  pending = { key: page.key, at: Date.now() };
  page.take();
  return true;
}

/** A session's page (PageShell): `open` shows the tab its switch list
 * lives on, when a request names `key` (or no one, while it is the page
 * showing) and its list isn't on screen. */
export function useHandoverPage(open: () => void, key: string | undefined): void {
  const ref = useRef(open);
  ref.current = open;
  useEffect(() => {
    if (!key) return;
    const page: Listener = { key, take: () => ref.current() };
    pages.push(page);
    return () => {
      const at = pages.indexOf(page);
      if (at >= 0) pages.splice(at, 1);
    };
  }, [key]);
}

/** What ⌘⇧H says where no switch list is (Home, a project, an account,
 * another computer's session). */
export const NO_SESSION_OPEN = "Nothing here to continue in another app: ⌘⇧H works on the page of a session on this computer.";

/** The same, for `key`'s page, which is only opening now (after selecting
 * it): its list takes the request when it mounts, and no other list ever
 * does. */
export function focusHandoverSoon(key: string): void {
  pending = { key, at: Date.now() };
  window.setTimeout(() => {
    if (pending?.key === key) takeFor(key);
  }, 0);
}

/** A switch list for the session `key`: `take` runs when a request names
 * it (or names no one while this page is the one showing), or at mount
 * when its request came a moment before the page opened. */
export function useHandoverFocus(take: () => void, key: string, active = true): void {
  const ref = useRef(take);
  ref.current = take;
  useEffect(() => {
    if (!active) return;
    const listener: Listener = { key, take: () => ref.current() };
    listeners.push(listener);
    if (pending && pending.key === key && Date.now() - pending.at < PENDING_MS) {
      pending = null;
      requestAnimationFrame(() => ref.current());
    }
    return () => {
      const at = listeners.indexOf(listener);
      if (at >= 0) listeners.splice(at, 1);
    };
  }, [active, key]);
}

const LIVE = new Set(["running", "waiting_user", "stuck", "starting", "quota_blocked"]);

/** The session to continue for an account that ran out: its newest open
 * one, AI Juggler's or another app's (the 0.2 walk, F16: the account had
 * a session open in the Claude app and the banner opened the account). */
export function openOnAccount(
  accountId: string | null | undefined,
  sessions: InstanceInfo[],
  conversations: ConversationInfo[],
): { session: InstanceInfo } | { conversation: ConversationInfo } | null {
  if (!accountId) return null;
  const when = (ms: number | null | undefined, start: number | null = 0) => ms ?? start ?? 0;
  const own = sessions
    .filter((s) => s.account_id === accountId && LIVE.has(s.status))
    .map((session) => ({ at: when(session.last_activity_ms, session.started_at_ms), pick: { session } }));
  const others = conversations
    .filter((c) => c.account_id === accountId && c.live && c.session_id === null)
    .map((conversation) => ({ at: when(conversation.last_activity_ms, conversation.started_ms), pick: { conversation } }));
  return [...own, ...others].sort((a, b) => b.at - a.at)[0]?.pick ?? null;
}

/** What opened Continue on another account, kept for the switch a pick
 * opens: Cancel there gives it the keyboard back (WCAG 2.4.3). Handed over
 * here, not read from the page, since a terminal that finishes loading in
 * between may have taken focus (0.3.2's keyboard walk). */
let switchOpener: { element: Element; at: number } | null = null;

export function noteSwitchOpener(element: Element | null): void {
  switchOpener = element ? { element, at: Date.now() } : null;
}

/** The opener noted a moment ago, once; else null. */
function takeSwitchOpener(): Element | null {
  const noted = switchOpener;
  switchOpener = null;
  return noted && Date.now() - noted.at < PENDING_MS && noted.element.isConnected ? noted.element : null;
}

/** What a switch's Cancel gives the keyboard back to: the opener noted
 * above, else what had focus as the switch opened. Taken when it mounts,
 * never while it renders: a render React sets aside (a part still
 * loading) spent it, and the one kept read the terminal that had taken
 * focus meanwhile (0.4.4, the a11y gate's keyboard walk). Before any
 * effect of the switch moves focus into it. */
export function useSwitchInvoker(): RefObject<Element | null> {
  const invoker = useRef<Element | null>(null);
  useLayoutEffect(() => {
    if (invoker.current === null) invoker.current = takeSwitchOpener() ?? document.activeElement;
  }, []);
  return invoker;
}
