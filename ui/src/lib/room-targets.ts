/**
 * The switch's targets in the one order by room (`lib/room`), with each
 * one's room as Home's Runway reads it (`runwayOf` on its account), and
 * "Suggested" on the best: Continue on another account, the switch card,
 * the limit dialog and the switch dialog all read this, so they agree with
 * Home's "Best for new work" (the 0.2 walk, F1). The backend's ranking
 * still explains each row; it no longer decides the order or the pick.
 */
import { runwayOf } from "../components/AccountRunway";
import type { QuotaDisplayData } from "./ipc/quota";
import type { SwitchTargetData } from "./ipc/switch-targets";
import { rankByRoom, type RoomKind } from "./room";
import type { CreditsCover } from "./credits";
import { accountDisplay, accountName, accountOfApp, accountOfProfile, listsApps, shareAccount, type SubscriptionInfo } from "./subscriptions";
import { ACCOUNT_NOT_KNOWN_YET } from "./account-words";

/** A target's room, in the one runway's words. */
export interface TargetRoom {
  left: number | null;
  kind: RoomKind;
  /** "75% left this week"; "Using credits · 14,150 left". */
  words: string;
  /** "Week resets Thu 08:00", when known. */
  resets: string | null;
  resetsAtMs: number | null;
  /** A used-up window its credits carry on past (0.3.2). */
  credits: CreditsCover | null;
  /** How old its reading is, when it's old ("as of 2 days ago",
   * F-rc3c-1): in `words`; after `resets` where they stand alone. */
  age?: string | null;
  /** D-62: whose room it is, its Account's name; unset or null for a
   * folder no Account lists (or before the accounts are read), which
   * keeps its own. */
  account?: string | null;
}

export type RoomTarget = SwitchTargetData & { room: TargetRoom };

/** The room of the account a profile is on (its account's runway; the
 * profile's own before the accounts are read). */
export function roomOfProfile(profileId: string, accounts: SubscriptionInfo[] | null, displays: QuotaDisplayData[]): Omit<TargetRoom, "kind"> {
  const account = accounts ? accountOfProfile(accounts, profileId) : undefined;
  const display = account ? accountDisplay(account, displays) : displays.find((d) => d.account === profileId);
  const run = runwayOf(display);
  return { left: run.left, words: run.words, resets: run.resets, resetsAtMs: run.resetsAtMs ?? null, credits: run.credits ?? null, age: run.age ?? null, account: account?.account_id ? accountName(account) : null };
}

/** AJ-E3210 (D-135): the room of the Account desktop App `appId` is on
 * (its own sign-in's, from its records), for a session it runs; before
 * that's known, "Account not known yet", never its folder's CLI's room.
 * A CLI App, or one not named, reads its folder's (`roomOfProfile`). */
export function roomOfApp(appId: string | null | undefined, profileId: string, accounts: SubscriptionInfo[] | null, displays: QuotaDisplayData[]): Omit<TargetRoom, "kind"> {
  if (!appId?.startsWith("app:") || !accounts || !listsApps(accounts)) return roomOfProfile(profileId, accounts, displays);
  const account = accountOfApp(accounts, appId);
  if (!account) return { left: null, words: ACCOUNT_NOT_KNOWN_YET, resets: null, resetsAtMs: null, credits: null, account: null };
  const run = runwayOf(accountDisplay(account, displays));
  return { left: run.left, words: run.words, resets: run.resets, resetsAtMs: run.resetsAtMs ?? null, credits: run.credits ?? null, account: accountName(account) };
}

/** The words for a folder whose tool isn't signed in in a terminal (the
 * rc5 walk: its account's "76% left" beside New session's "isn't signed
 * in"): never a percentage, never suggested. */
export const NOT_SIGNED_IN = "Not signed in · terminal";

/** 0.4.1 (main's rule: a label is true of what it labels): a target whose
 * terminal isn't signed in, but whose account's desktop app profile is
 * and takes the switch. */
export const DESKTOP_ONLY = "Desktop ✓ · Terminal: not signed in";

/** No room is its own to show: its terminal isn't signed in and no
 * desktop app profile takes it. */
export function roomless(target: Pick<SwitchTargetData, "signed_out" | "desktop">): boolean {
  return target.signed_out === true && target.desktop !== true;
}

/** Its sign-in, in words, when it isn't signed in everywhere; else `null`. */
export function signInWords(target: Pick<SwitchTargetData, "signed_out" | "desktop">): string | null {
  if (target.signed_out !== true) return null;
  return target.desktop === true ? DESKTOP_ONLY : NOT_SIGNED_IN;
}

function signedOutRoom(profileId: string, accounts: SubscriptionInfo[] | null): Omit<TargetRoom, "kind"> {
  const account = accounts ? accountOfProfile(accounts, profileId) : undefined;
  return { left: null, words: NOT_SIGNED_IN, resets: null, resetsAtMs: null, credits: null, account: account?.account_id ? accountName(account) : null };
}

/** `targets` by room, the best "Suggested". One the ranking holds back,
 * or on the source's own account (no new room), can't be suggested. The
 * source's own tool comes first among the ones with room (0.3.4). */
export function rankTargets(
  targets: readonly SwitchTargetData[],
  accounts: SubscriptionInfo[] | null,
  displays: QuotaDisplayData[],
  sourceProfile: string | null,
): RoomTarget[] {
  const rooms = new Map(targets.map((target) => [target, roomless(target) ? signedOutRoom(target.account_id, accounts) : roomOfProfile(target.account_id, accounts, displays)]));
  const usable = (target: SwitchTargetData) =>
    target.filtered_reason == null &&
    !roomless(target) &&
    target.account_id !== sourceProfile &&
    !(accounts && sourceProfile && shareAccount(accounts, sourceProfile, target.account_id));
  return rankByRoom(targets, (target) => rooms.get(target)!.left, usable, (target) => rooms.get(target)!.credits !== null, (target) => !target.cross_provider).map(({ item, kind, suggested }) => ({
    ...item,
    suggested,
    room: { ...rooms.get(item)!, kind },
  }));
}
