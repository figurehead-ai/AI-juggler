/**
 * "Continue in another app" as a fold, on a session's page (D-25: the
 * switch list one step away). One look for both of its cards: the
 * terminal session's (HandoverSection) and, once a session has ended, any
 * other's (ContinueOn, D-103.3), whose opening starts the search for room.
 * Under it, folded, an ended session's account almost or quite out of
 * room is one quiet line (D-103.3), not a notice on its page.
 *
 * The app's Disclosure, a button an accessibility press opens (the 0.4.6
 * rc2 walk P9), kept by its card: it opens by itself when the limit hits.
 */
import type { JSX, ReactNode } from "react";
import { useClock } from "../lib/clock";
import { Disclosure } from "./controls";
import { ownQuietLine, useOwnRoomNotices } from "../lib/own-room-notices";

export function ContinueFold({ open, onToggle, testId, children }: { open: boolean; onToggle: (open: boolean) => void; testId: string; children: ReactNode }): JSX.Element {
  return (
    <Disclosure tone="title" open={open} onToggle={onToggle} summary={<span className="text-section">Continue in another app</span>} testId={testId}>
      {children}
    </Disclosure>
  );
}

/** The quiet line: "This session's account, Work Max, is out of room ·
 * resets at 3:10 PM"; nothing while its account has room. */
export function AccountRoomLine({ accountId }: { accountId: string | null }): JSX.Element | null {
  const [notice] = useOwnRoomNotices(accountId || null);
  const now = useClock(); // SK48-6: the app's one clock
  if (!notice) return null;
  return (
    <p className="text-meta leading-[1.5] text-muted" data-testid="continue-account-room">
      {ownQuietLine(notice, now)}
    </p>
  );
}
