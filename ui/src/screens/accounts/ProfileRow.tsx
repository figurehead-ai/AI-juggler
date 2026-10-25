/**
 * AJ-E2109 (D-35) — one profile under its account: its badge, name, the
 * apps using it, its folder and how many sessions are open on it, Open,
 * and its menu (the registry's: start, sign in, launch settings, measure
 * its limits, rename, default, remove). Its launch settings unfold under
 * it on demand.
 */
import { useState, type JSX } from "react";
import { AccountAvatar } from "../../components/Avatar";
import { ToolMark } from "../../components/ToolMark";
import { Button, Chip } from "../../components/primitives";
import { useItemMenu } from "../../components/ItemMenu";
import { IconTerminal } from "../../components/icons";
import { ExpertSetup } from "../../components/ExpertSetup";
import { RecipeOptions } from "../../components/RecipeOptions";
import { launchSettingsName } from "../../lib/account-words";
import { providerDisplayName } from "../../lib/providers";
import { signedIn, type AccountInfo, type LaunchRecipeInfo } from "../../lib/bridge";
import { renewalLabel } from "../../lib/renewal";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { placementWords, UseOriginalModal } from "./UseOriginal";

export type ProfileRowProps = {
  account: AccountInfo;
  /** Its account, for the values its profiles don't agree on. */
  subscription: SubscriptionInfo | undefined;
  recipes: LaunchRecipeInfo[];
  /** Sessions open on it now (AI Juggler's and other apps'). */
  openNow: number;
  /** The apps those sessions run in ("Claude app, Terminal"). */
  apps: string[];
  /** Its folder, `~`-relative, when known. */
  folder: string | null;
  canUseInPlace: boolean;
  reportsLimits: boolean;
  now: number;
  onLaunch: (recipe: LaunchRecipeInfo) => void;
  onDeleteRecipe: (id: string) => Promise<void>;
  onNewRecipe: () => void;
  onLogin: () => void;
  onInspect?: () => void;
  onRechecked: () => Promise<void>;
};

export function ProfileRow(props: ProfileRowProps): JSX.Element {
  const { account, recipes, subscription } = props;
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [useOriginal, setUseOriginal] = useState(false);
  const [envFor, setEnvFor] = useState<string | null>(null);
  const loggedIn = signedIn(account.login_state);
  // AJ-E1601: through another service it starts with its account's key,
  // never a sign-in of its own.
  const service = account.service ?? null;
  const menu = useItemMenu("account", account, account.label, {
    openAccount: props.onInspect ? () => props.onInspect!() : undefined,
    startSessionOn: () => props.onLaunch(recipes[0]!),
    startBlockedWhy: () => (!loggedIn ? (service ? "Add its account's key first" : "Sign in first") : recipes.length === 0 ? "Add launch settings first" : null),
    signIn: loggedIn || service ? undefined : props.onLogin,
    editLaunchSettings: () => setSettingsOpen(true),
    measureLimits: props.reportsLimits && props.onInspect ? () => props.onInspect!() : undefined,
  });
  // Values its account's profiles disagree on show on each profile.
  const own = subscription?.profiles.find((p) => p.id === account.id);
  const differs = subscription?.differs ?? [];
  const renewal = differs.includes("renewal") ? renewalLabel(own?.renewal_date ?? null, props.now) : null;
  const perProfile = [
    differs.includes("plan") && own?.plan ? own.plan : null,
    renewal ? renewal.text : null,
    differs.includes("fee") && own?.fee_usd != null ? `$${own.fee_usd}/month` : null,
  ].filter(Boolean);
  const tool = service ? `${providerDisplayName(account.provider)} through ${service}` : providerDisplayName(account.provider);
  const where = [props.apps.length > 0 ? props.apps.join(", ") : tool, ...perProfile].join(" · ");
  return (
    <li className="flex flex-col" data-testid="profile-row" data-profile={account.id}>
      <div className="group flex flex-wrap items-center gap-3 px-5 py-2.5" {...menu.triggerProps}>
        <AccountAvatar accountId={account.id} label={account.label} size={28} live={props.openNow > 0} />
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="flex min-w-0 items-center gap-2">
            {service && <ToolMark tool={account.provider} size={13} className="text-muted" />}
            <button type="button" onClick={props.onInspect} title={`Open ${account.label}'s page`} className="truncate rounded text-[13px] font-semibold outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="account-label">
              {account.label}
            </button>
            {account.is_default && (
              <span title="New sessions start with this profile, unless a project picks its own" data-testid="account-default">
                <Chip tone="neutral">Default</Chip>
              </span>
            )}
          </span>
          <span className="flex min-w-0 items-center gap-1.5 text-[11px] text-muted">
            <span className="truncate" data-testid="profile-apps">{where}</span>
            {props.folder && (
              <span className="truncate font-mono text-[10.5px] text-muted" title={props.folder} data-testid="profile-folder">
                {props.folder}
              </span>
            )}
          </span>
          {placementWords(account) && (
            <span className="flex min-w-0 items-center gap-1.5 text-[11px]">
              <span className="truncate text-muted" title={account.import_source_path ?? undefined} data-testid="account-placement">
                {placementWords(account)}
              </span>
              {/* ADR-0018: a copy can switch to the folder it came from. */}
              {account.import_source_path && !account.in_place && props.canUseInPlace && (
                <button type="button" className="shrink-0 text-accent-hi hover:underline" onClick={() => setUseOriginal(true)} data-testid="account-use-original">
                  Use the original instead
                </button>
              )}
            </span>
          )}
        </div>
        <div className="ml-auto flex shrink-0 items-center gap-2">
          {loggedIn ? (
            <span className="text-[11px] text-muted" data-testid="account-activity">
              {props.openNow > 0 ? `${props.openNow} open now` : "Nothing open"}
            </span>
          ) : service ? (
            <span className="text-[11px] text-warning" data-testid="profile-key-missing">
              Its key isn't here
            </span>
          ) : (
            <>
              <span className="text-[11px] text-warning" data-testid="profile-signed-out">
                Not signed in
              </span>
              <Button size="sm" variant="secondary" onClick={props.onLogin}>
                Sign in
              </Button>
            </>
          )}
          <Button size="sm" variant="secondary" onClick={props.onInspect} disabled={!props.onInspect} data-testid="profile-open">
            Open
            <span className="sr-only"> {account.label}</span>
          </Button>
          {menu.button({ label: `Profile actions for ${account.label}`, shown: true, testId: "account-menu" })}
        </div>
      </div>
      {menu.element}
      {settingsOpen && (
        <LaunchSettings {...props} envFor={envFor} setEnvFor={setEnvFor} onClose={() => setSettingsOpen(false)} />
      )}
      {useOriginal && account.import_source_path && (
        <UseOriginalModal
          account={account}
          source={account.import_source_path}
          onClose={() => setUseOriginal(false)}
          onDone={async () => {
            setUseOriginal(false);
            await props.onRechecked();
          }}
        />
      )}
    </li>
  );
}

