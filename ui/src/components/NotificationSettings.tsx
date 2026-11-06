/**
 * Settings › Notifications: AJ-E611's anti-fatigue rules, now the user's
 * (quiet hours, which Accounts the hourly digest includes), and AJ-E1510's
 * system notifications — off until the user picks what may interrupt them.
 * The quiet hours and the Accounts left out apply to both.
 *
 * Design review 29–31: a card per part, each headed in the section-title
 * style; the rarely changed quiet hours and digest fold behind one line
 * that says their values.
 */
import { useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { SettingSwitch } from "./SettingSwitch";
import { NoAccounts, Status, accountName } from "./SettingsSections";
import { errorText, notificationPrefsGet, notificationPrefsSet, type AccountInfo, type NotificationPrefsInfo } from "../lib/bridge";
import { notificationsGet, notificationsSendTest, notificationsSet, type SystemNotificationsInfo } from "../lib/ipc/notifications";
import { Button } from "./primitives";
import { CardTitle } from "./CardTitle";
import { saveOptimistic } from "../lib/optimistic";
import { Checkbox, Disclosure, Picker } from "./controls";
import { THIS_MACHINE } from "../lib/machine-words";

const HOURS = Array.from({ length: 24 }, (_, hour) => hour);
/** AJ-E2904: the choices, as the backend accepts them. */
const WAITS = [0, 2, 5, 10, 15, 30];
const LEVELS = [50, 70, 80, 90, 95, 99];
const waitLabel = (minutes: number) => (minutes === 0 ? "Right away" : `${minutes} minutes`);
const hourLabel = (hour: number) => `${String(hour).padStart(2, "0")}:00`;

/** Each kind, as the switch says it, what it means, and the end of
 * "You'll get a notification when …". */
const KINDS: { id: keyof SystemNotificationsInfo; label: string; hint: string; when: string }[] = [
  { id: "waiting", label: "A session waits for you", hint: "It asked a question or wants your approval.", when: "a session waits for you" },
  { id: "stuck", label: "A session looks stuck", hint: "No output for a while, with its work unfinished.", when: "a session looks stuck" },
  { id: "limit", label: "A session hits its usage limit", hint: "Its account ran out until the limit resets.", when: "a session hits its usage limit" },
  { id: "finished", label: "A session finishes", hint: "It ended on its own, not stopped by you.", when: "a session finishes" },
  { id: "left_behind", label: "A session leaves something running", hint: "It ended and left processes running, such as a dev server. Once per session.", when: "a session leaves something running" },
  // AJ-E2501: what AI Juggler notices itself.
  { id: "quota", label: "An account is almost out of room", hint: "Its 5-hour or weekly window is nearly used up. Once until it resets.", when: "an account is almost out of room" },
  { id: "crash", label: "A session crashes and is restarted", hint: "It crashed and AI Juggler started it again, if you let it.", when: "a session crashes and is restarted" },
  { id: "reset", label: "An account has room again", hint: "Its window reset after it was almost out.", when: "an account has room again" },
  { id: "this_mac", label: `${THIS_MACHINE} is getting tight`, hint: `Memory, disk or heat near its limit, as ${THIS_MACHINE} shows it.`, when: `${THIS_MACHINE} gets tight` },
];

/** A card's heading: the section-title style (design review 31). */
const HEADING = "text-section font-semibold text-ink";

/** The fold's line: the quiet hours and who's in the digest, as set. */
export function moreWords(prefs: Pick<NotificationPrefsInfo, "quiet_start" | "quiet_end" | "digest_muted_accounts">, accounts: readonly { id: string }[]): string {
  const quiet = prefs.quiet_start === prefs.quiet_end ? "no quiet hours" : `quiet hours ${hourLabel(prefs.quiet_start)}–${hourLabel(prefs.quiet_end)}`;
  if (accounts.length === 0) return quiet;
  const out = accounts.filter((account) => prefs.digest_muted_accounts.includes(account.id)).length;
  const digest = out === 0 ? "all accounts" : out === 1 ? "1 account left out" : `${out} accounts left out`;
  return `${quiet}, ${digest}`;
}

/** AJ-E1510: which kinds may become a system notification. */
function SystemNotifications({ onStatus }: { onStatus: (status: string | null, error: string | null) => void }): JSX.Element | null {
  const [kinds, setKinds] = useState<SystemNotificationsInfo | null>(null);
  useEffect(() => {
    void notificationsGet().then(setKinds, (cause) => onStatus(null, errorText(cause)));
  }, [onStatus]);
  if (!kinds) return null;
  const save = (id: keyof SystemNotificationsInfo, on: boolean) => {
    const what = KINDS.find((entry) => entry.id === id)?.when ?? "";
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
      <legend className={`mb-2 ${HEADING}`}>Notify me outside AI Juggler</legend>
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
        Each shows once, never while AI Juggler is in front, and clicking it opens that session. The system may ask
        once whether AI Juggler can notify you; if none appear, allow it in System Settings › Notifications.
      </span>
    </fieldset>
  );
}

/** AJ-E2904: when a wait becomes news, the two warnings for an account
 * running out, and Send test. */
function WhenToTell({
  prefs,
  save,
  onStatus,
}: {
  prefs: NotificationPrefsInfo;
  save: (next: NotificationPrefsInfo, done: string) => void;
  onStatus: (status: string | null, error: string | null) => void;
}): JSX.Element {
  const [sent, setSent] = useState<string | null>(null);
  const sendTest = () => {
    setSent(null);
    notificationsSendTest().then(setSent, (cause: unknown) => onStatus(null, errorText(cause)));
  };
  const level = (pct: number) => ({ value: String(pct), label: `${pct} %` });
  return (
    <fieldset className="flex flex-col gap-2 text-xs" data-testid="when-to-tell">
      <legend className={`mb-2 ${HEADING}`}>When to tell me</legend>
      <label className="flex flex-wrap items-center gap-2">
        <span className="text-ink-2">Tell me when a session waits longer than</span>
        <Picker choice
          size="sm"
          ariaLabel="Tell me when a session waits longer than"
          value={String(prefs.wait_after_min)}
          onChange={(value) => {
            const minutes = Number.parseInt(value, 10);
            save(
              { ...prefs, wait_after_min: minutes },
              minutes === 0 ? "A waiting session is told right away." : `A waiting session is told once it has waited ${minutes} minutes.`,
            );
          }}
          testId="wait-after"
          options={WAITS.map((minutes) => ({ value: String(minutes), label: waitLabel(minutes) }))}
        />
      </label>
      <span className="text-label text-muted">Needs you and the bell show a wait at once either way.</span>
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-ink-2">Warn when an account has used</span>
        <Picker choice
          size="sm"
          ariaLabel="First warning at"
          value={String(prefs.quota_first_pct)}
          onChange={(value) => {
            const first = Number.parseInt(value, 10);
            save({ ...prefs, quota_first_pct: first }, `The first warning comes at ${first} % used.`);
          }}
          testId="quota-first"
          options={LEVELS.filter((pct) => pct < prefs.quota_last_pct).map(level)}
        />
        <span className="text-ink-2">and again at</span>
        <Picker choice
          size="sm"
          ariaLabel="Last warning at"
          value={String(prefs.quota_last_pct)}
          onChange={(value) => {
            const last = Number.parseInt(value, 10);
            save({ ...prefs, quota_last_pct: last }, `The last warning comes at ${last} % used.`);
          }}
          testId="quota-last"
          options={LEVELS.filter((pct) => pct > prefs.quota_first_pct).map(level)}
        />
      </div>
      <span className="text-label text-muted">Once each per 5-hour or weekly window. An estimate says "about".</span>
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="secondary" onClick={sendTest} data-testid="notify-send-test">
          Send test
        </Button>
        {sent && (
          <span className="text-label text-ink-2" role="status" data-testid="notify-test-sent">
            {sent}
          </span>
        )}
      </div>
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
    <div className="flex flex-col gap-3" data-testid="settings-notifications">
      <div className="px-1">
        <CardTitle>Notifications</CardTitle>
        <p className="mt-0.5 text-xs leading-relaxed text-ink-2">
          In AI Juggler, alerts you can act on, such as an account about to run out or a session waiting to resume,
          show once each time; everything else waits for an hourly digest. Outside it, the system shows only what you
          choose below.
        </p>
      </div>
      <Card className="flex flex-col gap-3 p-5">
        <SystemNotifications onStatus={onStatus} />
      </Card>
      {prefs && (
        <Card className="flex flex-col gap-3 p-5">
          <WhenToTell prefs={prefs} save={save} onStatus={onStatus} />
        </Card>
      )}
      {prefs && (
        <Card className="p-5">
          <Disclosure
            testId="notifications-more"
            summaryClassName="text-body"
            summary={
              <span>
                <span className="font-semibold text-ink">More:</span> <span className="text-ink-2">{moreWords(prefs, accounts)}</span>
              </span>
            }
          >
            <div className="mt-3 flex flex-col gap-4 border-t border-hairline pt-4">
              <fieldset className="flex flex-wrap items-center gap-2 text-xs" data-testid="quiet-hours">
                <legend className={`mb-2 w-full ${HEADING}`}>Quiet hours</legend>
                <div className="flex items-center gap-1.5">
                  <span className="text-muted">From</span>
                  <Picker choice
                    size="sm"
                    ariaLabel="Quiet hours from"
                    value={String(prefs.quiet_start)}
                    onChange={(value) => {
                      const start = Number.parseInt(value, 10);
                      save({ ...prefs, quiet_start: start }, quietWords(start, prefs.quiet_end));
                    }}
                    testId="quiet-start"
                    options={HOURS.map((hour) => ({ value: String(hour), label: hourLabel(hour) }))}
                  />
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="text-muted">to</span>
                  <Picker choice
                    size="sm"
                    ariaLabel="Quiet hours to"
                    value={String(prefs.quiet_end)}
                    onChange={(value) => {
                      const end = Number.parseInt(value, 10);
                      save({ ...prefs, quiet_end: end }, quietWords(prefs.quiet_start, end));
                    }}
                    testId="quiet-end"
                    options={HOURS.map((hour) => ({ value: String(hour), label: hourLabel(hour) }))}
                  />
                </div>
                <span className="w-full text-label text-muted">
                  {quietOff
                    ? "No quiet hours: the digest comes every hour, notifications any time."
                    : "The digest and notifications wait until the quiet hours end; nothing is dropped. The same hour twice means no quiet hours."}
                </span>
              </fieldset>
              <fieldset className="flex flex-col gap-1.5 text-xs" data-testid="digest-accounts">
                <legend className={`mb-2 ${HEADING}`}>In the digest and notifications</legend>
                {accounts.length === 0 ? (
                  <NoAccounts />
                ) : (
                  accounts.map((account) => {
                    const muted = prefs.digest_muted_accounts.includes(account.id);
                    return (
                      <label key={account.id} className="flex items-center gap-2" data-testid="digest-account">
                        <Checkbox
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
            </div>
          </Disclosure>
        </Card>
      )}
      <Status status={status} error={error} />
    </div>
  );
}
