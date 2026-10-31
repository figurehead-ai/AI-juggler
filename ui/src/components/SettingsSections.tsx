/**
 * Settings › the policies the user owns (wireframe 26; D-08, D-12, D-14):
 * Cleanup, Accounts & renewal dates and Fleet permissions (Notifications:
 * NotificationSettings.tsx).
 * Each is the user's choice, remembered, and shown before it takes
 * effect; nothing here is inferred or switched on by AI Juggler.
 */
import { revertedWords, saveOptimistic } from "../lib/optimistic";
import { useCallback, useEffect, useState, type JSX } from "react";
import { Button, Card, Chip } from "./primitives";
import { TierToggles } from "../pages/AccountPage";
import { AccessSheet } from "../screens/FleetScreen";
import { providerDisplayName } from "../lib/providers";
import {
  accountSetRenewal,
  appSettingGet,
  appSettingSet,
  cleanupDefaultGet,
  cleanupDefaultSet,
  cleanupPauseSet,
  cleanupStatus,
  errorText,
  fleetPeers,
  fleetSetPermission,
  type AccountInfo,
  type FleetPeerInfo,
} from "../lib/bridge";
import { CardTitle } from "./CardTitle";
import { Checkbox, Picker } from "./controls";

export function accountName(account: AccountInfo): string {
  return `${providerDisplayName(account.provider)} · ${account.label}`;
}

export function NoAccounts(): JSX.Element {
  return <p className="text-xs text-ink-2">No Accounts yet. Add one on the Accounts screen.</p>;
}

export function Status({ status, error }: { status: string | null; error: string | null }): JSX.Element | null {
  if (error)
    return (
      <p className="text-label text-danger" role="alert">
        {error}
      </p>
    );
  if (status)
    return (
      <p className="text-label text-muted" role="status">
        {status}
      </p>
    );
  return null;
}

/** D-08: automatic cleanup is opt-in per Account; the default for new
 * Accounts is off unless the user picks it. */
export function CleanupSettingsCard({ accounts }: { accounts: AccountInfo[] }): JSX.Element {
  const [defaultOn, setDefaultOn] = useState<boolean | null>(null);
  const [paused, setPaused] = useState<boolean | null>(null);
  // The pause switch reads the live status; if that fails it says so and
  // offers a retry instead of sitting disabled.
  const [pausedError, setPausedError] = useState<string | null>(null);
  const loadPaused = () => {
    setPausedError(null);
    void cleanupStatus().then(
      (now) => setPaused(now.paused),
      (cause) => {
        setPaused(null);
        setPausedError(errorText(cause));
      },
    );
  };
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void cleanupDefaultGet().then(setDefaultOn, (cause) => setError(errorText(cause)));
    loadPaused();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- once, on open
  }, []);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-cleanup">
      <div>
        <CardTitle>Cleanup</CardTitle>
        <p className="mt-0.5 text-xs leading-relaxed text-ink-2">
          AI Juggler tracks every process it starts. When one is left behind, it tells you; it only tidies up by
          itself for an account you've turned that on for, and only dev servers and watchers it verified as that
          account's. Shells, MCP servers and databases are never stopped automatically, and every stop is
          recorded. Tidying after a crash waits 5 minutes and asks first, with Undo.
        </p>
      </div>
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <span className="font-medium text-ink">Default for new accounts</span>
        <Picker
          size="sm"
          ariaLabel="Default for new accounts"
          value={defaultOn === null ? "" : defaultOn ? "tidy" : "notify"}
          disabled={defaultOn === null}
          onChange={(value) => {
            const on = value === "tidy";
            setError(null);
            void cleanupDefaultSet(on).then(
              (saved) => {
                setDefaultOn(saved);
                setStatus(
                  saved
                    ? "New Accounts will tidy left-behind dev servers. Existing Accounts keep their own choice."
                    : "New Accounts will notify only. Existing Accounts keep their own choice.",
                );
              },
              (cause) => setError(errorText(cause)),
            );
          }}
          testId="cleanup-default"
          options={[
            { value: "notify", label: "Notify only" },
            { value: "tidy", label: "Tidy left-behind dev servers" },
          ]}
        />
      </div>
      {/* AJ-E508 Req.4: the global kill-switch, here and in Cleanup's ⋯. */}
      <label className="flex items-start gap-2 text-xs">
        <Checkbox
          checked={paused === true}
          disabled={paused === null}
          onChange={(event) => {
            const next = event.target.checked;
            setError(null);
            void cleanupPauseSet(next).then(
              (now) => {
                setPaused(now.paused);
                setStatus(now.paused ? "Automatic cleanup is paused." : "Automatic cleanup is on again for the Accounts that have it.");
              },
              (cause) => setError(errorText(cause)),
            );
          }}
          className="mt-0.5"
          data-testid="settings-cleanup-pause"
        />
        <span className="flex flex-col gap-0.5">
          <span className="font-medium text-ink">Pause automatic cleanup</span>
          <span className="text-muted">For every account, until you turn this off. Nothing is stopped by itself while paused.</span>
        </span>
      </label>
      {pausedError && (
        <p className="flex flex-wrap items-center gap-2 text-xs text-danger" role="alert" data-testid="settings-cleanup-pause-error">
          Couldn't read whether automatic cleanup is paused: {pausedError}
          <Button size="sm" variant="secondary" onClick={loadPaused}>
            Try again
          </Button>
        </p>
      )}
      <Status status={status} error={error} />
      <div className="flex flex-col divide-y divide-hairline" data-testid="cleanup-accounts">
        {accounts.length === 0 ? (
          <NoAccounts />
        ) : (
          accounts.map((account) => (
            <div key={account.id} className="flex flex-col gap-1.5 py-2.5 text-xs" data-testid="cleanup-account">
              <span className="font-medium text-ink">{accountName(account)}</span>
              <TierToggles accountId={account.id} initial={account.cleanup_opt_in} explain={false} />
            </div>
          ))
        )}
      </div>
    </Card>
  );
}

