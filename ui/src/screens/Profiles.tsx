import { RowPlaceholders } from "../components/Placeholder";
import { useApps } from "../lib/apps";
import { useState } from "react";
import { Button, Card } from "../components/primitives";
import { Field, Input, Modal } from "../components/form";
import { IconProfiles } from "../components/icons";
import { launchTitle } from "../lib/account-words";
import { cliAppName } from "../lib/app-name";
import { useConversations } from "../lib/conversations";
import { BetaMatrix } from "../components/BetaMatrix";
import { accountAdvice } from "../components/HomeRunway";
import { useAccountManage } from "./AccountManage";
import { ActionHostProvider } from "../components/ItemMenu";
import { supportsInPlace, reportsLimits } from "../lib/providers";
import { withLiveLabels } from "../lib/subscriptions";
import { useAccountsData, useToolCapabilities } from "./accounts/useAccountsData";
import { useQuotaDisplays } from "../lib/kept-reads";
import { AccountCard } from "./accounts/AccountCard";
import { AppsSection } from "./accounts/AppsSection";
import { deleteProfile } from "../lib/ipc/manage";
import { AccountsViewSwitch, useAccountsView } from "./accounts/AccountsView";
import { LoginModal } from "./accounts/LoginModal";
import { FilteredNone, OutFilterChip, outTarget } from "./accounts/OutFilter";
import { accountRoom, accountsShown, filterWord, type AccountsFilter } from "../lib/accounts-filter";
import { createProfile, launchSession } from "../lib/ipc/launch";
import { pickDirectory } from "../lib/ipc/project-folders";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import { errorText } from "../lib/bridge";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

export { placementWords } from "./accounts/UseOriginal";

/**
 * Accounts screen (M1-3; the file keeps its M1 name). AJ-E2109 (D-35):
 * an **account** is a subscription you pay for; a **profile** is a setup
 * an app runs on (a Claude app profile, a Claude Code folder), what AI
 * Juggler stores per folder. Each account is a card with its runway, and
 * the profiles signed in to it sit beneath, worked out from each
 * profile's sign-in when the screen is read. Every action maps 1:1 to a
 * typed backend command; in plain-browser preview mode the screen renders
 * mock data and launches are no-ops.
 */

