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
 * out (`lib/credits`). F-rc3c-1: a mark from an old reading says how old
 * ("Out · resets Fri 12:00 PM · as of 2 days ago"), and one whose
 * window has reset since isn't a mark at all (`windowNow`).
 */
import { runwayOf } from "../components/AccountRunway";
import { accountDisplay, accountOfApp, accountOfProfile, listsApps, type SubscriptionInfo } from "./subscriptions";
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
  /** How old its reading is, when it's old ("as of 2 days ago"): in its
   * label and title. */
  age?: string | null;
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
  const run = runwayOf(display, [...sessions], now);
  if (run.left === null || run.credits) return null;
  return roomMarkFrom(run.left, run.resetsAtMs ?? null, estimated(run.window, sessions), now, run.age ?? null);
}

/** The mark for `left` % left (resetting at `resetsAtMs`), `about` when
 * that's an estimate, `age` how old its reading is when it's old
 * (`runwayOf`'s); `null` with room. */
export function roomMarkFrom(left: number, resetsAtMs: number | null, about = false, now: number = Date.now(), age: string | null = null): RoomMark | null {
  const kind = left <= 0 ? "out" : left <= LOW_LEFT ? "low" : null;
  if (kind === null) return null;
  const when = resetsAtMs !== null && resetsAtMs > now ? `${about ? "about " : ""}${whenWords(resetsAtMs, now)}` : null;
  const aged = (words: string) => (age ? `${words} · ${age}` : words);
  if (kind === "out") {
    return {
      kind,
      label: aged(when ? `Out · resets ${when}` : "Out"),
      short: "Out",
      title: aged(when ? `Out of room until ${when}` : "Out of room; when it resets isn't known yet"),
      about,
      resetsAtMs,
      age,
    };
  }
  const words = `${about ? "about " : ""}${left}% left`;
  return { kind, label: aged(`Low · ${words}`), short: "Low", title: aged(when ? `Low on room: ${words}, resets ${when}` : `Low on room: ${words}`), about, resetsAtMs, age };
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

/** AJ-E3210 (D-135): the mark for an App, by the Account it's on: a
 * desktop App (`app:…`) by its own sign-in's Account (none while that
 * isn't known yet, never its folder's guess); a CLI App, or an App not
 * named, by its folder (`roomMarkOfProfile`), whose Account is its CLI's. */
export function roomMarkOfApp(appId: string | null | undefined, ...folder: Parameters<typeof roomMarkOfProfile>): RoomMark | null {
  const [, accounts, displays, , now = Date.now()] = folder;
  if (!appId?.startsWith("app:") || (accounts !== null && !listsApps(accounts))) return roomMarkOfProfile(...folder);
  const account = accounts ? accountOfApp(accounts, appId) : undefined;
  return account ? roomMarkOf(accountDisplay(account, [...displays]), [], now) : null;
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
