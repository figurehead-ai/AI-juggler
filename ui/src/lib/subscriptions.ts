/**
 * AJ-E2109 (D-35) — accounts and profiles, in words. An **account** is a
 * subscription you pay for; a **profile** is a setup an app runs on (what
 * AI Juggler stores per folder: `AccountInfo`). Profiles on one account
 * share one runway, so moving between them gives no new room.
 */
import { useEffect, useState } from "react";
import { subscriptionsList, type SubscriptionInfo } from "./ipc/subscriptions";
import { runwayOf } from "../components/AccountRunway";
import { toolSpec } from "./tools";
import type { QuotaDisplayData } from "./bridge";
import { whenForgotten } from "./shared-read";

export type { SubscriptionInfo };

/** Read once per half minute at most: each read checks every profile's
 * sign-in. */
const FRESH_MS = 30_000;
/** `value`: the last answer, kept so a view opened again shows it in its
 * first frame (0.3.4) while it's read again behind it. */
let cache: { at: number; rows: Promise<SubscriptionInfo[]>; value?: SubscriptionInfo[] } | null = null;

/** Every account, with its profiles (from the cache when fresh). A read
 * that fails keeps the last answer. */
export function loadSubscriptions(force = false): Promise<SubscriptionInfo[]> {
  if (!force && cache && Date.now() - cache.at < FRESH_MS) return cache.rows;
  const kept = cache?.value;
  const entry: NonNullable<typeof cache> = { at: Date.now(), rows: Promise.resolve([]), value: kept };
  entry.rows = subscriptionsList().then(
    (rows) => {
      entry.value = rows;
      return rows;
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
 * that doesn't show them). */
export function useSubscriptions(version = 0, wanted = true): SubscriptionInfo[] | null {
  const [rows, setRows] = useState<SubscriptionInfo[] | null>(() => (wanted ? cachedSubscriptions() : null));
  useEffect(() => {
    if (!wanted) return;
    let alive = true;
    void loadSubscriptions(version > 0).then((next) => alive && setRows(next));
    return () => {
      alive = false;
    };
  }, [version, wanted]);
  return rows;
}

/** The account a profile is signed in to. */
export function accountOfProfile(rows: SubscriptionInfo[], profileId: string): SubscriptionInfo | undefined {
  return rows.find((row) => row.profiles.some((profile) => profile.id === profileId));
}

/** Whether two profiles share one account (one runway). */
export function shareAccount(rows: SubscriptionInfo[], a: string, b: string): boolean {
  const account = accountOfProfile(rows, a);
  return account !== undefined && account.state === "signed_in" && account.profiles.some((profile) => profile.id === b);
}

/** An account's name: the names you gave its profiles (the 0.2 walk, F3:
 * five seats in one team all read the team's name), the one the rail,
 * tiles and filters use. One profile is its name; two are "A and B";
 * more are "A, B and 2 more". A group of profiles not signed in is "Not
 * signed in"; with no name at all, its tool and plan ("Claude Max"). */
export function accountName(row: SubscriptionInfo): string {
  if (row.state === "signed_out") return "Not signed in";
  const labels = [...new Set(row.profiles.map((profile) => profile.label?.trim()).filter((label): label is string => !!label))];
  if (labels.length === 1) return labels[0]!;
  if (labels.length === 2) return `${labels[0]} and ${labels[1]}`;
  if (labels.length > 2) return `${labels[0]}, ${labels[1]} and ${labels.length - 2} more`;
  // AJ-E1601: an account through another service is named for it.
  if (row.service) return row.service.name;
  if (row.organisation) return row.organisation;
  const tool = toolSpec(row.tool).short_name || toolSpec(row.tool).display_name;
  return row.plan ? `${tool} ${row.plan}` : `${tool} account`;
}

/** The account with its profiles' names as they are now (a name changed
 * since the accounts were read shows at once). */
export function withLiveLabels(row: SubscriptionInfo, accounts: readonly { id: string; label: string }[]): SubscriptionInfo {
  return { ...row, profiles: row.profiles.map((p) => ({ ...p, label: accounts.find((a) => a.id === p.id)?.label ?? p.label })) };
}

/** Its team, sign-in and plan, in one line ("Acme · o…@work.com · Max"):
 * the team is the second line, never the title. */
export function accountLine(row: SubscriptionInfo): string | null {
  if (row.service) return [row.service.name, serviceLine(row)].filter(Boolean).join(" · ");
  const parts = [row.organisation, row.email, row.plan].filter(Boolean);
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

/** The account's runway: one for all its profiles, from whichever of
 * them has a measured reading (the freshest), else the first's. */
export function accountDisplay(row: SubscriptionInfo, displays: QuotaDisplayData[]): QuotaDisplayData | undefined {
  const mine = row.profiles
    .map((profile) => displays.find((display) => display.account === profile.id))
    .filter((display): display is QuotaDisplayData => display !== undefined);
  const when = (display: QuotaDisplayData) => Math.max(display.five_hour?.measured_at_ms ?? 0, display.weekly?.measured_at_ms ?? 0);
  const measured = mine.filter((display) => runwayOf(display).left !== null).sort((a, b) => when(b) - when(a));
  return measured[0] ?? mine[0];
}
