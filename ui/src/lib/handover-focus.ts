/**
 * "Continue on another account" for the session whose page is open: ⌘⇧H,
 * the row menu, the header's Switch account…, an out-of-room banner and
 * Home's "Continue on another account…" all ask through here (the 0.2
 * walk, F8 and F16: ⌘⇧H did nothing on another app's session, and the
 * banner's Hand over… opened the account page).
 *
 * The page's switch list takes the keyboard: the switch card of a session
 * AI Juggler runs in its terminal, or Continue on another account on any
 * other session's page (one another app runs included). A page opened a
 * moment ago takes it when it mounts. When no session's page is open,
 * the asker hears so (`focusHandover` returns false) and says why.
 */
import { useEffect, useRef } from "react";
import type { InstanceInfo } from "./bridge";
import type { ConversationInfo } from "./ipc/conversations";

export const HANDOVER_EVENT = "aij:focus-handover";

/** A page that opens now may take the request for this long. */
const PENDING_MS = 3_000;
let pendingAt = 0;

/** Asks the open session's switch list for the keyboard. True when a
 * session's page took it. */
export function focusHandover(): boolean {
  pendingAt = Date.now();
  const taken = !window.dispatchEvent(new Event(HANDOVER_EVENT, { cancelable: true }));
  if (!taken) pendingAt = 0;
  return taken;
}

/** What ⌘⇧H says where no switch list is (Home, a project, an account,
 * another computer's session). */
export const NO_SESSION_OPEN = "Nothing here to continue on another account: ⌘⇧H works on the page of a session on this computer.";

/** The same, for a page that is only opening now (after selecting it). */
export function focusHandoverSoon(): void {
  pendingAt = Date.now();
  window.setTimeout(() => window.dispatchEvent(new Event(HANDOVER_EVENT, { cancelable: true })), 0);
}

/** A switch list: `take` runs when asked, or at mount when it was asked
 * a moment before the page opened. */
export function useHandoverFocus(take: () => void, active = true): void {
  const ref = useRef(take);
  ref.current = take;
  useEffect(() => {
    if (!active) return;
    if (Date.now() - pendingAt < PENDING_MS) {
      pendingAt = 0;
      requestAnimationFrame(() => ref.current());
    }
    const on = (event: Event) => {
      event.preventDefault();
      pendingAt = 0;
      ref.current();
    };
    window.addEventListener(HANDOVER_EVENT, on);
    return () => window.removeEventListener(HANDOVER_EVENT, on);
  }, [active]);
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
