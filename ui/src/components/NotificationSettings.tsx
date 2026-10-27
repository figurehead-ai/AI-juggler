/**
 * Settings › Notifications: AJ-E611's anti-fatigue rules, now the user's
 * (quiet hours, which Accounts the hourly digest includes), and AJ-E1510's
 * macOS notifications — off until the user picks what may interrupt them.
 * The quiet hours and the Accounts left out apply to both.
 */
import { useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { SettingSwitch } from "./SettingSwitch";
import { NoAccounts, Status, accountName } from "./SettingsSections";
import { errorText, notificationPrefsGet, notificationPrefsSet, type AccountInfo, type NotificationPrefsInfo } from "../lib/bridge";
import { notificationsGet, notificationsSet, type SystemNotificationsInfo } from "../lib/ipc/notifications";
import { CardTitle } from "./CardTitle";
import { saveOptimistic } from "../lib/optimistic";

const HOURS = Array.from({ length: 24 }, (_, hour) => hour);
const hourLabel = (hour: number) => `${String(hour).padStart(2, "0")}:00`;

/** Each kind, as the switch says it and what it means. */
const KINDS: { id: keyof SystemNotificationsInfo; label: string; hint: string; on: string }[] = [
  { id: "waiting", label: "A session waits for you", hint: "It asked a question or wants your approval.", on: "waits for you" },
  { id: "stuck", label: "A session looks stuck", hint: "No output for a while, with its work unfinished.", on: "looks stuck" },
  { id: "limit", label: "A session hits its usage limit", hint: "Its account ran out until the limit resets.", on: "hits its usage limit" },
  { id: "finished", label: "A session finishes", hint: "It ended on its own, not stopped by you.", on: "finishes" },
  { id: "this_mac", label: "This Mac is getting tight", hint: "Memory, disk or heat near its limit, as This Mac shows it.", on: "" },
];

/** AJ-E1510: which kinds may become a macOS notification. */
function SystemNotifications({ onStatus }: { onStatus: (status: string | null, error: string | null) => void }): JSX.Element | null {
  const [kinds, setKinds] = useState<SystemNotificationsInfo | null>(null);
  useEffect(() => {
    void notificationsGet().then(setKinds, (cause) => onStatus(null, errorText(cause)));
  }, [onStatus]);
  if (!kinds) return null;
  const save = (id: keyof SystemNotificationsInfo, on: boolean) => {
    const kind = KINDS.find((entry) => entry.id === id);
    const what = id === "this_mac" ? "This Mac gets tight" : `a session ${kind?.on ?? ""}`;
    // §4.15: a failed save puts the switch back and says why.
    void saveOptimistic({
      previous: kinds,
      next: { ...kinds, [id]: on },
      show: setKinds,
      save: notificationsSet,
      onSaved: () => onStatus(on ? `You'll get a notification when ${what}.` : `No notification when ${what}.`, null),
      onFailed: (words) => onStatus(null, words),
    });
  };
  return (
    <fieldset className="flex flex-col gap-2 text-xs" data-testid="system-notifications">
      <legend className="mb-1 font-medium text-ink">Notify me outside AI Juggler</legend>
      {KINDS.map((kind) => (
        <SettingSwitch
          key={kind.id}
          id={`notify-${kind.id}`}
          label={kind.label}
          hint={kind.hint}
          checked={kinds[kind.id]}
          onChange={(on) => save(kind.id, on)}
        />
      ))}
      <span className="text-label leading-relaxed text-muted">
        Each shows once, never while AI Juggler is in front, and clicking it opens that session. macOS may ask once
        whether AI Juggler can notify you; if none appear, allow it in System Settings › Notifications.
      </span>
    </fieldset>
  );
}

export function NotificationSettingsCard({ accounts }: { accounts: AccountInfo[] }): JSX.Element {
  const [prefs, setPrefs] = useState<NotificationPrefsInfo | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void notificationPrefsGet().then(setPrefs, (cause) => setError(errorText(cause)));
  }, []);
  const save = (next: NotificationPrefsInfo, done: string) => {
    if (prefs === null) return;
    setError(null);
    void saveOptimistic({ previous: prefs, next, show: setPrefs, save: notificationPrefsSet, onSaved: () => setStatus(done), onFailed: setError });
  };
  const [onStatus] = useState(() => (done: string | null, failed: string | null) => {
    setError(failed);
    if (done) setStatus(done);
  });
  const quietOff = prefs !== null && prefs.quiet_start === prefs.quiet_end;
  const quietWords = (start: number, end: number) => `Quiet from ${hourLabel(start)} to ${hourLabel(end)}: the digest and notifications wait.`;
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-notifications">
      <div>
        <CardTitle>Notifications</CardTitle>
        <p className="mt-0.5 text-xs leading-relaxed text-muted">
          In AI Juggler, alerts you can act on, such as an account about to run out or a parked session ready to
          resume, show once each time; everything else waits for an hourly digest. Outside it, macOS shows only what
          you choose below.
        </p>
      </div>
      <SystemNotifications onStatus={onStatus} />
      {prefs && (
        <>
          <fieldset className="flex flex-wrap items-center gap-2 text-xs" data-testid="quiet-hours">
            <legend className="mb-1 w-full font-medium text-ink">Quiet hours</legend>
            <label className="flex items-center gap-1.5">
              <span className="text-muted">From</span>
              <select
                value={prefs.quiet_start}
                onChange={(event) => {
                  const start = Number.parseInt(event.target.value, 10);
                  save({ ...prefs, quiet_start: start }, quietWords(start, prefs.quiet_end));
                }}
                className="rounded-lg border border-hairline bg-surface-2 px-2 py-1 text-xs text-ink"
                data-testid="quiet-start"
              >
                {HOURS.map((hour) => (
                  <option key={hour} value={hour}>
                    {hourLabel(hour)}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex items-center gap-1.5">
              <span className="text-muted">to</span>
              <select
                value={prefs.quiet_end}
                onChange={(event) => {
                  const end = Number.parseInt(event.target.value, 10);
                  save({ ...prefs, quiet_end: end }, quietWords(prefs.quiet_start, end));
                }}
                className="rounded-lg border border-hairline bg-surface-2 px-2 py-1 text-xs text-ink"
                data-testid="quiet-end"
              >
                {HOURS.map((hour) => (
                  <option key={hour} value={hour}>
                    {hourLabel(hour)}
                  </option>
                ))}
              </select>
            </label>
            <span className="w-full text-label text-muted">
              {quietOff
                ? "No quiet hours: the digest comes every hour, notifications any time."
                : "The digest and notifications wait until the quiet hours end; nothing is dropped. The same hour twice means no quiet hours."}
            </span>
          </fieldset>
          <fieldset className="flex flex-col gap-1.5 text-xs" data-testid="digest-accounts">
            <legend className="mb-1 font-medium text-ink">In the digest and notifications</legend>
            {accounts.length === 0 ? (
              <NoAccounts />
            ) : (
              accounts.map((account) => {
                const muted = prefs.digest_muted_accounts.includes(account.id);
                return (
                  <label key={account.id} className="flex items-center gap-2" data-testid="digest-account">
                    <input
                      type="checkbox"
                      className="size-3.5 accent-[var(--color-accent)]"
                      checked={!muted}
                      onChange={(event) =>
                        save(
                          {
                            ...prefs,
                            digest_muted_accounts: event.target.checked
                              ? prefs.digest_muted_accounts.filter((id) => id !== account.id)
                              : [...prefs.digest_muted_accounts, account.id],
                          },
                          event.target.checked
                            ? `${accountName(account)} is back in the digest and notifications.`
                            : `${accountName(account)} is left out of the digest and notifications.`,
                        )
                      }
                    />
                    <span>{accountName(account)}</span>
                  </label>
                );
              })
            )}
          </fieldset>
        </>
      )}
      <Status status={status} error={error} />
    </Card>
  );
}
