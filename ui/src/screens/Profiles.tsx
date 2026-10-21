import { useCallback, useEffect, useState } from "react";
import { Button, Card, Chip, type ChipTone } from "../components/primitives";
import { renewalLabel, renewalRowVisible } from "../lib/renewal";
import { Field, Input, Modal } from "../components/form";
import { IconProfiles, IconTerminal } from "../components/icons";
import { LoginQueueCard } from "../components/LoginQueueCard";
import { MenuItem, RowMenu, menuPoint } from "../components/RowMenu";
import { BetaMatrix } from "../components/BetaMatrix";
import { ExpertSetup } from "../components/ExpertSetup";
import { EnvOverlayEditor } from "../components/EnvOverlayEditor";
import { AccountRunway, runwayLines } from "../components/AccountRunway";
import { providerMark } from "../components/indicator";
import { inPlaceProviders, ownSignIn, providerDisplayName, sameAsWords, supportsInPlace, type InPlaceSupport, limitProviders, reportsLimits, noLimitsWords } from "../lib/providers";
import {
  accountRemove,
  accountRename,
  accountSetDefault,
  accountUseOriginal,
  accountsList,
  createProfile,
  deleteProfile,
  launchSession,
  launchRecipesList,
  pickDirectory,
  providersCapabilities,
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
  onAccountsChanged,
}: {
  /** AJ-E1501: a rename, removal or new default — the rest of the app
   * (rail, Usage, New session) reads the accounts again, no restart. */
  onAccountsChanged?: () => void;
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
  // AJ-E1501: what the last change did, in words ("Removed …").
  const [notice, setNotice] = useState<string | null>(null);
  const [renameFor, setRenameFor] = useState<AccountInfo | null>(null);
  const [removeFor, setRemoveFor] = useState<AccountInfo | null>(null);
  const [displays, setDisplays] = useState<QuotaDisplayData[]>([]);
  // ADR-0018: which tools' copies can switch to their original — the
  // backend's static capabilities (no install probe). If the call fails
  // the offer stays; the backend refuses what it can't adopt.
  const [inPlace, setInPlace] = useState<InPlaceSupport | null>(null);
  // 0.0.11 walk (Q3): the tools that don't report their own limits.
  const [limits, setLimits] = useState<InPlaceSupport>("unknown");
  useEffect(() => {
    let alive = true;
    void providersCapabilities().then(
      (rows) => {
        if (!alive) return;
        setInPlace(inPlaceProviders(rows));
        setLimits(limitProviders(rows));
      },
      () => alive && setInPlace("unknown"),
    );
    return () => {
      alive = false;
    };
  }, []);
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

  const changed = async (words: string | null) => {
    setNotice(words);
    await refresh();
    onAccountsChanged?.();
  };

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
      <p role="status" className="text-xs text-muted empty:hidden" data-testid="accounts-notice">
        {notice}
      </p>

      {accounts.length === 0 ? (
        <EmptyAccounts onAdd={onAddAccounts} />
      ) : (
        <div className="flex flex-col gap-4">
          {accounts.map((account) => (
            <AccountCard
              key={account.id}
              account={account}
              canUseInPlace={inPlace !== null && supportsInPlace(inPlace, account.provider)}
              reportsLimits={reportsLimits(limits, account.provider)}
              profiles={profiles.filter((recipe) => recipe.account_id === account.id)}
              onLaunch={launch}
              onDeleteProfile={async (id) => {
                await deleteProfile(id);
                void refresh();
              }}
              onNewProfile={() => setNewProfileFor(account)}
              onLogin={() => setLoginFor(account)}
              onInspect={() => onInspectAccount?.(account.id)}
              onRename={() => setRenameFor(account)}
              onRemove={() => setRemoveFor(account)}
              onToggleDefault={async () => {
                setError(null);
                const wasDefault = account.is_default === true;
                try {
                  await accountSetDefault(wasDefault ? null : account.id);
                  await changed(
                    wasDefault
                      ? `${account.label} is no longer the default. New sessions start with the account you used last.`
                      : `New sessions start with ${account.label}, unless a project picks its own.`,
                  );
                } catch (cause) {
                  setError(errorText(cause));
                }
              }}
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

      {renameFor && (
        <RenameAccountModal
          account={renameFor}
          onClose={() => setRenameFor(null)}
          onDone={async (renamed, before) => {
            setRenameFor(null);
            await changed(renamed.label !== before ? `Renamed to ${renamed.label}.` : null);
          }}
        />
      )}

      {removeFor && (
        <RemoveAccountModal
          account={removeFor}
          sessions={sessions.filter((session) => session.account_id === removeFor.id)}
          onClose={() => setRemoveFor(null)}
          onDone={async (words) => {
            setRemoveFor(null);
            await changed(words);
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
  canUseInPlace,
  now = Date.now(),
  profiles,
  onLaunch,
  onDeleteProfile,
  onNewProfile,
  onLogin,
  onLoginRechecked,
  onInspect,
  onRename,
  onRemove,
  onToggleDefault,
  display,
  sessions = [],
  reportsLimits = true,
}: {
  account: AccountInfo;
  /** AJ-E1501: the card's menu. */
  onRename: () => void;
  onRemove: () => void;
  onToggleDefault: () => void;
  /** 0.0.11 walk (Q3): its tool reports usage limits (else no limit rows). */
  reportsLimits?: boolean;
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
  /** ADR-0018: this tool's setups can be used where they are. */
  canUseInPlace: boolean;
}) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [useOriginal, setUseOriginal] = useState(false);
  const [envFor, setEnvFor] = useState<string | null>(null);
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null);
  const onToggleSettings = () => setSettingsOpen((v) => !v);
  const loggedIn = signedIn(account.login_state);
  const loginTone: ChipTone = loggedIn ? "success" : "warning";
  return (
    <Card className="overflow-hidden">
      <div
        className="flex flex-wrap items-center gap-2.5 border-b border-hairline px-5 py-3.5"
        onContextMenu={(event) => {
          event.preventDefault();
          setMenuAt({ x: event.clientX, y: event.clientY });
        }}
      >
        <HarnessGlyph provider={account.provider} onClick={onInspect} />
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-semibold" data-testid="account-label">{account.label}</span>
            {account.plan && <Chip tone="accent">{account.plan}</Chip>}
            {account.is_default && (
              <span title="New sessions start with this account, unless a project picks its own" data-testid="account-default">
                <Chip tone="neutral">Default</Chip>
              </span>
            )}
          </div>
          <div className="flex min-w-0 items-center gap-1.5 text-[11px]">
            <span className="truncate text-muted" title={account.import_source_path ?? undefined} data-testid="account-placement">
              {providerDisplayName(account.provider)}
              {placementWords(account) && ` · ${placementWords(account)}`}
            </span>
            {/* ADR-0018: a copy can switch to the folder it came from. */}
            {account.import_source_path && !account.in_place && canUseInPlace && (
              <button
                type="button"
                className="shrink-0 text-accent-hi hover:underline"
                onClick={() => setUseOriginal(true)}
                data-testid="account-use-original"
              >
                Use the original instead
              </button>
            )}
          </div>
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
          {/* AJ-E1501: the row menu (the rail's pattern) — right-click
              the header, or this button. 24 px: the WCAG 2.2 target. */}
          <button
            type="button"
            aria-label={`Account actions for ${account.label}`}
            aria-haspopup="menu"
            aria-expanded={menuAt !== null}
            data-testid="account-menu"
            onClick={(event) => setMenuAt(menuPoint(event))}
            className="flex size-6 items-center justify-center rounded text-[12px] text-muted outline-none transition-colors hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            ⋯
          </button>
        </div>
      </div>
      {menuAt && (
        <RowMenu heading={account.label} ariaLabel={`Actions for ${account.label}`} at={menuAt} onClose={() => setMenuAt(null)}>
          <MenuItem testId="account-rename" onClick={() => { setMenuAt(null); onRename(); }}>
            Rename…
          </MenuItem>
          <MenuItem testId="account-toggle-default" onClick={() => { setMenuAt(null); onToggleDefault(); }}>
            {account.is_default ? "Stop being the default" : "Make the default"}
          </MenuItem>
          <MenuItem testId="account-remove" danger onClick={() => { setMenuAt(null); onRemove(); }}>
            Remove…
          </MenuItem>
        </RowMenu>
      )}

      {/* D-25: runway lives here and on Home. A tool that doesn't
          report its limits says so once, instead of empty windows. */}
      <div className="px-5 py-3.5">
        {reportsLimits ? (
          <AccountRunway display={display} sessions={sessions} />
        ) : (
          <p className="text-[11.5px] text-muted" data-testid="account-no-limits">
            {noLimitsWords(account.provider)}.
          </p>
        )}
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
      {useOriginal && account.import_source_path && (
        <UseOriginalModal
          account={account}
          source={account.import_source_path}
          onClose={() => setUseOriginal(false)}
          onDone={async () => {
            setUseOriginal(false);
            await onLoginRechecked();
          }}
        />
      )}
    </Card>
  );
}

/** AJ-E1501: rename an account (and set its plan). A name another
 * account has is refused in words. */
function RenameAccountModal({
  account,
  onClose,
  onDone,
}: {
  account: AccountInfo;
  onClose: () => void;
  /** `before`: the name when the sheet opened. */
  onDone: (renamed: AccountInfo, before: string) => Promise<void>;
}) {
  const [before] = useState(account.label);
  const [label, setLabel] = useState(account.label);
  const [plan, setPlan] = useState(account.plan ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <Modal title={`Rename ${account.label}`} onClose={onClose}>
      <form
        className="flex flex-col gap-4"
        data-testid="rename-account"
        onSubmit={async (event) => {
          event.preventDefault();
          setBusy(true);
          setError(null);
          try {
            await onDone(await accountRename(account.id, label, plan.trim() === "" ? null : plan), before);
          } catch (cause) {
            setError(errorText(cause));
            setBusy(false);
          }
        }}
      >
        <Field label="Name">
          <Input autoFocus value={label} onChange={(event) => setLabel(event.target.value)} data-testid="rename-account-label" maxLength={64} />
        </Field>
        <Field label="Plan (optional)" hint="Shown beside the name, like Max or Pro.">
          <Input value={plan} placeholder="Max" onChange={(event) => setPlan(event.target.value)} data-testid="rename-account-plan" />
        </Field>
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" type="submit" disabled={busy || label.trim() === ""} data-testid="rename-account-save">
            {busy ? "Saving…" : "Save"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

const OPEN = ["starting", "running", "waiting_user", "stuck", "quota_blocked"];

/** AJ-E1501: the remove sheet. An account used where it is: its folder
 * stays exactly as it is. A copy: keep its folder, or move it to the
 * Trash (never deleted). Open sessions keep it, named. */
function RemoveAccountModal({
  account,
  sessions,
  onClose,
  onDone,
}: {
  account: AccountInfo;
  sessions: InstanceInfo[];
  onClose: () => void;
  onDone: (words: string) => Promise<void>;
}) {
  const [folder, setFolder] = useState<"keep" | "trash">("keep");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const open = sessions.filter((session) => OPEN.includes(session.status));
  const names = [...new Set(open.map((session) => session.git_branch ?? ((session.cwd ?? "").replace(/\/+$/, "").split("/").pop() || session.account_label)))];
  const where = account.import_source_path ? shortPath(account.import_source_path) : null;
  return (
    <Modal title={`Remove ${account.label}?`} onClose={onClose}>
      <div className="flex flex-col gap-4 text-sm" data-testid="remove-account" data-kind={account.in_place ? "in-place" : "copy"}>
        {open.length > 0 ? (
          <p className="leading-relaxed text-muted" data-testid="remove-account-open">
            {account.label} has {open.length === 1 ? "a session" : `${open.length} sessions`} open: <span className="text-ink">{names.join(", ")}</span>.
            Stop {open.length === 1 ? "it" : "them"} first, then remove the account.
          </p>
        ) : account.in_place ? (
          <>
            <p className="leading-relaxed text-muted">
              AI Juggler forgets this account. Your folder stays exactly as it is: nothing in it is moved or changed.
            </p>
            {where && (
              <p className="truncate font-mono text-xs text-ink" title={account.import_source_path ?? undefined}>
                {where}
              </p>
            )}
          </>
        ) : (
          <>
            <p className="leading-relaxed text-muted">
              AI Juggler forgets this account.{" "}
              {where ? (
                <>
                  It's a copy of <span className="font-mono text-xs text-ink">{where}</span>, which stays as it is. What should happen to the copy?
                </>
              ) : (
                "What should happen to its folder in AI Juggler's data?"
              )}
            </p>
            <div className="flex flex-col gap-1.5" role="radiogroup" aria-label="The account's folder">
              {(
                [
                  { id: "keep", label: "Keep the folder", what: "It stays in AI Juggler's data. Nothing is deleted." },
                  { id: "trash", label: "Move it to the Trash", what: "You can put it back from the Trash." },
                ] as const
              ).map((option) => (
                <label
                  key={option.id}
                  className={
                    "flex items-center gap-2 rounded-lg border px-3 py-1.5 text-sm " +
                    (folder === option.id ? "border-accent/60 bg-accent-soft" : "border-hairline")
                  }
                >
                  <input
                    type="radio"
                    name={`remove-folder-${account.id}`}
                    checked={folder === option.id}
                    onChange={() => setFolder(option.id)}
                    data-testid={`remove-folder-${option.id}`}
                  />
                  <span className="flex flex-col">
                    <span>{option.label}</span>
                    <span className="text-[11px] text-muted">{option.what}</span>
                  </span>
                </label>
              ))}
            </div>
          </>
        )}
        {open.length === 0 && <p className="text-xs text-muted">Its past sessions stay in History, marked “removed account”.</p>}
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            {open.length > 0 ? "Close" : "Cancel"}
          </Button>
          {open.length === 0 && (
            <Button
              size="sm"
              variant="danger"
              disabled={busy}
              data-testid="remove-account-confirm"
              onClick={async () => {
                setBusy(true);
                setError(null);
                try {
                  const done = await accountRemove(account.id, folder);
                  await onDone(
                    done.folder === "trashed"
                      ? `Removed ${done.label}. Its folder is in the Trash.`
                      : done.folder === "kept"
                        ? `Removed ${done.label}. Its folder was kept.`
                        : `Removed ${done.label}. Your folder wasn't touched.`,
                  );
                } catch (cause) {
                  setError(errorText(cause));
                  setBusy(false);
                }
              }}
            >
              {busy ? "Removing…" : "Remove"}
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}

/** ADR-0018: confirm switching a copy to its original folder — what
 * changes, and that nothing is deleted. */
function UseOriginalModal({
  account,
  source,
  onClose,
  onDone,
}: {
  account: AccountInfo;
  source: string;
  onClose: () => void;
  onDone: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const where = shortPath(source);
  return (
    <Modal title="Use the original folder?" onClose={onClose}>
      <div className="flex flex-col gap-4 text-sm" data-testid="use-original">
        <p className="truncate font-mono text-xs text-ink" title={source}>
          {where}
        </p>
        <p className="leading-relaxed text-muted">
          {account.label} will use your original folder: the same sign-in, settings and history {sameAsWords([account.provider])}.
          AI Juggler's copy stays where it is (nothing is deleted); sessions you started in the copy stay in the copy, and
          won't resume from the original.
        </p>
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button onClick={onClose}>Not now</Button>
          <Button
            variant="primary"
            disabled={busy}
            data-testid="use-original-confirm"
            onClick={async () => {
              setBusy(true);
              setError(null);
              try {
                await accountUseOriginal(account.id);
                await onDone();
              } catch (cause) {
                setError(errorText(cause));
                setBusy(false);
              }
            }}
          >
            {busy ? "Switching…" : "Use the original"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

function HarnessGlyph({ provider, onClick }: { provider: string; onClick?: () => void }) {
  // Each tool's own mark, the one its sessions carry in the rail (I-01):
  // Claude's diamond, Codex's hexagon, a generic mark for the rest.
  const mark = providerMark(provider);
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={!onClick}
      title={onClick ? `Inspect ${providerDisplayName(provider)}` : providerDisplayName(provider)}
      className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-accent/20 bg-accent-soft text-accent-hi outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 enabled:hover:border-accent/40 disabled:cursor-default"
      data-testid="account-tool-mark"
      data-provider={provider}
    >
      <mark.Icon width={16} height={16} aria-hidden="true" />
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
      subtitle={`${ownSignIn(account.provider)}, opened for you — AI Juggler never sees your credentials.`}
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

/** ADR-0018: where an account lives, in a few words — "uses ~/x where
 * it is" or "a copy of ~/x"; nothing for an account made fresh here. */
export function placementWords(account: AccountInfo): string | null {
  if (!account.import_source_path) return null;
  const where = shortPath(account.import_source_path);
  return account.in_place ? `uses ${where} where it is` : `a copy of ${where}`;
}

/** `/Users/<you>/x` → `~/x`, for a line of text. */
function shortPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~").replace(/^\/home\/[^/]+/, "~");
}
