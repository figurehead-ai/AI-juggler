import { useCallback, useEffect, useState } from "react";
import { Button, Card, Chip, type ChipTone } from "../components/primitives";
import { renewalLabel, renewalRowVisible } from "../lib/renewal";
import { Field, Input, Modal } from "../components/form";
import { IconProfiles, IconTerminal } from "../components/icons";
import { OnboardingWizard } from "./OnboardingWizard";
import { AddProviderSheet } from "./AddProviderSheet";
import { LoginQueueCard } from "../components/LoginQueueCard";
import { BetaMatrix } from "../components/BetaMatrix";
import { ExpertSetup } from "../components/ExpertSetup";
import { EnvOverlayEditor } from "../components/EnvOverlayEditor";
import { PACK_POLICY_OPTIONS, packPolicyRestatement, type PackPolicyWire } from "../components/PackPolicyChooser";
import { providerDisplayName } from "../lib/providers";
import {
  accountsList,
  createProfile,
  deleteProfile,
  launchSession,
  launchRecipesList,
  pickDirectory,
  type AccountInfo,
  type LaunchRecipeInfo,
  expertSetupInfo,
  redactionPolicyGet,
  redactionPolicySetGlobal,
  redactionPolicySetOverride,
  type RedactionInfoData, errorText,
  signedIn,
} from "../lib/bridge";

/**
 * Accounts screen (M1-3; component name kept from the M1 "Profiles"
 * era — the UI noun is Accounts, D-10): accounts with isolated state,
 * their Launch settings, and the Provider-level pack-secrets policy.
 * Every action maps 1:1 to a typed backend command; in plain-browser
 * preview mode the screen renders mock data and launches are no-ops so
 * the design can be reviewed without a backend.
 */

