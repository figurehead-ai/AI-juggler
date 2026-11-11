/**
 * AJ-E2105 — Runway by account: each account's room left (the tighter of
 * its windows, as measured), its meter, the profiles that use it, and
 * when it resets, or "Measure its limits ›" when it isn't measured yet.
 *
 * Accounts are subscriptions (D-35): profiles grouped by their sign-in
 * (AJ-E2109's `subscriptions_list`), one runway for all of an account's
 * profiles. Before that list answers, each profile stands for itself.
 * No summary sentence under the rows: it only repeated them (the 0.4
 * design review, 42); unknown room is one line, never also a "—".
 */
import { RoomChip } from "./RoomChip";
import { roomMarkFrom } from "../lib/room-mark";
import type { JSX } from "react";
import { AccountAvatar } from "./Avatar";
import { SectionTitle } from "./SectionTitle";
import { useActionHost } from "./ItemMenu";
import { signedIn } from "../lib/bridge";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import { newWorkAdvice, runwayOf } from "./AccountRunway";
import { cliAppName } from "../lib/app-name";
import { namedAccountApps } from "../lib/account-items";
import { useApps, type AppModel } from "../lib/apps";
import type { CreditsCover } from "../lib/credits";
import { accountDisplay, accountLine, accountName, useSubscriptions, type SubscriptionInfo } from "../lib/subscriptions";
import { openAccount } from "../lib/accounts-page";

export interface AccountGroup {
  key: string;
  name: string;
  /** Its plan, or its email once accounts know it. */
  detail: string | null;
  profiles: AccountInfo[];
  /** The subscription, once the list has answered. */
  subscription: SubscriptionInfo | null;
}

/** Accounts with their profiles: by subscription, any profile no account
 * lists standing for itself. */
export function accountGroups(accounts: AccountInfo[], subscriptions: SubscriptionInfo[] | null): AccountGroup[] {
  const out: AccountGroup[] = [];
  const placed = new Set<string>();
  for (const subscription of subscriptions ?? []) {
    const profiles = accounts.filter((account) => subscription.profiles.some((profile) => profile.id === account.id));
    if (profiles.length === 0) continue;
    for (const profile of profiles) placed.add(profile.id);
    out.push({ key: subscription.key, name: accountName(subscription), detail: accountLine(subscription), profiles, subscription });
  }
  for (const account of accounts.filter((account) => !placed.has(account.id))) {
    out.push({ key: account.id, name: account.label, detail: account.plan, profiles: [account], subscription: null });
  }
  return out;
}

export interface RunwayLine {
  left: number | null;
  words: string;
  resets: string | null;
  /** A used-up window its credits carry on past (0.3.2). */
  credits?: CreditsCover | null;
}

/** An Account's one runway: its newest measured window over every folder
 * signed in to it, and the sessions on any of them. */
export function groupRunway(group: AccountGroup, quota: QuotaDisplayData[], sessions: InstanceInfo[]): RunwayLine {
  const ids = new Set(group.profiles.map((profile) => profile.id));
  const display = group.subscription ? accountDisplay(group.subscription, quota) : quota.find((q) => q.account === group.profiles[0]!.id);
  return runwayOf(display, sessions.filter((s) => s.account_id !== null && ids.has(s.account_id)));
}

/** Where to start new work, by Account (D-62): the Runway card's rows,
 * named as it names them, never by a folder's App. */
export function accountAdvice(accounts: AccountInfo[], subscriptions: SubscriptionInfo[] | null, quota: QuotaDisplayData[], sessions: InstanceInfo[]) {
  const rows = accountGroups(accounts.filter((account) => signedIn(account.login_state)), subscriptions).map((group) => ({ name: group.name, group, run: groupRunway(group, quota, sessions) }));
  return { count: rows.length, ...newWorkAdvice(rows) };
}

/** Every App on an Account, each by its whole name (the 0.4 review): the
 * one rule every screen lists them by (rc22, A3: `namedAccountApps`);
 * `null` while they're read. */
export function appsOn(group: AccountGroup, apps: AppModel[] | null): string[] | null {
  const folder = (id: string) => group.profiles.find((one) => one.id === id);
  return namedAccountApps(apps, group.subscription, new Set(group.profiles.map((one) => one.id)), (id) => {
    const one = folder(id);
    return one ? cliAppName(one.provider, one.label) : id;
  })?.map((app) => app.name) ?? null;
}

/** An Account's Apps line; a placeholder while they're read. */
function RunwayApps({ named }: { named: string[] | null }): JSX.Element | null {
  if (named === null) return <span aria-hidden="true" className="h-3 w-48 animate-pulse rounded bg-skeleton" data-testid="home-runway-apps-pending" />;
  return named.length > 0 ? <span className="text-label text-muted">Apps: {named.join(", ")}</span> : null;
}

