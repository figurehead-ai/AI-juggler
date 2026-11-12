/**
 * AJ-E2109 (D-35) — one profile under its account: its badge, name, the
 * apps its sessions are open in and how many, Open, and its menu (the
 * registry's: start, sign in, launch settings, measure its limits,
 * rename, default, remove). Its launch settings unfold under it on
 * demand. Its folder is said on its App's page, and its renewal on its
 * Account's card, not here (the 0.4 review).
 */
import { AppIcon } from "../../components/AppIcon";
import type { AppRef } from "../../lib/app-of";
import { useState, type JSX } from "react";
import { CliAppMark } from "../../components/PlaceMarks";
import { AppKindBadge } from "../../components/AppKindBadge";
import { ToolMark } from "../../components/ToolMark";
import { Button, Chip } from "../../components/primitives";
import { useItemMenu } from "../../components/ItemMenu";
import { providerDisplayName } from "../../lib/providers";
import { cliAppName } from "../../lib/app-name";
import { signedIn } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";
import { planWords, signInUncheckedWords, type SubscriptionInfo } from "../../lib/subscriptions";
import { placementWords, UseOriginalModal } from "./UseOriginal";
import { LaunchSettingsList } from "./LaunchSettingsList";
export { liveWords } from "./LaunchSettingsList";


export type ProfileRowProps = {
  account: AccountInfo;
  /** Its account, for the values its profiles don't agree on. */
  subscription: SubscriptionInfo | undefined;
  recipes: LaunchRecipeInfo[];
  /** Sessions open on it now (AI Juggler's and other apps'). */
  openNow: number;
  /** AJ-E2907 R3: live sessions per launch settings id. */
  liveByRecipe?: Record<string, number>;
  /** The apps those sessions run in, each with its icon's ref. */
  apps: AppRef[];
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
  // Values its account's profiles disagree on show on each profile; but
  // not its renewal: that's the Account's, said on its card, where the
  // choice between two is asked (the 0.4 review, 17).
  const own = subscription?.profiles.find((p) => p.id === account.id);
  const differs = subscription?.differs ?? [];
  const perProfile = [differs.includes("plan") && own?.plan ? planWords(own.plan) : null, differs.includes("fee") && own?.fee_usd != null ? `$${own.fee_usd}/month` : null].filter(Boolean);
  const tool = service ? `${providerDisplayName(account.provider)} through ${service}` : providerDisplayName(account.provider);
  // D-62: the row is an App; its full name where the tool isn't beside it.
  const name = cliAppName(account.provider, account.label);
  // Where its sessions are open, said so: a Claude Code App "open in
  // Claude app, Terminal", never as if it were the Claude app (64).
  const where = [props.apps.length > 0 ? `open in ${props.apps.map((app) => app.name).join(", ")}` : tool, ...perProfile].join(" · ");
  return (
    <li className="flex flex-col" data-testid="profile-row" data-profile={account.id}>
      <div className="group flex flex-wrap items-center gap-3 px-5 py-3" {...menu.triggerProps}>
        {/* D-62 (AJ-E3206 req. 2): one mark per App, as everywhere: its
            app's icon with its profile's dot, never initials. */}
        <span className="inline-flex shrink-0 items-center gap-1" data-testid="profile-app-mark">
          <CliAppMark tool={account.provider} folderId={account.id} folder={account.label} size={24} ring="var(--color-surface)" />
          {/* D-100: a CLI App, beside its icon. */}
          <AppKindBadge kind="cli" />
        </span>
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="flex min-w-0 items-center gap-2">
            {service && <ToolMark tool={account.provider} size={13} className="text-muted" />}
            <button type="button" onClick={props.onInspect} title={`Open ${name}'s page`} className="truncate rounded text-body font-semibold outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="account-label">
              {name}
            </button>
            {account.is_default && (
              <span title="New sessions start with this app, unless a project picks its own" data-testid="account-default">
                <Chip tone="neutral">Default</Chip>
              </span>
            )}
          </span>
          <span className="flex min-w-0 items-center gap-2 text-meta text-muted">
            {props.apps.length > 0 && (
              <span aria-hidden="true" className="flex shrink-0 items-center gap-1">
                {props.apps.slice(0, 4).map((app) => (
                  <AppIcon key={app.name} {...app} size={16} />
                ))}
              </span>
            )}
            <span className="truncate" data-testid="profile-apps">{where}</span>
          </span>
          {/* 0.4.1: couldn't tell this time; it stays in this Account meanwhile. */}
          {own?.unchecked && (
            <span className="truncate text-meta text-warning" title="It stays in this account until a check says otherwise. AI Juggler checks again on its own." data-testid="profile-sign-in-unchecked">
              {signInUncheckedWords(own.unchecked)}
            </span>
          )}
          {placementWords(account) && (
            <span className="flex min-w-0 items-center gap-1.5 text-meta">
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
            <span className="text-meta text-muted" data-testid="account-activity">
              {props.openNow > 0 ? `${props.openNow} open now` : "Nothing open"}
            </span>
          ) : service ? (
            <span className="text-label text-warning" data-testid="profile-key-missing">
              Its key isn't here
            </span>
          ) : (
            <>
              <span className="text-meta text-warning" data-testid="profile-signed-out">
                Not signed in
              </span>
              <Button size="sm" variant="secondary" onClick={props.onLogin}>
                Sign in
              </Button>
            </>
          )}
          <Button size="sm" variant="secondary" onClick={props.onInspect} disabled={!props.onInspect} data-testid="profile-open">
            Open
            <span className="sr-only"> {name}</span>
          </Button>
          {menu.button({ label: `App actions for ${name}`, shown: true, testId: "account-menu" })}
        </div>
      </div>
      {menu.element}
      {settingsOpen && (
        <LaunchSettingsList
          account={props.account}
          recipes={props.recipes}
          liveByRecipe={props.liveByRecipe}
          envFor={envFor}
          setEnvFor={setEnvFor}
          onLaunch={props.onLaunch}
          onDeleteRecipe={props.onDeleteRecipe}
          onNewRecipe={props.onNewRecipe}
          onClose={() => setSettingsOpen(false)}
        />
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


