/**
 * D-87, D-103.3 — a session's own account almost or quite out of room,
 * as its page says it: while the session runs, a notice inline under its
 * header (`SessionNotices`); once it has ended, one quiet line under
 * "Continue in another app" (`ContinueOn`): testers reading an ended
 * session twice called the amber notice shouting. Never another
 * account's: that one is in the bell.
 */
import { useMemo } from "react";
import { usePagedList } from "./paged";
import { notificationsPage, type NoticeInfo } from "./ipc/notices";
import { accountOfProfile, useSubscriptions, type SubscriptionInfo } from "./subscriptions";
import { resetsWords } from "./time-words";

/** Whether folders `a` and `b` share their room: the same folder, or two
 * folders of one Account (D-62). */
function sameRoom(subscriptions: readonly SubscriptionInfo[] | null, a: string | null | undefined, b: string | null | undefined): boolean {
  if (!a || !b) return false;
  if (a === b) return true;
  const of = (id: string) => (subscriptions ? accountOfProfile(subscriptions, id)?.account_id : undefined);
  const left = of(a);
  return left !== undefined && left !== null && left === of(b);
}

/** The notices about the room of the folder a session runs on (`null`:
 * none are read). */
export function useOwnRoomNotices(accountId: string | null): NoticeInfo[] {
  const subscriptions = useSubscriptions(0, accountId !== null);
  const banners = usePagedList<NoticeInfo>(accountId === null ? null : "notices:banners", (request) => notificationsPage("banners", request), { keyOf: (row) => String(row.id) });
  return useMemo(() => banners.items.filter((notice) => sameRoom(subscriptions, notice.target.account_id, accountId)), [banners.items, subscriptions, accountId]);
}

/** This session's account in words: "This session's account, Work Max,
 * is almost out of room". */
export function ownTitle(notice: NoticeInfo): string {
  const name = notice.facts.account;
  const out = notice.meter !== null && notice.meter <= 0;
  return `This session's account${name ? `, ${name},` : ""} ${out ? "is out of room" : "is almost out of room"}`;
}

/** The quiet line an ended session's page says it in: "This session's
 * account, Work Max, is out of room · resets at 3:10 PM". */
export function ownQuietLine(notice: NoticeInfo, now: number): string {
  const resets = notice.facts.resets_at_ms;
  return resets !== null && resets > now ? `${ownTitle(notice)} · ${resetsWords(resets, now)}` : ownTitle(notice);
}
