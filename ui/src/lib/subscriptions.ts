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

export type { SubscriptionInfo };

/** Read once per half minute at most: each read checks every profile's
 * sign-in. */
const FRESH_MS = 30_000;
let cache: { at: number; rows: Promise<SubscriptionInfo[]> } | null = null;

/** Every account, with its profiles (from the cache when fresh). */
export function loadSubscriptions(force = false): Promise<SubscriptionInfo[]> {
  if (!force && cache && Date.now() - cache.at < FRESH_MS) return cache.rows;
  const rows = subscriptionsList().catch(() => [] as SubscriptionInfo[]);
  cache = { at: Date.now(), rows };
  return rows;
}

/** The accounts, `null` while they load; `version` re-reads them. */
export function useSubscriptions(version = 0): SubscriptionInfo[] | null {
  const [rows, setRows] = useState<SubscriptionInfo[] | null>(null);
  useEffect(() => {
    let alive = true;
    void loadSubscriptions(version > 0).then((next) => alive && setRows(next));
    return () => {
      alive = false;
    };
  }, [version]);
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

/** An account's name: its team's, else its tool and plan ("Claude Max"). */
export function accountName(row: SubscriptionInfo): string {
  if (row.state === "signed_out") return "Not signed in";
  if (row.organisation) return row.organisation;
  const tool = toolSpec(row.tool).short_name || toolSpec(row.tool).display_name;
  return row.plan ? `${tool} ${row.plan}` : `${tool} account`;
}

/** Its sign-in and plan, in one line ("o…@work.com · Max"). */
export function accountLine(row: SubscriptionInfo): string | null {
  const parts = [row.email, row.organisation && row.plan ? row.plan : null].filter(Boolean);
  if (row.state === "unverified") parts.push(row.reason ?? "its sign-in can't be read");
  return parts.length > 0 ? parts.join(" · ") : null;
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
