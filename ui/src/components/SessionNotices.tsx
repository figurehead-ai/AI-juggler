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
 */
import { useEffect, useState, type JSX } from "react";
import { Banner } from "./Banner";
import { usePagedList } from "../lib/paged";
import { notificationsDismiss, notificationsPage, type NoticeInfo } from "../lib/ipc/notices";
import { acted } from "../lib/notices-store";
import { useNoticeActions } from "../lib/notice-actions";
import { quotaDetail, secondOf } from "../lib/notice-words";
import { resetQueueDismiss, resetQueueList, type ResetQueueRow } from "../lib/ipc/alerts";
import { parkedDismiss, parkedOffers, parkedResume } from "../lib/ipc/park";
import type { ParkedOfferInfo } from "../types/generated/ParkedOfferInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { accountOfProfile, useSubscriptions, type SubscriptionInfo } from "../lib/subscriptions";
import { focusHandoverSoon } from "../lib/handover-focus";

/** Whether folders `a` and `b` share their room: the same folder, or two
 * folders of one Account (D-62). */
export function sameRoom(subscriptions: readonly SubscriptionInfo[] | null, a: string | null | undefined, b: string | null | undefined): boolean {
  if (!a || !b) return false;
  if (a === b) return true;
  const of = (id: string) => (subscriptions ? accountOfProfile(subscriptions, id)?.account_id : undefined);
  const left = of(a);
  return left !== undefined && left !== null && left === of(b);
}

/** This session's account in words: "This session's account, Work Max,
 * is almost out of room". */
export function ownTitle(notice: NoticeInfo): string {
  const name = notice.facts.account;
  const out = notice.meter !== null && notice.meter <= 0;
  return `This session's account${name ? `, ${name},` : ""} ${out ? "is out of room" : "is almost out of room"}`;
}

export function SessionNotices({
  accountId,
  switchKey,
  reminderKey,
  parkedId,
  sessions,
}: {
  /** The folder the session runs on. */
  accountId: string | null;
  /** The key its switch list answers to (focusHandoverSoon). */
  switchKey: string;
  /** Its reminder's key in the reset queue (a session's id, or `conv:` and
   * another app's session's id). */
  reminderKey: string;
  /** AI Juggler's own session's id, for its parked offer. */
  parkedId: string | null;
  sessions: InstanceInfo[];
}): JSX.Element | null {
  const subscriptions = useSubscriptions(0, accountId !== null);
  const actions = useNoticeActions(sessions);
  const banners = usePagedList<NoticeInfo>("notices:banners", (request) => notificationsPage("banners", request), { keyOf: (row) => String(row.id) });
  const [queue, setQueue] = useState<ResetQueueRow[]>([]);
  const [offers, setOffers] = useState<ParkedOfferInfo[]>([]);
  const [hidden, setHidden] = useState<ReadonlySet<string>>(() => new Set());
  const [now] = useState(() => Date.now());
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
  for (const notice of banners.items) {
    const key = `notice-${notice.id}`;
    if (hidden.has(key) || !sameRoom(subscriptions, notice.target.account_id, accountId)) continue;
    const second = secondOf(notice);
    items.push(
      <Banner
        key={key}
        tone="warning"
        title={ownTitle(notice)}
        detail={quotaDetail(notice, now)}
        meter={notice.meter !== null ? { used: 100 - notice.meter, label: `${100 - notice.meter}% used` } : undefined}
        actions={[
          // Its own switch, here on its page: never another session's.
          { label: "Continue this session in another app…", primary: true, onClick: () => focusHandoverSoon(switchKey), testId: "session-notice-hand-over" },
          ...(second?.id === "remind-reset" ? [{ label: second.label, onClick: () => actions.second(notice, "remind-reset"), testId: "session-notice-remind" }] : []),
          { label: "Snooze 1 h", quiet: true, testId: "session-notice-snooze", onClick: () => hideWhile(key, () => actions.snooze(notice, "hour")) },
        ]}
        onDismiss={() => hideWhile(key, () => acted(() => notificationsDismiss(notice.id)))}
        dismissLabel={`Close: ${ownTitle(notice)} (it stays in Notifications)`}
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

function without(set: ReadonlySet<string>, key: string): ReadonlySet<string> {
  const next = new Set(set);
  next.delete(key);
  return next;
}
