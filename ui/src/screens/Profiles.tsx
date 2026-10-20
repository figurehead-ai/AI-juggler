import { useCallback, useEffect, useState } from "react";
import { Button, Card, Chip, type ChipTone } from "../components/primitives";
import { renewalLabel, renewalRowVisible } from "../lib/renewal";
import { Field, Input, Modal } from "../components/form";
import { IconProfiles, IconTerminal } from "../components/icons";
import { LoginQueueCard } from "../components/LoginQueueCard";
import { BetaMatrix } from "../components/BetaMatrix";
import { ExpertSetup } from "../components/ExpertSetup";
import { EnvOverlayEditor } from "../components/EnvOverlayEditor";
import { AccountRunway, runwayLines } from "../components/AccountRunway";
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
  errorText,
  signedIn,
  quotaDisplayAll,
  type InstanceInfo,
  type QuotaDisplayData,
} from "../lib/bridge";

const LIVE = ["running", "starting", "waiting_user", "stuck"];

/** What runs on an account, in a few words ("2 running · 1 needs you"). */
export function activityWords(sessions: InstanceInfo[]): string {
  const live = sessions.filter((s) => LIVE.includes(s.status));
  const needs = sessions.filter((s) => s.status === "waiting_user" || s.status === "stuck" || s.status === "quota_blocked" || s.status === "failed");
  const parts = [live.length > 0 ? `${live.length} running` : null, needs.length > 0 ? `${needs.length} need${needs.length === 1 ? "s" : ""} you` : null].filter(Boolean);
  return parts.length > 0 ? parts.join(" · ") : "Nothing running";
}

/** The account with the most runway left for new work, if any is measured. */
export function bestForNewWork(
  accounts: AccountInfo[],
  displays: QuotaDisplayData[],
  sessions: InstanceInfo[],
): { account: AccountInfo; left: number } | null {
  let best: { account: AccountInfo; left: number } | null = null;
  for (const account of accounts) {
    if (!signedIn(account.login_state)) continue;
    const lines = runwayLines(displays.find((d) => d.account === account.id), sessions.filter((s) => s.account_id === account.id));
    const measured = lines.filter((line) => line.left !== null).map((line) => line.left!);
    if (measured.length === 0) continue;
    const left = Math.min(...measured);
    if (!best || left > best.left) best = { account, left };
  }
  return best;
}

/**
 * Accounts screen (M1-3; component name kept from the M1 "Profiles"
 * era — the UI noun is Accounts, D-10): accounts with isolated state,
 * their runway (D-25) and Launch settings. The pack-secrets policy
 * lives in Settings › Context packs & secrets.
 * Every action maps 1:1 to a typed backend command; in plain-browser
 * preview mode the screen renders mock data and launches are no-ops so
 * the design can be reviewed without a backend.
 */

export function ProfilesScreen({
  openTerminal,
  onInspectAccount,
  sessions = [],
  onAddAccounts,
}: {
  /** D-25: Add account opens the find → bring in (or a fresh account, or
   * a folder) → sign in flow; with more than one tool installed it asks
   * which tool first (AJ-E1023). */
  onAddAccounts: () => void;
  openTerminal: (id: string, title: string) => void;
  /** D-25: what runs where (activity line, runway fallback). */
  sessions?: InstanceInfo[];
  onInspectAccount?: (id: string) => void;
}) {
  const [accounts, setAccounts] = useState<AccountInfo[] | null>(null);
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[] | null>(null);
  const [newProfileFor, setNewProfileFor] = useState<AccountInfo | null>(null);
  const [loginFor, setLoginFor] = useState<AccountInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [displays, setDisplays] = useState<QuotaDisplayData[]>([]);
  useEffect(() => {
    let alive = true;
    const load = () => void quotaDisplayAll().then((rows) => alive && setDisplays(rows), () => null);
    load();
    const timer = window.setInterval(load, 30_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);

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
          <p className="text-xs text-muted" data-testid="accounts-summary">
            {(() => {
              const best = bestForNewWork(accounts, displays, sessions);
              return best
                ? `Best for new work: ${best.account.label}, ${best.left}% left.`
                : "Each account is its own sign-in, kept apart from the others.";
            })()}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2 whitespace-nowrap">
        <Button variant="primary" size="sm" onClick={onAddAccounts} data-testid="add-account">
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
        <EmptyAccounts onAdd={onAddAccounts} />
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
              display={displays.find((d) => d.account === account.id)}
              sessions={sessions.filter((session) => session.account_id === account.id)}
            />
          ))}
        </div>
      )}

      {/* D-25: the pack-secrets policy lives in Settings; the matrix of
          proven switches is one click away, not on every visit. */}
      <p className="text-xs text-muted">
        How keys and tokens in context packs are handled is in{" "}
        <button
          type="button"
          data-testid="accounts-pack-settings"
          onClick={() => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "packs" } }))}
          className="text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          Settings › Context packs &amp; secrets
        </button>
        .
      </p>
      <details className="group" data-testid="accounts-beta-matrix">
        <summary className="cursor-pointer text-xs text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
          Which switches between accounts are proven (Beta)
        </summary>
        <div className="mt-3">
          <BetaMatrix />
        </div>
      </details>

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
        No accounts yet. Add your first one: AI Juggler keeps each account's sign-in in its
        own folder, apart from anything you already use.
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
  display,
  sessions = [],
}: {
  account: AccountInfo;
  /** D-25: this account's two windows (runway), if measured. */
  display?: QuotaDisplayData;
  /** D-25: its sessions (what runs on it; a runway fallback). */
  sessions?: InstanceInfo[];
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
          {loggedIn ? (
            <span className="text-[11px] text-muted" data-testid="account-activity">
              {activityWords(sessions)}
            </span>
          ) : (
            <>
              <Chip tone={loginTone} dot breathing>
                Needs sign-in
              </Chip>
              <Button size="sm" variant="secondary" onClick={onLogin}>
                Sign in
              </Button>
            </>
          )}
        </div>
      </div>

      {/* D-25: runway lives here and on Home. */}
      <div className="px-5 py-3.5">
        <AccountRunway display={display} sessions={sessions} />
      </div>

      <div className="flex items-center gap-2 border-t border-hairline px-5 py-3">
        <Button
          size="sm"
          variant="primary"
          disabled={profiles.length === 0 || !loggedIn}
          title={!loggedIn ? "Sign in first" : profiles.length === 0 ? "Add launch settings first (Edit launch settings)" : "Starts a session on this account"}
          onClick={() => onLaunch(profiles[0])}
        >
          <IconTerminal width={13} height={13} />
          Start a session
          <span className="sr-only">on {account.label}</span>
        </Button>
        <Button size="sm" variant="secondary" onClick={onToggleSettings} aria-expanded={settingsOpen}>
          Edit launch settings {settingsOpen ? "▾" : "▸"}
        </Button>
      </div>

      <div className={settingsOpen ? "divide-y divide-hairline" : "hidden"}>
        <div className="flex items-center justify-between px-5 py-2">
          <span className="text-[11px] uppercase tracking-wider text-muted">Launch settings · used when you start on this account</span>
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
      title={onClick ? `Inspect ${providerDisplayName(provider)}` : providerDisplayName(provider)}
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
              placeholder="Where sessions start — or choose…"
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
