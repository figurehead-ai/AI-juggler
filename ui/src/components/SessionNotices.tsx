/**
 * D-87 (2026-11-06; the maintainer's finding, reported in 0.3 and again in
 * 0.4): a session's page shows only what is about THIS session, inline
 * under its header, in its own words: its account almost or quite out of
 * room ("This session's account…", whose Continue opens this session's
 * own switch), the reminder it was given that's due, and its parked
 * session's offer. Every other account's or session's notice is in the
 * bell and the notification centre, never in a band over a page: a band
 * over one session's page about another session's account was nearly
 * clicked for the session on the page.
 * D-103.3: an ended session's page stays quiet: its account's notice is
 * one quiet line under "Continue in another app" there instead.
 */
import { useEffect, useState, type JSX } from "react";
import { useClock } from "../lib/clock";
import { Banner } from "./Banner";
import { notificationsDismiss } from "../lib/ipc/notices";
import { acted } from "../lib/notices-store";
import { useNoticeActions } from "../lib/notice-actions";
import { quotaDetail, secondOf } from "../lib/notice-words";
import { resetQueueDismiss, resetQueueList, type ResetQueueRow } from "../lib/ipc/alerts";
import { parkedDismiss, parkedOffers, parkedResume } from "../lib/ipc/park";
import type { ParkedOfferInfo } from "../types/generated/ParkedOfferInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { ownTitle, useOwnRoomNotices } from "../lib/own-room-notices";

export function SessionNotices({
  accountId,
  quiet = false,
  reminderKey,
  parkedId,
  sessions,
}: {
  /** The folder the session runs on. */
  accountId: string | null;
  /** D-103.3: it has ended; its account's notice is ContinueOn's quiet
   * line, not a notice here. */
  quiet?: boolean;
  /** Its reminder's key in the reset queue (a session's id, or `conv:` and
   * another app's session's id). */
  reminderKey: string;
  /** AI Juggler's own session's id, for its parked offer. */
  parkedId: string | null;
  sessions: InstanceInfo[];
}): JSX.Element | null {
  const actions = useNoticeActions(sessions);
  // rc33 M4: greyed with the switch's own reason when it would refuse.
  const own = useOwnRoomNotices(quiet ? null : accountId);
  const [queue, setQueue] = useState<ResetQueueRow[]>([]);
  const [offers, setOffers] = useState<ParkedOfferInfo[]>([]);
  const [hidden, setHidden] = useState<ReadonlySet<string>>(() => new Set());
  const now = useClock(); // SK48-6: the app's one clock, never the page's opening
  useEffect(() => {
    let alive = true;
    const poll = () => {
      resetQueueList().then((rows) => alive && setQueue(rows), () => null);
      if (parkedId) parkedOffers().then((rows) => alive && setOffers(rows), () => null);
    };
    poll();
    const timer = window.setInterval(poll, 30_000); // ≥30 s — never a busy loop
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [parkedId]);
  /** Hides it now; the write follows, and a refusal puts it back. */
  const hideWhile = (key: string, write: () => Promise<unknown>) => {
    setHidden((prior) => new Set(prior).add(key));
    void write().then(
      (done) => done === false && setHidden((prior) => without(prior, key)),
      () => setHidden((prior) => without(prior, key)),
    );
  };
  const items: JSX.Element[] = [];
  for (const notice of own) {
    const key = `notice-${notice.id}`;
    if (hidden.has(key)) continue;
    const second = secondOf(notice);
    items.push(
      <Banner
        key={key}
        tone="warning"
        title={ownTitle(notice)}
        detail={quotaDetail(notice, now)}
        meter={notice.meter !== null ? { used: 100 - notice.meter, label: `${100 - notice.meter}% used` } : undefined}
        actions={[
          // AJ-E1548 #13 (D-143, PAT-SW-3): the facts and Remind/Snooze; the
          // side column's Switch account… is the page's one door to a switch.
          ...(second?.id === "remind-reset" ? [{ label: second.label, onClick: () => actions.second(notice, "remind-reset"), testId: "session-notice-remind" }] : []),
          { label: "Snooze 1 h", quiet: true, testId: "session-notice-snooze", onClick: () => hideWhile(key, () => actions.snooze(notice, "hour")) },
        ]}
        onDismiss={() => hideWhile(key, () => acted(() => notificationsDismiss(notice.id)))}
        dismissLabel={`Close: ${ownTitle(notice)} (it stays in Notifications)`}
        className={SHORT_WINDOW_FIT}
        testId="session-notice-quota"
      />,
    );
  }
  for (const row of queue) {
    const key = `reminder-${row.id}`;
    if (row.state !== "ready" || row.session !== reminderKey || hidden.has(key)) continue;
    items.push(
      <Banner
        key={key}
        tone="accent"
        title="This session's account has room again: it can carry on"
        detail="The reminder you set. Nothing restarted by itself."
        onDismiss={() => hideWhile(key, () => resetQueueDismiss(row.id))}
        dismissLabel="Close the reminder"
        testId="session-notice-reminder"
      />,
    );
  }
  for (const offer of offers) {
    const key = `parked-${offer.session_id}`;
    if (offer.session_id !== parkedId || hidden.has(key)) continue;
    const gone = () => setOffers((rows) => rows.filter((r) => r.session_id !== offer.session_id));
    items.push(
      <Banner
        key={key}
        tone="accent"
        title={offer.text}
        actions={[
          { label: "Resume", primary: true, testId: "session-notice-resume", onClick: () => hideWhile(key, () => parkedResume(offer.session_id).then(gone)) },
          { label: offer.last ? "Stop reminding me" : "Remind me in 30 min", quiet: true, testId: offer.last ? "session-notice-parked-stop" : "session-notice-parked-snooze", onClick: () => hideWhile(key, () => parkedDismiss(offer.session_id).then(gone)) },
        ]}
        testId="session-notice-parked"
      />,
    );
  }
  if (items.length === 0) return null;
  return (
    <output aria-live="polite" className="flex flex-col gap-2" data-testid="session-notices">
      {items}
    </output>
  );
}

/** AJ-E2314 L2 (the 0.5.0 re-walk, 1024×680): with this notice up, the
 * session page left its side column a 204 px window onto 723. In a window
 * under 720 px tall the notice takes one paragraph instead of three lines:
 * its title and detail run on, the meter goes (the detail says the same
 * "5% … left"), and its padding tightens, so the column gets that height. */
export const SHORT_WINDOW_FIT =
  "[@media(max-height:719px)]:py-2 [@media(max-height:719px)]:[&>div>p]:inline [@media(max-height:719px)]:[&>div>p]:mr-1.5 [@media(max-height:719px)]:[&>div>p>span]:inline [@media(max-height:719px)]:[&>div>p+div]:inline [@media(max-height:719px)]:[&_[role=meter]]:hidden";

function without(set: ReadonlySet<string>, key: string): ReadonlySet<string> {
  const next = new Set(set);
  next.delete(key);
  return next;
}