/** A profile's launch settings: each with its model, folder and options,
 * Launch, and a new set. */
function LaunchSettings(props: ProfileRowProps & { envFor: string | null; setEnvFor: (id: string | null) => void; onClose: () => void }): JSX.Element {
  const { account, recipes, envFor, setEnvFor } = props;
  return (
    <div className="mx-5 mb-3 divide-y divide-hairline rounded-xl border border-hairline bg-surface-2/40" data-testid="profile-launch-settings">
      <div className="flex items-center justify-between px-4 py-2">
        <span className="text-[11px] uppercase tracking-wider text-muted">Launch settings · used when you start on this profile</span>
        <span className="flex items-center gap-1">
          <button type="button" className="rounded-md px-2 py-1 text-[11px] text-muted hover:bg-surface hover:text-ink" onClick={props.onNewRecipe}>
            ＋ New settings set
          </button>
          <button type="button" className="rounded-md px-2 py-1 text-[11px] text-muted hover:bg-surface hover:text-ink" onClick={props.onClose} title="Close launch settings">
            Done
          </button>
        </span>
      </div>
      {recipes.length === 0 ? (
        <div className="px-4 py-3 text-xs text-muted">No launch settings yet — the default applies when you start.</div>
      ) : (
        recipes.map((recipe) => (
          <div key={recipe.id} className="group flex flex-wrap items-center gap-3 px-4 py-2.5">
            <span className="text-sm">{launchSettingsName(recipe.name)}</span>
            <Chip tone="neutral">{recipe.model ?? "default model"}</Chip>
            {recipe.default_cwd && <span className="truncate font-mono text-[11px] text-muted">{recipe.default_cwd}</span>}
            <button
              type="button"
              aria-expanded={envFor === recipe.id}
              onClick={() => setEnvFor(envFor === recipe.id ? null : recipe.id)}
              className="rounded-md px-2 py-1 text-[11px] text-muted hover:bg-surface hover:text-ink"
              data-testid="recipe-env-toggle"
            >
              Options {envFor === recipe.id ? "▾" : "▸"}
            </button>
            <div className="ml-auto flex items-center gap-1">
              <Button size="sm" variant="ghost" title="Delete launch settings" aria-label={`Delete ${launchSettingsName(recipe.name)}`} onClick={() => void props.onDeleteRecipe(recipe.id)}>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" aria-hidden="true">
                  <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
                </svg>
              </Button>
              <Button size="sm" variant="primary" disabled={!signedIn(account.login_state)} onClick={() => props.onLaunch(recipe)}>
                <IconTerminal width={13} height={13} />
                Launch
              </Button>
            </div>
            {envFor === recipe.id && (
              <div className="basis-full pt-1">
                <RecipeOptions provider={account.provider} accountId={account.id} recipe={recipe} />
              </div>
            )}
          </div>
        ))
      )}
      <div className="px-4 py-3">
        <ExpertSetup provider={account.provider} accounts={[{ id: account.id, label: account.label }]} surface="accounts" showMetricsExport />
      </div>
    </div>
  );
}