const WARN_DAYS = [1, 3, 7, 14, 30];

function RenewalRow({ account }: { account: AccountInfo }): JSX.Element {
  const [date, setDate] = useState(account.renewal_date ?? "");
  const [error, setError] = useState<string | null>(null);
  const id = `settings-renewal-${account.id}`;
  const commit = (next: string | null) => {
    setError(null);
    const previous = account.renewal_date ?? "";
    void accountSetRenewal(account.id, next).then(
      (saved) => setDate(saved.renewal_date ?? ""),
      (cause) => {
        // §4.15: the date goes back to the saved one, and says why.
        setDate(previous);
        setError(revertedWords(cause));
      },
    );
  };
  return (
    <div className="flex flex-wrap items-center gap-2 py-2.5 text-xs" data-testid="settings-renewal">
      <label htmlFor={id} className="min-w-44 font-medium text-ink">
        {accountName(account)}
      </label>
      <input
        id={id}
        type="date"
        value={date}
        onChange={(event) => {
          setDate(event.target.value);
          commit(event.target.value === "" ? null : event.target.value);
        }}
        className="rounded-lg border border-hairline bg-surface px-1.5 py-0.5 text-xs text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
      />
      {date ? (
        <Button size="sm" variant="secondary" onClick={() => commit(null)}>
          Clear{" "}<span className="sr-only">the renewal date for {accountName(account)}</span>
        </Button>
      ) : (
        <span className="text-label text-muted">Not set: no renewal reminders</span>
      )}
      {error && (
        <span className="text-label text-danger" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}

/** D-12: renewal dates are entered by the user, never guessed. */
export function RenewalSettingsCard({ accounts }: { accounts: AccountInfo[] }): JSX.Element {
  const [warnDays, setWarnDays] = useState<number | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  useEffect(() => {
    void appSettingGet("renewal_warn_days").then((raw) => {
      const parsed = raw === null ? Number.NaN : Number.parseInt(raw, 10);
      setWarnDays(Number.isNaN(parsed) ? 7 : Math.min(30, Math.max(1, parsed)));
    });
  }, []);
  const options = warnDays !== null && !WARN_DAYS.includes(warnDays) ? [...WARN_DAYS, warnDays].sort((a, b) => a - b) : WARN_DAYS;
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-renewal-card">
      <div>
        <CardTitle>Accounts &amp; renewal dates</CardTitle>
        <p className="mt-0.5 text-xs leading-relaxed text-ink-2">
          When each subscription renews, if you want reminders. AI Juggler never reads your billing and never guesses
          a date: an account without one simply gets no renewal reminders.
        </p>
      </div>
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <span className="font-medium text-ink">Remind me</span>
        <Picker
          size="sm"
          ariaLabel="Remind me"
          value={warnDays === null ? "" : String(warnDays)}
          disabled={warnDays === null}
          onChange={(value) => {
            const days = Number.parseInt(value, 10);
            // §4.15: a failed save puts the old choice back and says why.
            void saveOptimistic({
              previous: warnDays,
              next: days,
              show: setWarnDays,
              save: () => appSettingSet("renewal_warn_days", String(days)),
              onSaved: () => setStatus(`Renewal reminders start ${days} ${days === 1 ? "day" : "days"} before.`),
              onFailed: setStatus,
            });
          }}
          testId="renewal-warn-days"
          options={options.map((days) => ({ value: String(days), label: `${days} ${days === 1 ? "day" : "days"}` }))}
        />
        <span className="text-muted">before a renewal</span>
      </div>
      <Status status={status} error={null} />
      <div className="flex flex-col divide-y divide-hairline">
        {accounts.length === 0 ? <NoAccounts /> : accounts.map((account) => <RenewalRow key={account.id} account={account} />)}
      </div>
    </Card>
  );
}

/** What each paired Mac may do on this one (the same sheet as Fleet). */
export function FleetPermissionsCard(): JSX.Element {
  const [peers, setPeers] = useState<FleetPeerInfo[] | null>(null);
  const [editing, setEditing] = useState<FleetPeerInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const reload = useCallback(async () => {
    setPeers(await fleetPeers());
  }, []);
  useEffect(() => {
    void reload().catch((cause) => setError(errorText(cause)));
  }, [reload]);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-fleet">
      <div>
        <CardTitle>Fleet permissions</CardTitle>
        <p className="mt-0.5 text-xs leading-relaxed text-ink-2">
          What each of your paired Macs may do on this one. Only this Mac can change it, and no Mac can ever read
          another's credentials.
        </p>
      </div>
      {peers !== null && peers.length === 0 && (
        <p className="text-xs text-ink-2">No paired Macs. Pair one on the Fleet screen.</p>
      )}
      {peers !== null && peers.length > 0 && (
        <ul className="flex flex-col divide-y divide-hairline">
          {peers.map((peer) => (
            <li key={peer.machine_id} className="flex flex-wrap items-center gap-3 py-2.5" data-testid="settings-fleet-peer">
              <span className="min-w-0 flex-1 text-sm font-medium">{peer.label}</span>
              <Chip tone="neutral">
                {peer.permission === "control" && peer.ask_each_time ? "View + control · asks each time" : peer.permission_label}
              </Chip>
              <Button size="sm" variant="ghost" onClick={() => setEditing(peer)}>
                Access…{" "}<span className="sr-only">for {peer.label} on this Mac</span>
              </Button>
            </li>
          ))}
        </ul>
      )}
      <Status status={null} error={error} />
      {editing !== null && (
        <AccessSheet
          peer={editing}
          busy={busy}
          onCancel={() => setEditing(null)}
          onSave={(permission, ask) => {
            setBusy(true);
            setError(null);
            void fleetSetPermission(editing.machine_id, permission, ask)
              .then(reload)
              .then(
                () => setEditing(null),
                (cause) => setError(errorText(cause)),
              )
              .finally(() => setBusy(false));
          }}
        />
      )}
    </Card>
  );
}
