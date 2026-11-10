/**
 * 0.1.5 — Usage's runway: one card per account (the subscription its
 * profiles share), its limits and its last week, each opening the
 * account's first profile page.
 */
import { openAccount } from "../../lib/accounts-page";
import { useMemo, type JSX } from "react";
import { useActionHost } from "../ItemMenu";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { QuotaDisplayData } from "../../lib/ipc/quota";
import { useQuotaDisplays } from "../../lib/kept-reads";
import { accountDisplay, accountLine, useSubscriptions } from "../../lib/subscriptions";
import { runwayOf } from "../AccountRunway";
import { BurnDowns } from "../BurnDown";
import { ProfileBadge } from "../ProfileBadge";
import { cliAppId, cliAppName } from "../../lib/app-name";
import { IconGauge } from "./controls";
import { accountGroups, runwayTitle, type AccountGroup } from "./accounts";
import { RunwayPanel } from "./Runway";
import { useApps, type AppModel } from "../../lib/apps";
import { appsOfAccount, type AccountOfApps } from "../../lib/account-items";
import type { AppMark } from "../../lib/marks";
import { useMinute, useUsageHistory } from "./useUsage";

/** Every account's two clocks, read now and each half minute (the last
 * answer, any screen's, at once). */
export function useDisplays(): QuotaDisplayData[] {
  return useQuotaDisplays().displays;
}

/** The last 7 days per profile, for the runway cards' usage lines. */
export function useWeekByProfile(): { values: (profiles: string[]) => number[] | null; total: (profiles: string[]) => number | null } {
  const week = useUsageHistory("7d", "profile", {}, 0);
  return useMemo(() => {
    const data = week.data;
    const pick = (profiles: string[]) => data?.series.filter((entry) => profiles.includes(entry.key)) ?? [];
    return {
      values: (profiles) => {
        if (!data) return null;
        const rows = pick(profiles);
        return data.edges_ms.slice(1).map((_, i) => rows.reduce((sum, entry) => sum + (entry.io[i] ?? 0), 0));
      },
      total: (profiles) => (data ? pick(profiles).reduce((sum, entry) => sum + entry.io.reduce((a, b) => a + b, 0), 0) : null),
    };
  }, [week.data]);
}

/** An Account's card's App chips: its Apps by the one rule every screen
 * uses (rc22, A3: `appsOfAccount`); before the Apps are read, its
 * folders' CLI Apps. */
export function runwayChips(apps: readonly AppModel[] | null, sub: AccountOfApps | null, folders: readonly string[], appNames: ReadonlyMap<string, string>): { key: string; name: string; mark: AppMark }[] {
  if (!apps) return folders.map((id) => ({ key: id, name: appNames.get(id) ?? "An app", mark: cliAppId(id) }));
  return appsOfAccount(apps, sub, new Set(folders)).map((app) => ({ key: app.id, name: app.name, mark: app.slot }));
}

export function RunwayCards({ accounts, className = "" }: { accounts: AccountInfo[]; className?: string }): JSX.Element {
  const displays = useDisplays();
  const subscriptions = useSubscriptions();
  const now = useMinute();
  const host = useActionHost();
  const week = useWeekByProfile();
  const apps = useApps(accounts);
  const labels = useMemo(() => new Map(accounts.map((account) => [account.id, account.label])), [accounts]);
  // Each App whole, as every screen names it ("Claude Code · Work Max"):
  // two Apps can share a folder's name (the 0.4 review).
  const appNames = useMemo(() => new Map(accounts.map((account) => [account.id, cliAppName(account.provider, account.label)])), [accounts]);
  const groups = useMemo(() => {
    const unique = new Map<string, AccountGroup>();
    for (const group of accountGroups(subscriptions, labels).values()) unique.set(group.key, group);
    return [...unique.values()];
  }, [subscriptions, labels]);
  return (
    <section aria-labelledby="usage-runway-title" className={"flex flex-col gap-3 " + className} data-testid="usage-runway">
      <h3 id="usage-runway-title" className="flex items-center gap-2 text-section text-ink">
        <IconGauge className="text-muted" />
        Runway and limit history
      </h3>
      <ul className="grid grid-cols-[repeat(auto-fill,minmax(min(100%,320px),1fr))] gap-grid">
        {groups.map((group) => {
          const sub = subscriptions?.find((entry) => entry.key === group.key);
          const display = sub ? accountDisplay(sub, displays) : displays.find((entry) => entry.account === group.profiles[0]);
          const measuredProfile = display && runwayOf(display).left !== null ? display.account : null;
          const first = accounts.find((account) => account.id === group.profiles[0]);
          const chips = runwayChips(apps, sub ?? null, group.profiles, appNames);
          const names = chips.map((chip) => chip.name);
          const title = runwayTitle(group, subscriptions);
          return (
            <li key={group.key} className="@container sheen flex min-w-0 flex-col gap-3.5 rounded-card border border-hairline bg-surface p-card shadow-card" data-testid="usage-account">
              <div className="flex items-center gap-3">
                <span aria-hidden="true" className="size-3 shrink-0 rounded-full" style={{ background: group.color }} />
                <span className="flex min-w-0 flex-1 flex-col">
                  {first ? (
                    <button type="button" onClick={() => (sub?.account_id ? openAccount(sub.account_id) : host.openAccount?.(first))} className="truncate text-left text-section text-ink outline-none hover:text-accent-hi focus-visible:ring-2 focus-visible:ring-accent/60">
                      {title}
                    </button>
                  ) : (
                    <span className="truncate text-section text-ink">{title}</span>
                  )}
                  <span className="truncate text-meta text-muted">{(sub && accountLine(sub)) ?? names.join(", ")}</span>
                </span>
              </div>
              <RunwayPanel display={display} limitsProfile={measuredProfile} usage={week.values(group.profiles)} usageTotal={week.total(group.profiles)} color={group.color} name={group.label} now={now} />
              {/* AJ-E1512: each window's sessions left and its burn-down. */}
              <BurnDowns display={display} />
              {chips.length > 0 && (
                <ul className="flex flex-wrap gap-1.5" aria-label={`Apps on ${group.label}`}>
                  {chips.map((chip) => (
                    <li key={chip.key} className="flex min-w-0 items-center gap-1.5 rounded-md border border-hairline bg-surface-2 px-2 py-0.5 text-label text-ink" data-testid="usage-account-app">
                      <ProfileBadge app={chip.mark} name={chip.name} size={6} />
                      <span className="truncate">{chip.name}</span>
                    </li>
                  ))}
                </ul>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