export function ProfilesScreen({
  openTerminal,
  onInspectAccount,
  onLaunchFirstSession,
}: {
  openTerminal: (id: string, title: string) => void;
  onInspectAccount?: (id: string) => void;
  /** AJ-E305: the wizard's [Launch first session →] opens the launch
   * sheet with the Start-here project preselected. */
  onLaunchFirstSession?: (startHere: string | null) => void;
}) {
  const [accounts, setAccounts] = useState<AccountInfo[] | null>(null);
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[] | null>(null);
  const [addAccount, setAddAccount] = useState(false);
  // AJ-E306 flow (c): the Provider picker sheet.
  const [addProvider, setAddProvider] = useState(false);
  const [newProfileFor, setNewProfileFor] = useState<AccountInfo | null>(null);
  const [loginFor, setLoginFor] = useState<AccountInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [accountRows, profileRows] = await Promise.all([
        accountsList(),
        launchRecipesList(),
      ]);
      setAccounts(accountRows);
      setProfiles(profileRows);
    } catch (cause) {
      setError(errorText(cause));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const launch = async (recipe: LaunchRecipeInfo) => {
    setError(null);
    try {
      const result = await launchSession(recipe.id, null);
      if (!result) {
        setError("Preview mode: no backend connected — nothing launched.");
        return;
      }
      // Interactive sessions stream straight into the terminal dock.
      openTerminal(
        result.session_id,
        `${recipe.account_label} · ${recipe.name}`,
      );
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  if (accounts === null || profiles === null) {
    return <div className="py-24 text-center text-sm text-muted">Loading…</div>;
  }

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-5">
      <header className="flex items-end justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold tracking-tight">Accounts</h2>
          <p className="text-xs text-muted">
            One account per Provider sign-in — each fully isolated. Start any of
            them in parallel with one click.
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2 whitespace-nowrap">
        <Button variant="secondary" size="sm" onClick={() => setAddProvider(true)} data-testid="add-provider">
          <IconProfiles width={14} height={14} />
          Add Provider
        </Button>
        <Button variant="primary" size="sm" onClick={() => setAddAccount(true)} data-testid="add-account">
          <IconProfiles width={14} height={14} />
          Add account
        </Button>
        </div>
      </header>

      {error && (
        <Card className="border-danger/30 bg-danger-soft p-3 text-xs text-danger">
          {error}
        </Card>
      )}

      {accounts.length === 0 ? (
        <EmptyAccounts onAdd={() => setAddAccount(true)} />
      ) : (
        <div className="flex flex-col gap-4">
          {accounts.map((account) => (
            <AccountCard
              key={account.id}
              account={account}
              profiles={profiles.filter((recipe) => recipe.account_id === account.id)}
              onLaunch={launch}
              onDeleteProfile={async (id) => {
                await deleteProfile(id);
                void refresh();
              }}
              onNewProfile={() => setNewProfileFor(account)}
              onLogin={() => setLoginFor(account)}
              onInspect={() => onInspectAccount?.(account.id)}
              onLoginRechecked={refresh}
            />
          ))}
        </div>
      )}

      {/* AJ-E309 Req.2 (D-14): the remembered pack-secrets choice —
          global + per-Provider override. "Decide later" in the first
          run points here. */}
      <PackSecretsCard providers={[...new Set(accounts.map((account) => account.provider))]} />

      {/* AJ-E210: the Beta validation matrix (Settings → Beta,
          disclosure only) — the same table the switch badges derive
          from, so "which pairs are proven" is never a guess. */}
      <BetaMatrix />

      {addAccount && (
        <OnboardingWizard
          provider="claude-code"
          locked
          onClose={() => setAddAccount(false)}
          onAccountCreated={() => void refresh()}
          openLoginTerminal={openTerminal}
          onLaunchFirstSession={(startHere) => {
            setAddAccount(false);
            onLaunchFirstSession?.(startHere);
          }}
          onAddAnother={() => {
            // Reopen fresh: unmount + remount resets the wizard.
            setAddAccount(false);
            window.setTimeout(() => setAddAccount(true), 0);
          }}
        />
      )}

      {addProvider && (
        <AddProviderSheet
          onClose={() => setAddProvider(false)}
          onAccountCreated={() => void refresh()}
          openLoginTerminal={openTerminal}
        />
      )}

      {newProfileFor && (
        <NewProfileModal
          account={newProfileFor}
          onClose={() => setNewProfileFor(null)}
          onCreated={() => {
            setNewProfileFor(null);
            void refresh();
          }}
        />
      )}

      {loginFor && (
        <LoginModal
          account={loginFor}
          openTerminal={openTerminal}
          onClose={() => {
            setLoginFor(null);
            void refresh();
          }}
        />
      )}
    </div>
  );
}

function EmptyAccounts({ onAdd }: { onAdd: () => void }) {
  // AJ-E301 Req.6: re-entry after dismissal — with zero accounts the
  // wizard is re-offered with its own copy, never buried behind a
  // generic "add".
  const dismissed =
    typeof window !== "undefined" &&
    localStorage.getItem("aij.onboarding.dismissed") === "1";
  return (
    <Card className="flex flex-col items-center justify-center gap-3 py-14">
      <span className="flex size-11 items-center justify-center rounded-xl border border-accent/20 bg-accent-soft text-accent-hi">
        <IconProfiles width={22} height={22} />
      </span>
      <p className="max-w-sm text-center text-sm text-muted">
        No accounts yet. Add your first Provider sign-in — AI Juggler keeps its
        state fully isolated from any existing install.
      </p>
      <Button variant="primary" size="sm" onClick={onAdd} data-testid="finish-setup">
        {dismissed ? "Finish setting up AI Juggler" : "Add account"}
      </Button>
    </Card>
  );
}

function AccountCard({
  account,
  now = Date.now(),
  profiles,
  onLaunch,
  onDeleteProfile,
  onNewProfile,
  onLogin,
  onLoginRechecked,
  onInspect,
}: {
  account: AccountInfo;
  /** Local clock for the expiry tiers (AJ-E411). */
  now?: number;
  profiles: LaunchRecipeInfo[];
  onLaunch: (recipe: LaunchRecipeInfo) => void;
  onDeleteProfile: (id: string) => Promise<void>;
  onNewProfile: () => void;
  onLogin: () => void;
  onLoginRechecked: () => Promise<void>;
  onInspect?: () => void;
}) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [envFor, setEnvFor] = useState<string | null>(null);
  const onToggleSettings = () => setSettingsOpen((v) => !v);
  const loggedIn = signedIn(account.login_state);
  const loginTone: ChipTone = loggedIn ? "success" : "warning";
  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-2.5 border-b border-hairline px-5 py-3.5">
        <HarnessGlyph provider={account.provider} onClick={onInspect} />
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-semibold">{account.label}</span>
            {account.plan && <Chip tone="accent">{account.plan}</Chip>}
          </div>
          <span className="text-[11px] text-muted">{providerDisplayName(account.provider)}</span>
        </div>
        <div className="ml-auto flex items-center gap-2">
          {renewalRowVisible(renewalLabel(account.renewal_date, now)) && (
            <Chip tone={renewalLabel(account.renewal_date, now)!.tier === "danger" ? "danger" : "warning"}>
              {renewalLabel(account.renewal_date, now)!.tier === "danger" ? "⛔" : "⚠"}{" "}
              {renewalLabel(account.renewal_date, now)!.text}
            </Chip>
          )}
          <Chip tone={loginTone} dot breathing={!loggedIn}>
            {loggedIn ? "Logged in" : "Needs login"}
          </Chip>
          {!loggedIn && (
            <Button size="sm" variant="secondary" onClick={onLogin}>
              Log in
            </Button>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 border-t border-hairline px-5 py-3">
        <Button
          size="sm"
          variant="primary"
          disabled={profiles.length === 0 || !loggedIn}
          title={loggedIn ? "Starts an Instance on this Account" : "Sign in first (Ready home)"}
          onClick={() => onLaunch(profiles[0])}
        >
          <IconTerminal width={13} height={13} />
          Start on {account.label}
        </Button>
        <Button size="sm" variant="secondary" onClick={onToggleSettings}>
          Edit launch settings {settingsOpen ? "▾" : "▸"}
        </Button>
        <span className="ml-auto text-[11px] text-muted">
          Applies when you start on this account.
        </span>
      </div>

      <div className={settingsOpen ? "divide-y divide-hairline" : "hidden"}>
        <div className="flex items-center justify-between px-5 py-2">
          <span className="text-[11px] uppercase tracking-wider text-muted">Launch settings</span>
          <button
            type="button"
            className="rounded-md px-2 py-1 text-[11px] text-muted hover:bg-surface hover:text-ink"
            onClick={onNewProfile}
          >
            ＋ New settings set
          </button>
        </div>
        {profiles.length === 0 ? (
          <div className="px-5 py-4 text-xs text-muted">
            No launch settings yet — the default applies when you start.
          </div>
        ) : (
          profiles.map((recipe) => (
            <div key={recipe.id} className="group flex flex-wrap items-center gap-3 px-5 py-2.5">
              <span className="text-sm">{recipe.name === "Main" ? "Default" : recipe.name}</span>
              {recipe.model ? (
                <Chip tone="neutral">{recipe.model}</Chip>
              ) : (
                <Chip tone="neutral">default model</Chip>
              )}
              {recipe.default_cwd && (
                <span className="truncate font-mono text-[11px] text-muted">
                  {recipe.default_cwd}
                </span>
              )}
              <button
                type="button"
                aria-expanded={envFor === recipe.id}
                onClick={() => setEnvFor((current) => (current === recipe.id ? null : recipe.id))}
                className="rounded-md px-2 py-1 text-[11px] text-muted hover:bg-surface hover:text-ink"
                data-testid="recipe-env-toggle"
              >
                Environment {envFor === recipe.id ? "▾" : "▸"}
              </button>
              <div className="ml-auto flex items-center gap-1 opacity-60 transition-opacity group-hover:opacity-100">
                <Button
                  size="sm"
                  variant="ghost"
                  title="Delete launch settings"
                  onClick={() => void onDeleteProfile(recipe.id)}
                >
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round">
                    <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
                  </svg>
                </Button>
                <Button size="sm" variant="primary" onClick={() => onLaunch(recipe)}>
                  <IconTerminal width={13} height={13} />
                  Launch
                </Button>
              </div>
              {envFor === recipe.id && (
                <div className="basis-full pt-1">
                  <RecipeEnvEditor
                    provider={account.provider}
                    accountId={account.id}
                    recipeId={recipe.id}
                  />
                </div>
              )}
            </div>
          ))
        )}
        <div className="px-5 py-3">
          <ExpertSetup
            provider={account.provider}
            accounts={[{ id: account.id, label: account.label }]}
            surface="accounts"
            showMetricsExport
          />
        </div>
      </div>
      {/* keep recheck reachable for logged-in cards via silent refresh */}
      <span className="hidden">{void onLoginRechecked}</span>
    </Card>
  );
}

function HarnessGlyph({ provider, onClick }: { provider: string; onClick?: () => void }) {
  // Claude Code mark: an abstract spark; per-harness glyphs grow with M2.
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={!onClick}
      title={onClick ? `Inspect ${provider}` : provider}
      className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-accent/20 bg-accent-soft text-accent-hi outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 enabled:hover:border-accent/40 disabled:cursor-default"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 2.6c.5 3.9 2.4 6.6 6.9 8-4.5 1.4-6.4 4.1-6.9 8-.5-3.9-2.4-6.6-6.9-8 4.5-1.4 6.4-4.1 6.9-8Z" />
      </svg>
    </button>
  );
}