export function HomeRunway({ accounts, quota, quotaKnown = true, sessions }: { accounts: AccountInfo[]; quota: QuotaDisplayData[]; quotaKnown?: boolean; sessions: InstanceInfo[] }): JSX.Element {
  const host = useActionHost();
  const subscriptions = useSubscriptions();
  const apps = useApps(accounts);
  const lineOf = (group: AccountGroup): RunwayLine => groupRunway(group, quota, sessions);
  return (
    <section aria-labelledby="home-runway" className="flex min-w-0 flex-col gap-2.5" data-testid="home-runway">
      <SectionTitle id="home-runway">Runway by account</SectionTitle>
      <div className="flex flex-col rounded-xl border border-hairline bg-surface px-4 py-1.5">
        {accountGroups(accounts, subscriptions).map((group) => {
          const first = group.profiles[0]!;
          const row = lineOf(group);
          const low = row.left !== null && row.left <= 25;
          // 0.3.2: used up, but its credits carry its work on.
          const credits = row.credits != null;
          return (
            <div key={group.key} className="flex flex-col gap-1.5 border-t border-hairline/60 py-3 first:border-t-0" data-testid="home-runway-account">
              <div className="flex items-center justify-between gap-3 text-sm">
                <span className="flex min-w-0 items-center gap-2">
                  {/* An Account in its colour; a group not signed in is no Account yet: neutral. */}
                  <AccountAvatar tone={group.subscription?.state === "signed_in" ? "colour" : "neutral"} account={group.subscription ?? { folder: first.id }} label={group.name} size={18} />
                  <span className="flex min-w-0 flex-col">
                    {group.subscription?.account_id ? (
                      // D-62 (AJ-E3205): the account's name opens its page.
                      <button type="button" onClick={() => openAccount(group.subscription!.account_id!)} title={`Open ${group.name}'s page`} className="truncate rounded text-left font-medium text-ink outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="home-runway-name">
                        {group.name}
                      </button>
                    ) : (
                      <span className="truncate font-medium text-ink">{group.name}</span>
                    )}
                    {group.detail && <span className="truncate text-label text-muted">{group.detail}</span>}
                  </span>
                </span>
                {/* Unknown room is said once, on its own line below (the 0.4
                    design review, 82): no "—" figure here. */}
                {/* 0.4.1: an Account out of room says Out, as everywhere else
                    (its reset is the line below). */}
                {!credits && row.left !== null && row.left <= 0 ? (
                  <RoomChip mark={roomMarkFrom(row.left, null)} size="md" testId="home-runway-out" />
                ) : (credits || row.left !== null) && (
                  <span className={"shrink-0 text-body font-semibold tabular-nums " + (low ? "text-warning" : "text-ink")}>
                    {credits ? "On credits" : `${row.left}% left`}
                  </span>
                )}
              </div>
              {/* Before the first reading its meter's track holds the row's
                  height, so the reading doesn't push what's below. */}
              {!quotaKnown && row.left === null && <div className="h-1.5 rounded-full bg-surface-3" aria-hidden="true" data-testid="home-runway-meter-pending" />}
              {row.left !== null && (
                <div className="h-1.5 overflow-hidden rounded-full bg-surface-3" role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={row.left} aria-label={`${group.name}: ${row.words}`}>
                  <div className={"h-full rounded-full " + (low ? "bg-warning" : "bg-accent")} style={{ width: `${row.left}%` }} />
                </div>
              )}
              <RunwayApps named={appsOn(group, apps)} />
              {credits ? (
                <span className="flex flex-col text-label" data-testid="home-runway-credits">
                  <span className="text-ink">{row.words}</span>
                  {row.resets && <span className="text-muted">{row.resets}</span>}
                </span>
              ) : row.left !== null ? (
                <span className="text-label text-muted">{row.resets ?? row.words}</span>
              ) : (
                <span className="flex flex-wrap items-baseline gap-x-2 text-label text-muted">
                {row.words}
                {(row.words.startsWith("Not measured") || row.words.startsWith("Last reading")) && <button
                  type="button"
                  onClick={() => (group.subscription?.account_id ? openAccount(group.subscription.account_id) : host.openAccount?.(first))}
                  className="self-start rounded text-label text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
                >
                  Measure its limits ›<span className="sr-only"> — {group.name}</span>
                </button>}
                </span>
              )}
            </div>
          );
        })}
      </div>
    </section>
  );
}
