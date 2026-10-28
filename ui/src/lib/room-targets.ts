/**
 * The switch's targets in the one order by room (`lib/room`), with each
 * one's room as Home's Runway reads it (`runwayOf` on its account), and
 * "Suggested" on the best: Continue on another account, the switch card,
 * the limit dialog and the switch dialog all read this, so they agree with
 * Home's "Best for new work" (the 0.2 walk, F1). The backend's ranking
 * still explains each row; it no longer decides the order or the pick.
 */
import { runwayOf } from "../components/AccountRunway";
import type { QuotaDisplayData, SwitchTargetData } from "./bridge";
import { rankByRoom, type RoomKind } from "./room";
import { accountDisplay, accountOfProfile, shareAccount, type SubscriptionInfo } from "./subscriptions";

/** A target's room, in the one runway's words. */
export interface TargetRoom {
  left: number | null;
  kind: RoomKind;
  /** "75% left this week". */
  words: string;
  /** "Week resets Thu 08:00", when known. */
  resets: string | null;
  resetsAtMs: number | null;
}

export type RoomTarget = SwitchTargetData & { room: TargetRoom };

/** The room of the account a profile is on (its account's runway; the
 * profile's own before the accounts are read). */
export function roomOfProfile(profileId: string, accounts: SubscriptionInfo[] | null, displays: QuotaDisplayData[]): Omit<TargetRoom, "kind"> {
  const account = accounts ? accountOfProfile(accounts, profileId) : undefined;
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === profileId);
  const run = runwayOf(display);
  return { left: run.left, words: run.words, resets: run.resets, resetsAtMs: run.resetsAtMs ?? null };
}

/** `targets` by room, the best "Suggested". One the ranking holds back,
 * or on the source's own account (no new room), can't be suggested. */
export function rankTargets(
  targets: readonly SwitchTargetData[],
  accounts: SubscriptionInfo[] | null,
  displays: QuotaDisplayData[],
  sourceProfile: string | null,
): RoomTarget[] {
  const rooms = new Map(targets.map((target) => [target, roomOfProfile(target.account_id, accounts, displays)]));
  const usable = (target: SwitchTargetData) =>
    target.filtered_reason == null &&
    target.account_id !== sourceProfile &&
    !(accounts && sourceProfile && shareAccount(accounts, sourceProfile, target.account_id));
  return rankByRoom(targets, (target) => rooms.get(target)!.left, usable).map(({ item, kind, suggested }) => ({
    ...item,
    suggested,
    room: { ...rooms.get(item)!, kind },
  }));
}
