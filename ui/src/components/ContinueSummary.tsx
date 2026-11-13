/**
 * "Continue in another app" as a fold's title, on a session's page (D-25:
 * the switch list one step away). One look for both of its cards: the
 * terminal session's (HandoverSection) and, once a session has ended, any
 * other's (ContinueOn, D-103.3), whose opening starts the search for room.
 * Under it, folded, an ended session's account almost or quite out of
 * room is one quiet line (D-103.3), not a notice on its page.
 */
import { useState, type JSX } from "react";
import { ownQuietLine, useOwnRoomNotices } from "../lib/own-room-notices";

export function ContinueSummary(): JSX.Element {
  return (
    <summary className="-m-1 flex cursor-pointer list-none items-center justify-between gap-2 rounded p-1 text-section font-semibold text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60 [&::-webkit-details-marker]:hidden">
      Continue in another app
      <svg aria-hidden="true" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" className="shrink-0 text-muted transition-transform group-open:rotate-90 motion-reduce:transition-none">
        <path d="M9 6l6 6-6 6" />
      </svg>
    </summary>
  );
}

/** The quiet line: "This session's account, Work Max, is out of room ·
 * resets at 3:10 PM"; nothing while its account has room. */
export function AccountRoomLine({ accountId }: { accountId: string | null }): JSX.Element | null {
  const [notice] = useOwnRoomNotices(accountId || null);
  const [now] = useState(() => Date.now());
  if (!notice) return null;
  return (
    <p className="text-meta leading-[1.5] text-muted" data-testid="continue-account-room">
      {ownQuietLine(notice, now)}
    </p>
  );
}
