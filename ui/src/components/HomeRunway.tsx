/**
 * AJ-E2105 — Runway by account: each account's room left (the tighter of
 * its windows, as measured), its meter, and when it resets ("Low" and the
 * sessions on it beside), or "Measure its limits ›" when it isn't measured
 * yet. D-118's mock: an 8 px disc, the name, the figure, a thin meter, one
 * line of facts; its plan or email and its Apps in the name's tooltip and
 * said to a screen reader.
 *
 * Accounts are subscriptions (D-35): profiles grouped by their sign-in
 * (AJ-E2109's `subscriptions_list`), one runway for all of an account's
 * profiles. Before that list answers, each profile stands for itself.
 * No summary sentence under the rows: it only repeated them (the 0.4
 * design review, 42); unknown room is one line, never also a "—".
 */
import { MeasureLimits } from "./MeasureLimits";
import { LOW_LEFT } from "../lib/room";
import type { JSX } from "react";
import { AccountAvatar } from "./Avatar";
import { HomeSection } from "./HomeSection";
import { attentionLevel, type AttentionLevel } from "../lib/attention";
import { signedIn } from "../lib/bridge";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import { newWorkAdvice, runwayDetail, runwayOf } from "./AccountRunway";
import { cliAppName } from "../lib/app-name";
import { namedAccountApps } from "../lib/account-items";
import { useApps, type AppModel } from "../lib/apps";
import type { CreditsCover } from "../lib/credits";
import { accountDisplay, accountLine, accountNamesApart, useSubscriptions, type SubscriptionInfo } from "../lib/subscriptions";
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
 * lists standing for itself. AJ-E3210 (D-135): an Account only a Claude
 * app profile is on (its own sign-in, no folder's CLI on it) is a row too,
 * with that App, unless `cliOnly` (where new work starts in a terminal). */
export function accountGroups(accounts: AccountInfo[], subscriptions: SubscriptionInfo[] | null, cliOnly = false): AccountGroup[] {
  const out: AccountGroup[] = [];
  const placed = new Set<string>();
  const shown: Array<{ subscription: SubscriptionInfo; profiles: AccountInfo[] }> = [];
  for (const subscription of subscriptions ?? []) {
    const profiles = accounts.filter((account) => subscription.profiles.some((profile) => profile.id === account.id));
    const appsOnly = !cliOnly && subscription.profiles.length === 0 && (subscription.apps?.length ?? 0) > 0;
    if (profiles.length === 0 && !appsOnly) continue;
    for (const profile of profiles) placed.add(profile.id);
    shown.push({ subscription, profiles });
  }
  // SK48-8 (the 0.5.0 walk, F3): twins listed together carry their tool.
  const named = accountNamesApart(shown.map((row) => row.subscription));
  for (const { subscription, profiles } of shown) {
    out.push({ key: subscription.key, name: named(subscription), detail: accountLine(subscription), profiles, subscription });
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
  /** How old its reading is, when it's old (F-rc3c-1). */
  age?: string | null;
}

/** An Account's one runway: its newest measured window over every folder
 * signed in to it, and the sessions on any of them. */
function groupRunway(group: AccountGroup, quota: QuotaDisplayData[], sessions: InstanceInfo[]): RunwayLine {
  const ids = new Set(group.profiles.map((profile) => profile.id));
  const display = group.subscription ? accountDisplay(group.subscription, quota) : quota.find((q) => q.account === group.profiles[0]?.id);
  return runwayOf(display, sessions.filter((s) => s.account_id !== null && ids.has(s.account_id)));
}

/** Where to start new work, by Account (D-62): the Runway card's rows,
 * named as it names them, never by a folder's App. */
export function accountAdvice(accounts: AccountInfo[], subscriptions: SubscriptionInfo[] | null, quota: QuotaDisplayData[], sessions: InstanceInfo[]) {
  const rows = accountGroups(accounts.filter((account) => signedIn(account.login_state)), subscriptions, true).map((group) => ({ name: group.name, group, run: groupRunway(group, quota, sessions) }));
  return { count: rows.length, ...newWorkAdvice(rows) };
}

/** The sessions running on an Account's folders now (its room matters
 * only while one does). */
function liveOn(group: AccountGroup, sessions: readonly InstanceInfo[]): number {
  const ids = new Set(group.profiles.map((profile) => profile.id));
  return sessions.filter((s) => s.account_id !== null && ids.has(s.account_id) && LIVE.has(s.status)).length;
}

const LIVE = new Set(["starting", "running", "waiting_user", "stuck", "quota_blocked"]);

/** A runway figure's and meter's colour by level: neutral, amber, red. */
const LEVEL_TEXT: Record<AttentionLevel, string> = { 0: "text-ink", 1: "text-ink", 2: "text-warning", 3: "text-danger" };
const LEVEL_FILL: Record<AttentionLevel, string> = { 0: "bg-muted", 1: "bg-muted", 2: "bg-warning", 3: "bg-danger" };

/** A runway row's level (D-117): the quota need's, from its room and live work. */
function runwayLevel(left: number | null, live: number): AttentionLevel {
  return left === null ? 0 : attentionLevel({ need: "quota", left, liveSessions: live });
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

/** An Account's Apps, said to a screen reader and in its name's tooltip
 * (D-118's mock draws the row as a name, its room, a meter and when it
 * resets; the Apps are one hover or the Account's page away). */
function appsWords(named: string[] | null): string | null {
  return named && named.length > 0 ? `Apps: ${named.join(", ")}` : null;
}

/** The line under the meter (the mock's "low · resets 3:13 PM · 2
 * sessions on it"): "Low" at 25 % or less with room left, when it resets
 * (else the runway's words), and the live sessions on it. */
export function runwayFacts(row: RunwayLine, live: number): string {
  const low = row.left !== null && row.left > 0 && row.left <= LOW_LEFT;
  const sessions = live > 0 ? `${live} ${live === 1 ? "session" : "sessions"} on it` : null;
  // Without a reset, the runway's words less the figure the row shows
  // ("58% left, from its latest session" → "from its latest session").
  // F-rc3c-1: with a reset, an old reading says how old after it.
  const when = row.resets ? runwayDetail(row) : (row.words.replace(/^\d+% left,?\s*/, "") || null);
  const lower = (part: string) => part.charAt(0).toLowerCase() + part.slice(1);
  const line = [low ? "low" : null, when && lower(when), sessions].filter(Boolean).join(" · ");
  return line.charAt(0).toUpperCase() + line.slice(1);
}

export function HomeRunway({ accounts, quota, quotaKnown = true, sessions }: { accounts: AccountInfo[]; quota: QuotaDisplayData[]; quotaKnown?: boolean; sessions: InstanceInfo[] }): JSX.Element {
  const subscriptions = useSubscriptions();
  const appModels = useApps(accounts);
  const lineOf = (group: AccountGroup): RunwayLine => groupRunway(group, quota, sessions);
  return (
    <HomeSection id="home-runway" label="Runway by account">
      <div className="flex flex-col rounded-xl bg-surface px-4 py-1.5" data-testid="home-runway">
        {accountGroups(accounts, subscriptions).map((group) => {
          const first = group.profiles[0]!;
          const row = lineOf(group);
          const live = liveOn(group, sessions);
          // D-117: neutral, amber at 10 % or less with a live session, red
          // when out with live work (the one level table).
          const level = runwayLevel(row.left, live);
          // 0.3.2: used up, but its credits carry its work on.
          const credits = row.credits != null;
          const out = !credits && row.left !== null && row.left <= 0;
          const apps = appsWords(appsOn(group, appModels));
          const named = [group.detail, apps].filter(Boolean).join(" · ");
          return (
            <div key={group.key} className="flex flex-col gap-1.5 py-2" data-testid="home-runway-account" data-level={level}>
              <div className="flex items-center justify-between gap-3 text-sm">
                <span className="flex min-w-0 items-center gap-2" title={named || undefined}>
                  {/* D-117 (c2): an 8 px disc in the Account's colour; a group
                      not signed in is no Account yet: neutral. */}
                  <AccountAvatar tone={group.subscription?.state === "signed_in" ? "colour" : "neutral"} account={group.subscription ?? { folder: first.id }} label={group.name} size={8} />
                  {group.subscription?.account_id ? (
                    // D-62 (AJ-E3205): the account's name opens its page.
                    <button type="button" onClick={() => openAccount(group.subscription!.account_id!)} title={`Open ${group.name}'s page`} className="truncate rounded text-left font-medium text-ink outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="home-runway-name">
                      {group.name}
                    </button>
                  ) : (
                    <span className="truncate font-medium text-ink">{group.name}</span>
                  )}
                  {named && <span className="sr-only" data-testid="home-runway-apps">, {named}</span>}
                </span>
                {/* Unknown room is said once, on its own line below (the 0.4
                    design review, 82): no "—" figure here. 0.4.1: an Account
                    out of room says Out, as everywhere else: a word in the
                    figure's place (red only with live work on it, D-117),
                    its reset on the line below. */}
                {out ? (
                  <span className={"shrink-0 text-body font-semibold " + LEVEL_TEXT[level]} data-testid="home-runway-out">Out</span>
                ) : (credits || row.left !== null) && (
                  <span className={"shrink-0 text-body font-semibold tabular-nums " + LEVEL_TEXT[level]}>
                    {credits ? "On credits" : `${row.left}% left`}
                  </span>
                )}
              </div>
              {/* Before the first reading its meter's track holds the row's
                  height, so the reading doesn't push what's below. */}
              {!quotaKnown && row.left === null && <div className="h-1 rounded-full bg-surface-3" aria-hidden="true" data-testid="home-runway-meter-pending" />}
              {row.left !== null && (
                <div className="h-1 overflow-hidden rounded-full bg-surface-3" role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={row.left} aria-label={`${group.name}: ${row.words}`}>
                  <div className={"h-full rounded-full " + LEVEL_FILL[level]} style={{ width: `${row.left}%` }} />
                </div>
              )}
              {credits ? (
                <span className="flex flex-col text-label" data-testid="home-runway-credits">
                  <span className="text-ink">{row.words}</span>
                  {row.resets && <span className="text-muted">{row.resets}</span>}
                </span>
              ) : row.left !== null ? (
                // F-rc3c-1: an old reading says how old after its reset.
                <span className="text-label text-muted" data-testid="home-runway-facts">{runwayFacts(row, live)}</span>
              ) : (
                <span className="flex flex-wrap items-baseline gap-x-2 text-label text-muted">
                {row.words}
                {/* 0.4.4: it measures here (the tool's own check), not a page. */}
                {(row.words.startsWith("Not measured") || row.words.startsWith("Last reading")) && (
                  <MeasureLimits folders={group.profiles.map((profile) => profile.id)} tool={first.provider} name={group.name} />
                )}
                </span>
              )}
            </div>
          );
        })}
      </div>
    </HomeSection>
  );
}
