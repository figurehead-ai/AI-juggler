/**
 * AJ-E2105 — Runway by account: each account's room left (the tighter of
 * its windows, as measured), its meter, the profiles that use it, and
 * when it resets, or "Measure its limits ›" when it isn't measured yet.
 *
 * Accounts are subscriptions (D-35): profiles grouped by their sign-in
 * (AJ-E2109's `subscriptions_list`), one runway for all of an account's
 * profiles. Before that list answers, each profile stands for itself.
 */
import type { JSX } from "react";
import { AccountAvatar } from "./Avatar";
import { SectionTitle } from "./SectionTitle";
import { useActionHost } from "./ItemMenu";
import type { AccountInfo, InstanceInfo, QuotaDisplayData } from "../lib/bridge";
import { runwayOf } from "./AccountRunway";
import type { CreditsCover } from "../lib/credits";
import { accountDisplay, accountLine, accountName, useSubscriptions, type SubscriptionInfo } from "../lib/subscriptions";

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

export function HomeRunway({ accounts, quota, quotaKnown = true, sessions, advice }: { accounts: AccountInfo[]; quota: QuotaDisplayData[]; quotaKnown?: boolean; sessions: InstanceInfo[]; advice: string | null }): JSX.Element {
  const host = useActionHost();
  const subscriptions = useSubscriptions();
  const lineOf = (group: AccountGroup): RunwayLine => {
    const ids = new Set(group.profiles.map((profile) => profile.id));
    const display = group.subscription ? accountDisplay(group.subscription, quota) : quota.find((q) => q.account === group.profiles[0]!.id);
    return runwayOf(display, sessions.filter((s) => s.account_id !== null && ids.has(s.account_id)));
  };
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
                  <AccountAvatar accountId={first.id} label={group.name} size={18} />
                  <span className="flex min-w-0 flex-col">
                    <span className="truncate font-medium text-ink">{group.name}</span>
                    {group.detail && <span className="truncate text-label text-muted">{group.detail}</span>}
                  </span>
                </span>
                <span className={"shrink-0 text-body font-semibold tabular-nums " + (low ? "text-warning" : row.left !== null ? "text-ink" : "text-muted")}>
                  {credits ? "On credits" : row.left !== null ? `${row.left}% left` : "—"}
                </span>
              </div>
              {/* Before the first reading its meter's track holds the row's
                  height, so the reading doesn't push what's below. */}
              {!quotaKnown && row.left === null && <div className="h-1.5 rounded-full bg-surface-3" aria-hidden="true" data-testid="home-runway-meter-pending" />}
              {row.left !== null && (
                <div className="h-1.5 overflow-hidden rounded-full bg-surface-3" role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={row.left} aria-label={`${group.name}: ${row.words}`}>
                  <div className={"h-full rounded-full " + (low ? "bg-warning" : "bg-accent")} style={{ width: `${row.left}%` }} />
                </div>
              )}
              <span className="text-label text-muted">Profiles: {group.profiles.map((profile) => profile.label).join(", ")}</span>
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
                  onClick={() => host.openAccount?.(first)}
                  className="self-start rounded text-label text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
                >
                  Measure its limits ›<span className="sr-only"> — {group.name}</span>
                </button>}
                </span>
              )}
            </div>
          );
        })}
        {advice && accounts.length > 1 && (
          <p className="border-t border-hairline/60 py-3 text-xs text-ink-2" data-testid="new-work-advice">
            {advice}
          </p>
        )}
      </div>
    </section>
  );
}
