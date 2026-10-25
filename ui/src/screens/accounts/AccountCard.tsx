/**
 * AJ-E2109 (D-35) — one account (a subscription you pay for) with the
 * profiles signed in to it beneath. The account holds the runway, the
 * plan, the renewal and the fee; its profiles share them. Profiles that
 * aren't signed in to anything sit last, under "Not signed in".
 */
import type { JSX } from "react";
import { Card, Chip } from "../../components/primitives";
import { runwayOf } from "../../components/AccountRunway";
import { RunwayPanel } from "../../components/usage/Runway";
import { useWeekByProfile } from "../../components/usage/RunwayCards";
import { accountKeyColor } from "../../components/usage/accounts";
import { providerMark } from "../../components/indicator";
import { providerDisplayName, noLimitsWords } from "../../lib/providers";
import { renewalLabel, renewalRowVisible } from "../../lib/renewal";
import { accountDisplay, accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import { toolSpec } from "../../lib/tools";
import type { AccountInfo, InstanceInfo, LaunchRecipeInfo, QuotaDisplayData } from "../../lib/bridge";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import { ProfileRow } from "./ProfileRow";

const LIVE = ["starting", "running", "waiting_user", "stuck", "quota_blocked"];

/** How many sessions are open on a profile, and in which apps. */
export function openOn(profileId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): { count: number; apps: string[] } {
  const own = sessions.filter((s) => s.account_id === profileId && LIVE.includes(s.status));
  const elsewhere = conversations.filter((c) => c.account_id === profileId && c.live && c.session_id === null);
  const apps = new Set<string>();
  for (const s of own) apps.add(s.kind === "desktop" ? `${toolSpec(s.provider).desktop_app ?? providerDisplayName(s.provider)} app` : "AI Juggler");
  for (const c of elsewhere) {
    for (const place of c.places ?? []) {
      if (place.app) apps.add(toolSpec(c.tool).desktop_app === place.app ? `${place.app} app` : place.app);
    }
  }
  return { count: own.length + elsewhere.length, apps: [...apps] };
}

/** Why an account has no reading now, and where one comes from: its
 * tool's own records (the desktop app's meter while it runs, a limit hit),
 * never a dead end (the maintainer's third report). */
export function unmeasuredWords(tool: string): string {
  const spec = toolSpec(tool);
  const app = spec.desktop_app ? `the ${spec.desktop_app} app's meter while it's open on this profile` : null;
  const name = providerDisplayName(tool);
  return app
    ? `No current reading. Limits come from ${app}, and from ${name} when it hits a limit.`
    : `No current reading. Limits come from ${name}'s own records when it reports them.`;
}

export type AccountCardProps = {
  subscription: SubscriptionInfo;
  profiles: AccountInfo[];
  recipes: LaunchRecipeInfo[];
  displays: QuotaDisplayData[];
  sessions: InstanceInfo[];
  conversations: ConversationInfo[];
  now: number;
  canUseInPlace: (provider: string) => boolean;
  reportsLimits: (provider: string) => boolean;
  onLaunch: (recipe: LaunchRecipeInfo) => void;
  onDeleteRecipe: (id: string) => Promise<void>;
  onNewRecipe: (account: AccountInfo) => void;
  onLogin: (account: AccountInfo) => void;
  onInspect?: (id: string) => void;
  onRechecked: () => Promise<void>;
};

export function AccountCard(props: AccountCardProps): JSX.Element {
  const { subscription: sub, profiles } = props;
  const mark = providerMark(sub.tool);
  const signedOut = sub.state === "signed_out";
  const display = accountDisplay(sub, props.displays);
  const ownSessions = props.sessions.filter((s) => profiles.some((p) => p.id === s.account_id));
  const unmeasured = runwayOf(display, ownSessions).left === null;
  const renewal = renewalLabel(sub.renewal_date, props.now);
  const line = [
    sub.email,
    sub.organisation && sub.plan ? sub.plan : null,
    renewal && renewal.tier === "date" ? renewal.text : null,
    sub.fee_usd != null ? `$${sub.fee_usd}/month` : null,
  ].filter(Boolean);
  const week = useWeekByProfile();
  return (
    <Card className="@container overflow-hidden" data-testid="account-card" data-state={sub.state}>
      <div className="flex flex-wrap items-center gap-3 border-b border-hairline px-5 py-4">
        <span
          className="relative flex size-10 shrink-0 items-center justify-center rounded-xl border border-hairline bg-surface-2 text-ink"
          title={providerDisplayName(sub.tool)}
          data-testid="account-tool-mark"
          data-provider={sub.tool}
        >
          <mark.Icon width={18} height={18} aria-hidden="true" />
        </span>
        <span aria-hidden="true" className="size-2.5 shrink-0 rounded-full" style={{ background: accountKeyColor(sub) }} title="This account's colour in the charts" />
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-section text-ink" data-testid="account-name">
            {accountName(sub)}
          </h3>
          <p className="truncate text-meta text-muted" data-testid="account-line">
            {signedOut
              ? `${profiles.length === 1 ? "This profile isn't" : "These profiles aren't"} signed in, so ${profiles.length === 1 ? "its" : "their"} account isn't known yet.`
              : sub.state === "unverified"
                ? `${providerDisplayName(sub.tool)} · ${sub.reason ?? "its sign-in can't be read"}`
                : line.join(" · ") || providerDisplayName(sub.tool)}
          </p>
        </div>
        {renewal && renewalRowVisible(renewal) && renewal.tier !== "date" && (
          <Chip tone={renewal.tier === "danger" ? "danger" : "warning"}>
            {renewal.tier === "danger" ? "⛔" : "⚠"} {renewal.text}
          </Chip>
        )}
        {profiles.length > 1 && !signedOut && (
          <span className="text-meta text-muted" data-testid="account-profiles-count">
            {profiles.length} profiles share this account
          </span>
        )}
      </div>
      {!signedOut && (
        <div className="flex flex-col gap-2 px-5 py-4">
          {props.reportsLimits(sub.tool) ? (
            <>
              <RunwayPanel
                display={display}
                limitsProfile={display && !unmeasured ? display.account : null}
                usage={week.values(profiles.map((p) => p.id))}
                usageTotal={week.total(profiles.map((p) => p.id))}
                color={accountKeyColor(sub)}
                name={accountName(sub)}
                now={props.now}
                compact
              />
              {unmeasured && ownSessions.length === 0 && (
                <p className="text-meta text-muted" data-testid="account-unmeasured">
                  {unmeasuredWords(sub.tool)}{" "}
                  {profiles[0] && props.onInspect && (
                    <button type="button" onClick={() => props.onInspect!(profiles[0]!.id)} className="text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="account-measure">
                      Measure its limits…
                    </button>
                  )}
                </p>
              )}
            </>
          ) : (
            <p className="text-meta text-muted" data-testid="account-no-limits">
              {noLimitsWords(sub.tool)}.
            </p>
          )}
        </div>
      )}
      <ul className="divide-y divide-hairline border-t border-hairline" aria-label={`Profiles on ${accountName(sub)}`}>
        {profiles.map((profile) => {
          const open = openOn(profile.id, props.sessions, props.conversations);
          return (
            <ProfileRow
              key={profile.id}
              account={profile}
              subscription={sub}
              recipes={props.recipes.filter((recipe) => recipe.account_id === profile.id)}
              openNow={open.count}
              apps={open.apps}
              folder={sub.profiles.find((p) => p.id === profile.id)?.folder ?? null}
              canUseInPlace={props.canUseInPlace(profile.provider)}
              reportsLimits={props.reportsLimits(profile.provider)}
              now={props.now}
              onLaunch={props.onLaunch}
              onDeleteRecipe={props.onDeleteRecipe}
              onNewRecipe={() => props.onNewRecipe(profile)}
              onLogin={() => props.onLogin(profile)}
              onInspect={props.onInspect ? () => props.onInspect!(profile.id) : undefined}
              onRechecked={props.onRechecked}
            />
          );
        })}
      </ul>
    </Card>
  );
}
