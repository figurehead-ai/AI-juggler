/**
 * AJ-E2109 (D-35) — accounts and profiles, in words. An **account** is a
 * subscription you pay for; a **profile** is a setup an app runs on (what
 * AI Juggler stores per folder: `AccountInfo`). Profiles on one account
 * share one runway, so moving between them gives no new room.
 */
import { useEffect, useState } from "react";
import { subscriptionsList, type SubscriptionInfo } from "./ipc/subscriptions";
import { runwayOf } from "../components/AccountRunway";
import type { QuotaDisplayData } from "./ipc/quota";
import { whenForgotten } from "./shared-read";
import { everyWhileShown } from "./shown-window";

export type { SubscriptionInfo };

/** Read once per half minute at most: each read checks every profile's
 * sign-in. */
const FRESH_MS = 30_000;
/** `value`: the last answer, kept so a view opened again shows it in its
 * first frame (0.3.4) while it's read again behind it. */
let cache: { at: number; rows: Promise<SubscriptionInfo[]>; value?: SubscriptionInfo[] } | null = null;

/** Whether two answers say the same (their text; `false` when it can't
 * be told). */
function sameAnswer(a: SubscriptionInfo[], b: SubscriptionInfo[]): boolean {
  try {
    return JSON.stringify(a) === JSON.stringify(b);
  } catch {
    return false;
  }
}

/** Every account, with its profiles (from the cache when fresh). A read
 * that fails keeps the last answer; one that says what the last said
 * answers with it (the same rows: a view that reads again on a timer
 * isn't drawn again for nothing). */
export function loadSubscriptions(force = false): Promise<SubscriptionInfo[]> {
  if (!force && cache && Date.now() - cache.at < FRESH_MS) return cache.rows;
  const kept = cache?.value;
  const entry: NonNullable<typeof cache> = { at: Date.now(), rows: Promise.resolve([]), value: kept };
  entry.rows = subscriptionsList().then(
    (rows) => {
      const answer = kept && sameAnswer(rows, kept) ? kept : rows;
      entry.value = answer;
      return answer;
    },
    () => kept ?? [],
  );
  cache = entry;
  return entry.rows;
}

/** The last answer, if any (it may be older than half a minute). */
export function cachedSubscriptions(): SubscriptionInfo[] | null {
  return cache?.value ?? null;
}

/** Marks the accounts read as old (the next view reads them again, while
 * showing them): a sign-in started, an account added or changed. */
export function staleSubscriptions(): void {
  if (cache) cache.at = 0;
}

whenForgotten(() => (cache = null));

/** The accounts, `null` until known (the last answer at once, when there
 * is one); `version` re-reads them; `wanted` false reads nothing (a view
 * that doesn't show them). `everyMs` (F-rc3c-3): a view that shows them
 * with readings read again on a timer (an Out mark, the top bar's count)
 * reads them as often while the window is shown, from the half-minute
 * cache (one read for every such view), so a folder that moved to another
 * Account isn't counted in the one it left while the view stays drawn. */
export function useSubscriptions(version = 0, wanted = true, everyMs = 0): SubscriptionInfo[] | null {
  const [rows, setRows] = useState<SubscriptionInfo[] | null>(() => (wanted ? cachedSubscriptions() : null));
  useEffect(() => {
    if (!wanted) return;
    let alive = true;
    void loadSubscriptions(version > 0).then((next) => alive && setRows(next));
    return () => {
      alive = false;
    };
  }, [version, wanted]);
  useEffect(() => {
    if (!wanted || everyMs <= 0) return undefined;
    let alive = true;
    const stop = everyWhileShown(() => void loadSubscriptions(false).then((next) => alive && setRows(next)), everyMs);
    return () => {
      alive = false;
      stop();
    };
  }, [wanted, everyMs]);
  useRecheckSignIns(wanted ? rows : null, setRows);
  return rows;
}

/** 0.4.1: what an App says while its sign-in couldn't be checked: it
 * stays in its Account meanwhile ("Couldn't check its sign-in · …"). */
export function signInUncheckedWords(reason: string): string {
  return `Couldn't check its sign-in · ${reason}`;
}

/** The words for folder `folderId`'s App when its sign-in couldn't be
 * checked on the last read; `null` when it was, or isn't known. */
export function signInUnchecked(rows: readonly SubscriptionInfo[] | null | undefined, folderId: string | null | undefined): string | null {
  const reason = rows && folderId ? accountOfProfile(rows, folderId)?.profiles.find((p) => p.id === folderId)?.unchecked : null;
  return reason ? signInUncheckedWords(reason) : null;
}

/** 0.4.1: while any sign-in couldn't be checked, the accounts are read
 * again every half minute while the window is shown, so the words go as
 * soon as a read checks it. */
