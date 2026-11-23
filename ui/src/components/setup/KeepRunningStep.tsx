/**
 * AJ-E1545 step 4, Keep it running (AJ-E1509's always-on rules): opening
 * AI Juggler at login, where this build can (the installed app; hidden
 * where it can't, D-127), and keeping the Mac awake while sessions work
 * (on by default when plugged in; battery stays off and lives in
 * Settings › Sessions). Each switch takes effect at once (TGL-1).
 */
import { useEffect, useState, type JSX } from "react";
import { RowList, ToggleRow } from "../controls";
import { IconComputer } from "../icons-actions";
import { IconClock } from "../icons-menu";
import { appLoginItemGet, appLoginItemSet, type LoginItemInfo } from "../../lib/ipc/app";
import { sessionPrefsGet, sessionPrefsSet, type SessionPrefsInfo } from "../../lib/ipc/session-prefs";
import { errorText } from "../../lib/bridge";
import { SYSTEM_NAME, THIS_MACHINE_IN_TEXT } from "../../lib/machine-words";

/** Whether opening at login can be offered (`unavailable`: a dev build). */
export function loginOffered(state: LoginItemInfo["state"] | null): boolean {
  return state !== null && state !== "unavailable";
}

export function KeepRunningStep({ onValue }: { onValue: (atLogin: boolean | null, awake: boolean | null) => void }): JSX.Element {
  const [login, setLogin] = useState<LoginItemInfo["state"] | null>(null);
  const [prefs, setPrefs] = useState<SessionPrefsInfo | null>(null);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    void appLoginItemGet().then((info) => setLogin(info.state), () => setLogin("unavailable"));
    void sessionPrefsGet().then(setPrefs, () => setPrefs(null));
  }, []);
  const atLogin = login === "on" || login === "needs_approval";
  useEffect(() => onValue(loginOffered(login) ? atLogin : null, prefs ? prefs.keep_awake_on_power : null), [login, prefs, atLogin, onValue]);
  return (
    <div className="flex flex-col gap-4" data-testid="setup-keep">
      <p className="text-body text-ink-2">Long work runs while you're away. These keep it going; both can be changed later in Settings › Sessions.</p>
      <RowList>
        {loginOffered(login) && (
          <ToggleRow
            icon={IconComputer}
            title="Open AI Juggler at login"
            help={login === "needs_approval" ? "Waiting for your approval in System Settings › General › Login Items." : `So it's watching your sessions as soon as you log in. ${SYSTEM_NAME} shows a notice that it was added to Login Items.`}
            checked={atLogin}
            onChange={(on) => {
              setNote(null);
              void appLoginItemSet(on).then(
                (info) => setLogin(info.state),
                (cause) => setNote(errorText(cause)),
              );
            }}
            testId="setup-keep-login"
          />
        )}
        {prefs && (
          <ToggleRow
            icon={IconClock}
            title={`Keep ${THIS_MACHINE_IN_TEXT.replace(/^this/, "the")} awake while sessions work`}
            help="When plugged in, only while a session is working. The screen can still sleep, and closing the lid still sleeps it."
            checked={prefs.keep_awake_on_power}
            onChange={(on) => {
              setNote(null);
              void sessionPrefsSet({ ...prefs, keep_awake_on_power: on }).then(setPrefs, (cause) => setNote(errorText(cause)));
            }}
            testId="setup-keep-awake"
          />
        )}
      </RowList>
      {note && (
        <p className="text-meta text-danger" role="alert">
          {note}
        </p>
      )}
    </div>
  );
}
