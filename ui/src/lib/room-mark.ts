/**
 * 0.4.1 (the maintainer's finding): one unmistakable mark on every
 * Account and App that's out of room, the same wherever it appears (the
 * rail, Home's apps, the Account's and App's headers, the pickers, the
 * top bar's count): an empty gauge, the word and when it resets, "Out ·
 * resets Sun 6:00 AM"; colour, icon and word together, never colour
 * alone. Low (at or under 25% left, the one rule that keeps an account
 * from being suggested, `lib/room`) gets the calmer amber "Low · 12%
 * left". Read from the one runway every screen reads (`runwayOf`; out on
 * both windows, the later reset, 0.4.4):
 * measured or computed windows only, an estimate says "about"; an
 * account whose credits carry its work on past a used-up window is not
 * out (`lib/credits`).
 */
import { runwayOf } from "../components/AccountRunway";
import { accountDisplay, accountOfProfile, type SubscriptionInfo } from "./subscriptions";
import { LOW_LEFT } from "./room";
import { whenWords } from "./time-words";
import type { QuotaDisplayData, WindowDisplay } from "./ipc/quota";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

export interface RoomMark {
  kind: "out" | "low";
  /** "Out · resets Sun 6:00 AM", "Low · 12% left". */
  label: string;
  /** Where a line is tight (the rail's App header): "Out", "Low"; how
   * much and when it resets are in its title. */
  short: string;
  /** The whole of it, for its tooltip and screen readers. */
  title: string;
  /** Worked out from an estimate: its words say "about". */
  about: boolean;
  resetsAtMs: number | null;
}

/** Whether the reading the runway uses is an estimate: the badge of the
 * window it reads (the one that keeps it out longest, when out), else the
 * session reading's confidence. */
function estimated(window: WindowDisplay | undefined, sessions: readonly InstanceInfo[]): boolean {
  if (window) return window.badge === "estimated";
  const newest = [...sessions].filter((s) => s.quota != null).sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms))[0];
  return newest?.quota?.confidence === "estimated";
}

/** The mark for one reading of an account's room; `null` when it has
 * room, isn't measured, or its credits carry it on. */
export function roomMarkOf(display: QuotaDisplayData | undefined, sessions: readonly InstanceInfo[] = [], now: number = Date.now()): RoomMark | null {
  const run = runwayOf(display, [...sessions]);
  if (run.left === null || run.credits) return null;
  return roomMarkFrom(run.left, run.resetsAtMs ?? null, estimated(run.window, sessions), now);
}

/** The mark for `left` % left (resetting at `resetsAtMs`), `about` when
 * that's an estimate; `null` with room. */
export function roomMarkFrom(left: number, resetsAtMs: number | null, about = false, now: number = Date.now()): RoomMark | null {
  const kind = left <= 0 ? "out" : left <= LOW_LEFT ? "low" : null;
  if (kind === null) return null;
  const when = resetsAtMs !== null && resetsAtMs > now ? `${about ? "about " : ""}${whenWords(resetsAtMs, now)}` : null;
  if (kind === "out") {
    return {
      kind,
      label: when ? `Out · resets ${when}` : "Out",
      short: "Out",
      title: when ? `Out of room until ${when}` : "Out of room; when it resets isn't known yet",
      about,
      resetsAtMs,
    };
  }
  const words = `${about ? "about " : ""}${left}% left`;
  return { kind, label: `Low · ${words}`, short: "Low", title: when ? `Low on room: ${words}, resets ${when}` : `Low on room: ${words}`, about, resetsAtMs };
}

/** The mark for folder `profileId`: its Account's room (two folders of
 * one Account share it, D-62), else its own. */
export function roomMarkOfProfile(
  profileId: string | null | undefined,
  accounts: readonly SubscriptionInfo[] | null,
  displays: readonly QuotaDisplayData[],
  sessions: readonly InstanceInfo[] = [],
  now: number = Date.now(),
): RoomMark | null {
  if (!profileId) return null;
  const account = accounts ? accountOfProfile([...accounts], profileId) : undefined;
  const display = account ? accountDisplay(account, [...displays]) : displays.find((d) => d.account === profileId);
  const theirs = account ? sessions.filter((s) => account.profiles.some((p) => p.id === s.account_id)) : sessions.filter((s) => s.account_id === profileId);
  return roomMarkOf(display, theirs, now);
}

/** An Account's mark as the top bar counts it, and Accounts' "Out"
 * filter shows it: signed in, by its own reading; `null` otherwise. */
export function accountMark(account: SubscriptionInfo, displays: readonly QuotaDisplayData[], now: number = Date.now()): RoomMark | null {
  return account.state === "signed_in" ? roomMarkOf(accountDisplay(account, [...displays]), [], now) : null;
}

/** How many Accounts are out, and how many low (the top bar's count):
 * each Account once, whichever of its folders the readings came from. */
export function roomCounts(accounts: readonly SubscriptionInfo[] | null, displays: readonly QuotaDisplayData[], now: number = Date.now()): { out: number; low: number } {
  let out = 0;
  let low = 0;
  for (const account of accounts ?? []) {
    const mark = accountMark(account, displays, now);
    if (mark?.kind === "out") out += 1;
    else if (mark?.kind === "low") low += 1;
  }
  return { out, low };
}

/** The top bar's words: "1 account out", "2 accounts out · 1 low",
 * "1 account low"; `null` when every account has room. */
export function roomCountWords({ out, low }: { out: number; low: number }): string | null {
  const plural = (n: number) => (n === 1 ? "account" : "accounts");
  if (out > 0) return low > 0 ? `${out} ${plural(out)} out · ${low} low` : `${out} ${plural(out)} out`;
  if (low > 0) return `${low} ${plural(low)} low`;
  return null;
}