export function ProfilesScreen({
  filter = null,
  onFilter,
  openTerminal,
  onInspectAccount,
  sessions = [],
  onAddAccounts,
  onFindApps,
  onAccountsChanged,
  knownAccounts = null,
  knownProfiles = null,
}: {
  /** 0.4.2: the filter it opened with ("out": the top bar's count), by
   * its id; `onFilter(null)` clears it. */
  filter?: AccountsFilter | null;
  onFilter?: (next: AccountsFilter | null) => void;
  /** 0.3.4: the app's own lists, kept current by it (`null` before its
   * first read): the screen opens on them instead of reading them again. */
  knownAccounts?: AccountInfo[] | null;
  knownProfiles?: LaunchRecipeInfo[] | null;
  /** AJ-E1501: a rename, removal or new default — the rest of the app
   * (rail, Usage, New session) reads the profiles again, no restart. */
  onAccountsChanged?: () => void;
  /** D-25: Add an account opens the find → bring in (or a fresh profile,
   * or a folder) → sign in flow; with more than one tool installed it asks
   * which tool first (AJ-E1023). */
  onAddAccounts: () => void;
  /** D-97 (3): the empty page's Find my apps: first run's scan and found
   * step (absent: Add an app's flow). */
  onFindApps?: () => void;
  openTerminal: (id: string, title: string) => void;
  /** D-25: what runs where (open counts, runway fallback). */
  sessions?: InstanceInfo[];
  onInspectAccount?: (id: string) => void;
}) {
  const [newProfileFor, setNewProfileFor] = useState<AccountInfo | null>(null);
  const [loginFor, setLoginFor] = useState<AccountInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  // AJ-E1501: what the last change did, in words ("Removed …").
  const [notice, setNotice] = useState<string | null>(null);
  const { displays } = useQuotaDisplays();
  const { all: conversations } = useConversations();
  const { inPlace, limits } = useToolCapabilities();
  const { accounts, profiles, subscriptions, refresh } = useAccountsData({ knownAccounts, knownProfiles, onError: setError });
  const apps = useApps(accounts ?? []);
  // AJ-E3208: Accounts and Apps side by side, the one last chosen first.
  const [view, choose] = useAccountsView();

  const changed = async (words: string | null) => {
    setNotice(words);
    await refresh();
    onAccountsChanged?.();
  };
  // AJ-E1537: every profile menu's rename, default and remove.
  const accountManage = useAccountManage({ sessions, changed, failed: setError });

  const launch = async (recipe: LaunchRecipeInfo) => {
    setError(null);
    try {
      const result = await launchSession(recipe.id, null);
      if (!result) {
        setError("Preview mode: no backend connected — nothing launched.");
        return;
      }
      // Interactive sessions stream straight into the terminal dock.
      openTerminal(result.session_id, launchTitle(recipe));
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  if (accounts === null || profiles === null || subscriptions === null) {
    // AJ-E1503 Req.4: card-shaped placeholders while the accounts load.
    return (
      <div className="mx-auto flex w-full max-w-content flex-col gap-5">
        <span className="block h-6 w-32 rounded bg-skeleton motion-safe:animate-pulse" aria-hidden="true" />
        <RowPlaceholders shape="card" rows={3} label="Loading accounts" />
      </div>
    );
  }
  // D-62: where to start new work, by Account, as Home's Runway says it.
  const advice = accountAdvice(accounts, subscriptions, displays, sessions);
  const summary = advice.words;
  // 0.4.2: an out Account's best action: New session on the one with room.
  const target = outTarget(advice.best);
  // Every profile under its account; one the grouping missed (just added)
  // stands alone until the next read.
  const grouped = subscriptions
    .map((sub) => ({ sub: withLiveLabels(sub, accounts), rows: sub.profiles.map((p) => accounts.find((a) => a.id === p.id)).filter((a): a is AccountInfo => a !== undefined) }))
    .filter((group) => group.rows.length > 0);
  const missed = accounts.filter((a) => !subscriptions.some((sub) => sub.profiles.some((p) => p.id === a.id)));
  for (const a of missed) {
    grouped.push({
      sub: { key: `new-${a.id}`, display_name: "Account unknown", tool: a.provider, state: "unverified", email: null, organisation: null, reason: "just added: its sign-in is read next time", profiles: [{ id: a.id, label: a.label, folder: "", plan: a.plan, renewal_date: a.renewal_date, fee_usd: null }], plan: a.plan, renewal_date: a.renewal_date, fee_usd: null, differs: [], choices: [] },
      rows: [a],
    });
  }
  // 0.4.2: out Accounts first; with the Out (or Low) filter, only them
  // (and the Accounts tab, whichever was last chosen).
  const roomOf = (group: (typeof grouped)[number]) => accountRoom(group.sub, displays);
  const shown = accountsShown(grouped, roomOf, filter);
  const shownView = filter ? "accounts" : view;

  return (
    <div className="mx-auto flex w-full max-w-content flex-col gap-5">
      <header className="flex items-end justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h2 className="text-title text-ink">Accounts</h2>
          <p className="max-w-[70ch] text-meta text-ink-2" data-testid="accounts-definition">
            An <strong className="font-semibold text-ink">account</strong> is a subscription you pay for. An{" "}
            <strong className="font-semibold text-ink">app</strong> is where you work with it, one tool with its own sign-in, like Claude Code ·
            Work Max or the Claude app · Personal; one account can serve several apps.
          </p>
          {summary && (
            <p className="text-meta text-ink" data-testid="accounts-summary">
              {summary}
            </p>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2 whitespace-nowrap">
          <Button variant="primary" size="sm" onClick={onAddAccounts} data-testid="add-account">
            <IconProfiles width={14} height={14} />
            Add an app
          </Button>
        </div>
      </header>

      {error && <Card className="border-danger/30 bg-danger-soft p-3 text-xs text-danger">{error}</Card>}
      <p role="status" className="text-xs text-ink-2 empty:hidden" data-testid="accounts-notice">
        {notice}
      </p>

      {accounts.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          {/* A filter shows Accounts: Apps clears it; × clears it and stays on Accounts. */}
          <AccountsViewSwitch view={shownView} onChange={(next) => (choose(next), next === "apps" && onFilter?.(null))} accounts={grouped.length} apps={apps?.length ?? null} />
          {filter && <OutFilterChip word={filterWord(filter)} count={shown.length} onClear={() => (choose("accounts"), onFilter?.(null))} />}
        </div>
      )}
      {accounts.length === 0 ? (
        <EmptyAccounts onFind={onFindApps ?? onAddAccounts} onAdd={onAddAccounts} />
      ) : filter && shown.length === 0 ? (
        <FilteredNone word={filterWord(filter)} />
      ) : shownView === "apps" ? (
        <ActionHostProvider host={{ ...accountManage.host, signIn: setLoginFor }}>
          <AppsSection apps={apps} subscriptions={subscriptions} sessions={sessions} conversations={conversations} now={Date.now()} />
        </ActionHostProvider>
      ) : (
        <ActionHostProvider host={{ ...accountManage.host, signIn: setLoginFor }}>
          <div className="grid grid-cols-1 items-start gap-grid 2xl:grid-cols-2">
            {shown.map(({ sub, rows }) => (
              <AccountCard
                key={sub.key}
                out={accountRoom(sub, displays) === "out" ? target : undefined}
                subscription={sub}
                apps={apps ?? undefined}
                toolWords={new Set(grouped.map((group) => group.sub.tool)).size > 1}
                profiles={rows}
                recipes={profiles}
                displays={displays}
                sessions={sessions}
                conversations={conversations}
                now={Date.now()}
                canUseInPlace={(provider) => inPlace !== null && supportsInPlace(inPlace, provider)}
                reportsLimits={(provider) => reportsLimits(limits, provider)}
                onLaunch={launch}
                onDeleteRecipe={async (id) => {
                  await deleteProfile(id);
                  void refresh();
                }}
                onNewRecipe={setNewProfileFor}
                onLogin={setLoginFor}
                onInspect={onInspectAccount}
                onRechecked={refresh}
              />
            ))}
          </div>
        </ActionHostProvider>
      )}

      {/* D-25: the pack-secrets policy lives in Settings; the matrix of
          proven switches is one click away, not on every visit. */}
      <p className="text-xs text-ink-2">
        What happens to keys and passwords in a conversation you carry to another app is in{" "}
        <button
          type="button"
          data-testid="accounts-pack-settings"
          onClick={() => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "packs" } }))}
          className="text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          Settings › Switching
        </button>
        .
      </p>
      <details className="group" data-testid="accounts-beta-matrix">
        <summary className="cursor-pointer text-xs text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
          Which switches between apps are proven
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

      {accountManage.element}

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

/** D-97 (3): no Account yet. "Find my apps" runs first run's own scan
 * and found step (the one way in, never a second scan); "Add an app"
 * stays for adding one tool's app by hand. */
function EmptyAccounts({ onFind, onAdd }: { onFind: () => void; onAdd: () => void }) {
  return (
    <Card className="flex flex-col items-center justify-center gap-3 py-14" data-testid="accounts-empty">
      <span className="flex size-11 items-center justify-center rounded-xl border border-accent/20 bg-accent-soft text-accent-hi">
        <IconProfiles width={22} height={22} />
      </span>
      <p className="text-section font-semibold text-ink">No accounts yet</p>
      <p className="max-w-md text-center text-body text-ink-2">
        Find my apps looks on this computer for the AI tools you already use and brings each one in, signed in to its account.
        Nothing is changed by looking.
      </p>
      <div className="flex flex-wrap justify-center gap-2">
        <Button variant="primary" size="sm" onClick={onFind} data-testid="accounts-find-apps">
          Find my apps
        </Button>
        <Button variant="secondary" size="sm" onClick={onAdd} data-testid="accounts-empty-add">
          Add an app
        </Button>
      </div>
    </Card>
  );
}


export function NewProfileModal({
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
      title={`Launch settings · ${cliAppName(account.provider, account.label)}`}
      subtitle="Launch settings applied when this app starts (model, environment)."
      onClose={onClose}
    >
      <div className="flex flex-col gap-4">
        <Field label="Name">
          <Input autoFocus value={name} placeholder="Deep work" onChange={(event) => setName(event.target.value)} />
        </Field>
        <Field label="Model (optional)" hint="Written to the app's managed settings">
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