function NewProfileModal({
  account,
  onClose,
  onCreated,
}: {
  account: AccountInfo;
  onClose: () => void;
  onCreated: () => void;
}) {
  const [name, setName] = useState("");
  const [model, setModel] = useState("");
  const [cwd, setCwd] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    if (!name.trim()) {
      setError("Name these Launch settings, e.g. “Deep work”.");
      return;
    }
    setBusy(true);
    try {
      await createProfile(account.id, name.trim(), model.trim() || null, cwd.trim() || null);
      onCreated();
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };

  return (
    <Modal
      title={`Launch settings · ${account.label}`}
      subtitle="Launch settings applied when this Account starts (model, environment)."
      onClose={onClose}
    >
      <div className="flex flex-col gap-4">
        <Field label="Name">
          <Input autoFocus value={name} placeholder="Deep work" onChange={(event) => setName(event.target.value)} />
        </Field>
        <Field label="Model (optional)" hint="Written to the account's managed settings">
          <Input value={model} placeholder="opus" onChange={(event) => setModel(event.target.value)} />
        </Field>
        <Field label="Default folder (optional)">
          <div className="flex gap-2">
            <Input
              value={cwd}
              placeholder="Where Instances start — or choose…"
              onChange={(event) => setCwd(event.target.value)}
              className="min-w-0 flex-1"
            />
            <Button
              size="sm"
              variant="secondary"
              onClick={async () => {
                const picked = await pickDirectory();
                if (picked !== null) setCwd(picked);
              }}
            >
              Choose…
            </Button>
          </div>
        </Field>
        {error && <p className="text-xs text-danger">{error}</p>}
        <div className="ml-auto flex gap-2">
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" disabled={busy} onClick={() => void submit()}>
            Create Launch settings
          </Button>
        </div>
      </div>
    </Modal>
  );
}