export function useRecheckSignIns(rows: readonly SubscriptionInfo[] | null, onRows: (rows: SubscriptionInfo[]) => void): void {
  const pending = rows !== null && rows.some((row) => row.profiles.some((p) => !!p.unchecked));
  useEffect(() => {
    let alive = true;
    const stop = pending ? everyWhileShown(() => void loadSubscriptions(false).then((next) => alive && onRows(next)), FRESH_MS) : null;
    return () => {
      alive = false;
      stop?.();
    };
    // `onRows` is a state setter, stable.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pending]);
}

/** The account a profile is signed in to. */
export function accountOfProfile(rows: readonly SubscriptionInfo[], profileId: string): SubscriptionInfo | undefined {
  return rows.find((row) => row.profiles.some((profile) => profile.id === profileId));
}

/** AJ-E3210 (D-135): the Account App `appId` is on, as the backend lists
 * each Account's Apps (`SubscriptionInfo.apps`, by each App's own
 * sign-in): a desktop App on its records' Account, a CLI App on its
 * folder's. `undefined` when it's on none (a desktop App whose Account
 * isn't known yet, or not signed in). */
export function accountOfApp(rows: readonly SubscriptionInfo[], appId: string): SubscriptionInfo | undefined {
  return rows.find((row) => row.apps?.includes(appId) ?? false);
}

/** Whether the rows say which Account each App is on (the window's list
 * does; the preview's and a list read without its Apps don't). */
export function listsApps(rows: readonly SubscriptionInfo[]): boolean {
  return rows.some((row) => row.apps !== undefined);
}

/** Whether two profiles share one account (one runway). */
export function shareAccount(rows: SubscriptionInfo[], a: string, b: string): boolean {
  const account = accountOfProfile(rows, a);
  return account !== undefined && account.state === "signed_in" && account.profiles.some((profile) => profile.id === b);
}

/** An account's name (D-62, AJ-E3202 req. 3): the one the backend gives
 * it (`display_name`, `subscriptions::name_all`): your name; else its
 * service's, team, tool and plan ("Claude Max 20x") or masked email, told
 * apart from a twin. Never a folder's or an app's name: those are Apps. */
export function accountName(row: SubscriptionInfo): string {
  return row.display_name;
}

/** What a limit, a reset or a renewal calls folder `folderId` (D-62: they
 * are its Account's, shared by its folders): its Account's name, as the
 * backend's `subscriptions::folder_name` says it. A folder no stored
 * Account lists (not signed in, a sign-in that can't be read), or any
 * before the accounts are read, keeps `own`, its own name. */
export function accountNameOf(rows: readonly SubscriptionInfo[] | null | undefined, folderId: string | null | undefined, own: string): string {
  const account = rows && folderId ? accountOfProfile(rows, folderId) : undefined;
  return account?.account_id ? accountName(account) : own;
}

/** A plan as it's said: "max-20x" → "Max 20x", "pro" → "Pro". */
export function planWords(plan: string): string {
  return plan
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

/** The account with its profiles' names as they are now (a name changed
 * since the accounts were read shows at once). */
export function withLiveLabels(row: SubscriptionInfo, accounts: readonly { id: string; label: string }[]): SubscriptionInfo {
  return { ...row, profiles: row.profiles.map((p) => ({ ...p, label: accounts.find((a) => a.id === p.id)?.label ?? p.label })) };
}

/** Its team, sign-in and plan, in one line ("Acme · o…@work.com · Max"),
 * leaving out what its name already says. */
export function accountLine(row: SubscriptionInfo, { email = true }: { email?: boolean } = {}): string | null {
  if (row.service) return [row.service.name, serviceLine(row)].filter(Boolean).join(" · ");
  const name = accountName(row);
  const parts = [row.organisation, email ? row.email : null, row.plan ? planWords(row.plan) : null].filter((part): part is string => !!part && !name.includes(part));
  if (row.state === "unverified") parts.push(row.reason ?? "its sign-in can't be read");
  return parts.length > 0 ? parts.join(" · ") : null;
}

/** AJ-E1601: an account through another service in one line — its plan
 * and how honest its usage is ("Go Plus · estimate"), or why its key
 * can't be used. */
export function serviceLine(row: SubscriptionInfo): string {
  if (row.state === "unverified") {
    const reason = row.reason ?? "its key isn't here";
    return reason.charAt(0).toUpperCase() + reason.slice(1);
  }
  const usage = row.service?.usage === "estimate" ? "estimate" : "usage unknown";
  return [row.plan, usage].filter(Boolean).join(" · ");
}

/** Whether a reading is Account `row`'s (AJ-E3210, D-135): one read
 * under an Account (`signed_in_as`) is that Account's only; one that says
 * none is its folder's, so its folder's Account's. */
export function displayIsAccounts(row: SubscriptionInfo, display: QuotaDisplayData): boolean {
  const under = display.signed_in_as ?? null;
  if (under !== null) return row.account_id != null && under === row.account_id;
  return row.profiles.some((profile) => profile.id === display.account);
}

/** The account's runway: one for all its profiles, from whichever of
 * them has a measured reading (the freshest), else the first's. AJ-E3210:
 * only the readings read under it, wherever they were read (an Account
 * only a Claude app profile is on reads its own on that profile's folder). */
export function accountDisplay(row: SubscriptionInfo, displays: QuotaDisplayData[]): QuotaDisplayData | undefined {
  const mine = displays.filter((display) => displayIsAccounts(row, display));
  const when = (display: QuotaDisplayData) => Math.max(display.five_hour?.measured_at_ms ?? 0, display.weekly?.measured_at_ms ?? 0);
  const measured = mine.filter((display) => runwayOf(display).left !== null).sort((a, b) => when(b) - when(a));
  return measured[0] ?? mine[0];
}
