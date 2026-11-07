/**
 * AJ-E2109 (D-35) — one account (a subscription you pay for) with the
 * profiles signed in to it beneath. The account holds the runway, the
 * plan, the renewal and the fee; its profiles share them. Profiles that
 * aren't signed in to anything sit last, under "Not signed in".
 */
import { AccountAppRows } from "../../pages/subscription/AccountApps";
import type { AccountApp } from "../../lib/account-items";
import type { AppModel } from "../../lib/apps";
import { useState, type JSX } from "react";
import { Card } from "../../components/primitives";
import { runwayOf } from "../../components/AccountRunway";
import { nothingRead, RunwayPanel } from "../../components/usage/Runway";
import { useWeekByProfile } from "../../components/usage/RunwayCards";
import { accountColor } from "../../lib/marks";
import { providerDisplayName, noLimitsWords } from "../../lib/providers";
import { renewalLabel } from "../../lib/renewal";
import { accountDisplay, accountLine, accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import { openAccount } from "../../lib/accounts-page";
import { ChooseChip } from "../../pages/subscription/TermChoice";
import { RenewalChip } from "../../components/RenewalChip";
import { ReplaceKeyModal, ServiceUsage } from "./ServiceParts";
import { toolSpec } from "../../lib/tools";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";
import type { QuotaDisplayData } from "../../lib/ipc/quota";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import { ProfileRow } from "./ProfileRow";
import { OWN_APP_ID } from "../../components/AppIcon";
import { appOfSession, type AppRef } from "../../lib/app-of";

const LIVE = ["starting", "running", "waiting_user", "stuck", "quota_blocked"];

/** AJ-E2907 R3: AI Juggler's live sessions per launch settings, so the
 * window refuses a delete before the click while one runs. */
export function liveByRecipe(sessions: InstanceInfo[]): Record<string, number> {
  const out: Record<string, number> = {};
  for (const session of sessions) {
    if (LIVE.includes(session.status)) out[session.launch_recipe_id] = (out[session.launch_recipe_id] ?? 0) + 1;
  }
  return out;
}

/** How many sessions are open on a profile, and in which apps (each with
 * what its icon is read by, D-38). */
export function openOn(profileId: string, sessions: InstanceInfo[], conversations: ConversationInfo[]): { count: number; apps: AppRef[] } {
  const own = sessions.filter((s) => s.account_id === profileId && LIVE.includes(s.status));
  const elsewhere = conversations.filter((c) => c.account_id === profileId && c.live && c.session_id === null);
  const apps = new Map<string, AppRef>();
  for (const s of own) {
    const app = s.kind === "desktop" ? { ...appOfSession(s), name: `${toolSpec(s.provider).desktop_app ?? providerDisplayName(s.provider)} app` } : { appId: OWN_APP_ID, appPid: null, name: "AI Juggler" };
    apps.set(app.name, app);
  }
  for (const c of elsewhere) {
    for (const place of c.places ?? []) {
      if (!place.app) continue;
      const name = toolSpec(c.tool).desktop_app === place.app ? `${place.app} app` : place.app;
      apps.set(name, { appId: place.app_id ?? null, appPid: place.app_pid ?? null, name });
    }
  }
  return { count: own.length + elsewhere.length, apps: [...apps.values()] };
}

/** Why an account has no reading now, and where one comes from: its
 * tool's own records (the desktop app's meter while it runs, a limit hit),
 * never a dead end (the maintainer's third report). */
export function unmeasuredWords(tool: string): string {
  const spec = toolSpec(tool);
  // Only a tool whose app keeps a meter AI Juggler reads says so (the
  // Codex app's limits come in Codex's own records).
  const app = spec.desktop_app && spec.supports.app_meter ? `the ${spec.desktop_app} app's meter while it's open on this account` : null;
  const name = providerDisplayName(tool);
  return app
    ? `No current reading. Limits come from ${app}, and from ${name} when it hits a limit.`
    : `No current reading. Limits come from ${name}'s own records when it reports them.`;
}

/** "Not measured yet ▸ · Measure its limits ›": the unknown said once;
 * where limits come from folds behind it (the 0.4 review, 63). */
function Unmeasured({ tool, onMeasure }: { tool: string; onMeasure?: () => void }): JSX.Element {
  const [why, setWhy] = useState(false);
  const link = "rounded text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60";
  return (
    <div className="flex flex-col gap-1 text-meta text-ink-2" data-testid="account-unmeasured">
      <p className="flex flex-wrap items-center gap-x-1.5">
        <button type="button" aria-expanded={why} onClick={() => setWhy(!why)} className="rounded font-medium text-ink-2 outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="account-unmeasured-fold">
          Not measured yet <span aria-hidden="true">{why ? "▾" : "▸"}</span>
        </button>
        {onMeasure && (
          <>
            <span aria-hidden="true">·</span>
            <button type="button" onClick={onMeasure} className={link} data-testid="account-measure">
              Measure its limits ›
            </button>
          </>
        )}
      </p>
      {why && (
        <p className="text-label text-muted" data-testid="account-unmeasured-why">
          {unmeasuredWords(tool)}
        </p>
      )}
    </div>
  );
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
  /** The accounts span more than one tool: the line names this one's. */
  toolWords?: boolean;
  /** Every App (D-62): its desktop ones on this account get rows too. */
  apps?: readonly AppModel[];
};

export function AccountCard(props: AccountCardProps): JSX.Element {
  const { subscription: sub, profiles } = props;
  const signedOut = sub.state === "signed_out";
  const display = accountDisplay(sub, props.displays);
  const ownSessions = props.sessions.filter((s) => profiles.some((p) => p.id === s.account_id));
  const unmeasured = runwayOf(display, ownSessions).left === null;
  // No window read: said in one line, not a dial (the 0.4 review).
  const quiet = nothingRead(display);
  const renewal = renewalLabel(sub.renewal_date, props.now);
  // AJ-E1601: an account through another service says which.
  const service = sub.service ?? null;
  const first = profiles[0];
  const [newKey, setNewKey] = useState(false);
  // D-62: the Account's own name is the title; the line says the rest
  // once (its tool in words only where the accounts span several).
  const name = accountName(sub);
  const line = [
    props.toolWords && !service ? providerDisplayName(sub.tool) : null,
    accountLine(sub),
    renewal && renewal.tier === "date" ? renewal.text : null,
    sub.fee_usd != null ? `$${sub.fee_usd}/month` : null,
  ].filter(Boolean);
  const week = useWeekByProfile();
  const folderIds = new Set(profiles.map((profile) => profile.id));
  const desktop: AccountApp[] = (props.apps ?? [])
    .filter((app) => app.kind === "desktop" && ((sub.account_id && app.accountId === sub.account_id) || (app.folder !== null && folderIds.has(app.folder.id))))
    .map((app) => ({ app, sessions: app.place?.recent.length ?? 0, open: 0, lastMs: app.place?.seen_ms ?? 0 }));
  return (
    <Card className="@container overflow-hidden" data-testid="account-card" data-state={sub.state} data-service={service?.id}>
      <div className="flex flex-wrap items-center gap-3 border-b border-hairline px-5 py-4">
        <span aria-hidden="true" className="size-2.5 shrink-0 rounded-full" style={{ background: accountColor(sub) }} title="This account's colour in the charts" />
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-section text-ink" data-testid="account-name">
            {sub.account_id ? (
              <button type="button" onClick={() => openAccount(sub.account_id!)} title={`Open ${name}'s page`} className="max-w-full truncate rounded text-left outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="account-page">
                {name}
              </button>
            ) : (
              name
            )}
          </h3>
          <p className="truncate text-meta text-ink-2" data-testid="account-line">
            {signedOut
              ? `${profiles.length === 1 ? "This app isn't" : "These apps aren't"} signed in, so ${profiles.length === 1 ? "its" : "their"} account isn't known yet.`
              : sub.state === "unverified" && !service
                ? `${providerDisplayName(sub.tool)} · ${sub.reason ?? "its sign-in can't be read"}`
                : line.join(" · ") || providerDisplayName(sub.tool)}
          </p>
        </div>
        <RenewalChip renewal={renewal} />
        {/* D-62 (AJ-E3202 req. 4): its Apps said different things; chosen once on its Settings. */}
        <ChooseChip sub={sub} onOpen={() => openAccount(sub.account_id!, "settings")} />
        {/* Every App listed below: its folders and the desktop Apps on them (walk A2). */}
        {profiles.length + desktop.length > 1 && !signedOut && (
          <span className="text-meta text-muted" data-testid="account-profiles-count">
            {profiles.length + desktop.length} apps share this account
          </span>
        )}
        {service && first && (
          <button type="button" className="text-label text-muted underline decoration-hairline underline-offset-2 hover:text-ink" onClick={() => setNewKey(true)} data-testid="account-new-key">
            {sub.state === "signed_in" ? "Replace key…" : "Add its key…"}
          </button>
        )}
      </div>
      {newKey && service && first && (
        <ReplaceKeyModal profileId={first.id} service={service} onClose={() => setNewKey(false)} onDone={() => (setNewKey(false), void props.onRechecked())} />
      )}
      {!signedOut && (
        <div className="flex flex-col gap-2 px-5 py-4">
          {service ? (
            <ServiceUsage sub={sub} service={service} displays={props.displays} />
          ) : props.reportsLimits(sub.tool) ? (
            <>
              {/* Nothing measured: said once, in one line, with no dial (the 0.4 review, 50 and 79). */}
              {/* AJ-E3205 R3: an Account's door opens the Account, where its limits and how to measure them are. */}
              {quiet && <Unmeasured tool={sub.tool} onMeasure={sub.account_id ? () => openAccount(sub.account_id!) : profiles[0] && props.onInspect ? () => props.onInspect!(profiles[0]!.id) : undefined} />}
              <RunwayPanel
                display={display}
                limitsProfile={display && !unmeasured ? display.account : null}
                usage={week.values(profiles.map((p) => p.id))}
                usageTotal={week.total(profiles.map((p) => p.id))}
                color={accountColor(sub)}
                name={name}
                now={props.now}
                compact
                unreadSaidAbove={quiet}
              />
            </>
          ) : (
            <p className="text-meta text-ink-2" data-testid="account-no-limits">
              {noLimitsWords(sub.tool)}.
            </p>
          )}
        </div>
      )}
      <ul className="divide-y divide-hairline border-t border-hairline" aria-label={`Apps on ${name}`}>
        {profiles.map((profile) => {
          const open = openOn(profile.id, props.sessions, props.conversations);
          return (
            <ProfileRow
              key={profile.id}
              account={profile}
              subscription={sub}
              recipes={props.recipes.filter((recipe) => recipe.account_id === profile.id)}
              openNow={open.count}
              liveByRecipe={liveByRecipe(props.sessions)}
              apps={open.apps}
              canUseInPlace={props.canUseInPlace(profile.provider)}
              reportsLimits={!service && props.reportsLimits(profile.provider)}
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
        {/* D-62: the desktop Apps signed in to it (a Claude app profile on
            one of its folders), each a door to its App page. */}
        {desktop.length > 0 && (
          <li className="px-3 py-2" data-testid="account-desktop-apps">
            <AccountAppRows apps={desktop} now={props.now} compact />
          </li>
        )}
      </ul>
    </Card>
  );
}