function LoginModal({
  account,
  openTerminal,
  onClose,
}: {
  account: AccountInfo;
  openTerminal: (id: string, title: string) => void;
  onClose: () => void;
}) {
  return (
    <Modal
      title={`Log in · ${account.label}`}
      subtitle={`${providerDisplayName(account.provider)}'s own sign-in, opened for you — AI Juggler never sees your credentials.`}
      onClose={onClose}
    >
      {/* AJ-E308 Req.3: the ONE shared queue card — no alerts, no
          unhandled rejections; polls and auto-detects. */}
      <LoginQueueCard
        accountId={account.id}
        label={account.label}
        providerName={providerDisplayName(account.provider)}
        initialState={account.login_state}
        onSpawned={(terminalId) => {
          openTerminal(terminalId, `Sign in · ${account.label}`);
          onClose();
        }}
        onDone={onClose}
      />
    </Modal>
  );
}

/**
 * AJ-E309 Req.2 / AJ-E208 (D-14): Accounts → Pack secrets. The GLOBAL
 * choice (the same words as the first-run chooser) and one override
 * row per Provider in use ("Use global" clears it). Every write is
 * the user's click; an unset global says so plainly — the first
 * switch asks, nothing is assumed.
 */
function PackSecretsCard({ providers }: { providers: string[] }) {
  const [info, setInfo] = useState<Record<string, RedactionInfoData>>({});
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const providerKey = providers.join("|");

  const reload = useCallback(async () => {
    const targets = providers.length > 0 ? providers : ["claude-code"];
    const rows = await Promise.all(
      targets.map(async (provider) => [provider, await redactionPolicyGet(provider)] as const),
    );
    setInfo(Object.fromEntries(rows));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [providerKey]);

  useEffect(() => {
    void reload().catch((cause) => setError(errorText(cause)));
  }, [reload]);

  const first = Object.values(info)[0];
  const global = (first?.global ?? null) as PackPolicyWire | null;
  const storedOptions = PACK_POLICY_OPTIONS.filter((option) => option.value !== "decide_later");

  const run = async (action: () => Promise<void>, done: string) => {
    setError(null);
    setStatus(null);
    try {
      await action();
      await reload();
      setStatus(done);
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="pack-secrets">
      <div>
        <h3 className="text-sm font-semibold">Pack secrets</h3>
        <p className="mt-0.5 text-xs leading-relaxed text-muted">
          Context packs carry your work to another account when you switch. Choose how keys and
          tokens found in them are handled — remembered for every Provider unless one overrides
          it.
        </p>
      </div>
      <fieldset className="flex flex-col gap-1.5" data-testid="pack-secrets-global">
        <legend className="mb-1 text-[11px] font-medium uppercase tracking-wider text-muted">
          Your choice (all Providers)
        </legend>
        {global === null && (
          <p className="text-[11px] text-muted" data-testid="pack-secrets-unset">
            Not chosen yet — your first switch asks before any pack is built.
          </p>
        )}
        {storedOptions.map((option) => (
          <label
            key={option.value}
            className={
              "flex items-start gap-2.5 rounded-lg border px-3 py-2 text-xs transition-colors " +
              (global === option.value ? "border-accent/40 bg-accent-soft" : "border-hairline hover:border-accent/30")
            }
          >
            <input
              type="radio"
              name="pack-secrets-global"
              value={option.value}
              checked={global === option.value}
              onChange={() =>
                void run(
                  () => redactionPolicySetGlobal(option.value),
                  packPolicyRestatement(option.value as PackPolicyWire, "global", ""),
                )
              }
              className="mt-0.5 size-3.5 shrink-0 accent-[var(--color-accent)]"
            />
            <span className="flex flex-col gap-0.5">
              <span className="font-medium text-ink">{option.title}</span>
              <span className="text-[11px] leading-relaxed text-muted">{option.consequence}</span>
            </span>
          </label>
        ))}
      </fieldset>
      {providers.length > 0 && (
        <div className="flex flex-col gap-1.5" data-testid="pack-secrets-overrides">
          <p className="text-[11px] font-medium uppercase tracking-wider text-muted">
            Per-Provider override
          </p>
          {providers.map((provider) => {
            const row = info[provider];
            const name = providerDisplayName(provider);
            return (
              <label key={provider} className="flex flex-wrap items-center gap-2 text-xs" data-testid="provider-redaction">
                <span className="min-w-28 font-medium">{name}</span>
                <select
                  value={row?.override ?? ""}
                  onChange={(event) =>
                    void run(
                      () => redactionPolicySetOverride(provider, event.target.value || null),
                      event.target.value ? `${name} now overrides the global choice.` : `${name} follows your global choice.`,
                    )
                  }
                  aria-label={`Pack secrets for ${name}`}
                  className="rounded-lg border border-hairline bg-surface-2 px-2 py-1 text-xs text-ink"
                >
                  <option value="">Use my choice{global ? "" : " (not chosen yet)"}</option>
                  {storedOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.title}
                    </option>
                  ))}
                </select>
                {row?.override && <Chip tone="accent">Override</Chip>}
              </label>
            );
          })}
        </div>
      )}
      {status && (
        <p className="text-[11px] text-muted" role="status">
          {status}
        </p>
      )}
      {error && (
        <p className="text-[11px] text-danger" role="alert">
          {error}
        </p>
      )}
    </Card>
  );
}

/** The env-override editor for one Launch settings, with the
 * Provider's protected (isolation) keys fetched from the backend —
 * never a hard-coded Provider list in the UI. */
function RecipeEnvEditor({
  provider,
  accountId,
  recipeId,
}: {
  provider: string;
  accountId: string;
  recipeId: string;
}) {
  const [protectedKeys, setProtectedKeys] = useState<string[] | null>(null);
  useEffect(() => {
    let alive = true;
    void expertSetupInfo(provider, accountId)
      .then((info) => {
        if (alive) setProtectedKeys(info.protected_env_keys);
      })
      .catch(() => {
        // The backend still refuses protected keys on Save.
        if (alive) setProtectedKeys([]);
      });
    return () => {
      alive = false;
    };
  }, [provider, accountId]);
  if (protectedKeys === null) {
    return <p className="text-[11px] text-muted">Loading…</p>;
  }
  return <EnvOverlayEditor recipeId={recipeId} protectedKeys={protectedKeys} />;
}
