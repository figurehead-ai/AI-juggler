/**
 * Add account's fresh account (AJ-E1023): a name, and AJ-E1508's "Start
 * with the same settings as …" an account of the same tool. The account is
 * made in AI Juggler's own folder; what couldn't be copied is said before
 * sign-in.
 */
import { useMemo, useState, type JSX } from "react";
import { Button } from "../components/primitives";
import { StartWithSettings, type SettingsChoice } from "../components/StartWithSettings";
import { SwitchSetupResults } from "../components/SwitchSetupList";
import { createAccount, errorText, signedIn, type AccountInfo } from "../lib/bridge";
import { accountSettingsApply } from "../lib/ipc/accountSettings";
import type { AppliedInfo } from "../lib/ipc/switchSetup";
import { ownSignIn } from "../lib/providers";
import type { Signin } from "./FirstRun";

/** A name for a new account that no account has yet ("Claude Code 2"). */
export function freeLabel(base: string, taken: string[]): string {
  const used = new Set(taken.map((label) => label.trim().toLowerCase()));
  if (!used.has(base.toLowerCase())) return base;
  for (let n = 2; ; n += 1) if (!used.has(`${base} ${n}`.toLowerCase())) return `${base} ${n}`;
}

/** Distinct, equally light colours by position (a hash of similar paths
 * gives near-identical hues). */
const PALETTE = ["#A98BFF", "#3DD6E0", "#F59E6B", "#5BD18B", "#FF8FB8", "#6FB3FF", "#E8C547", "#C9A7FF"];
export const colourAt = (index: number) => PALETTE[index % PALETTE.length]!;

export function FreshAccountStep({
  toolName,
  provider,
  taken,
  accounts,
  canGoBack,
  onBack,
  onImportFolder,
  onCreated,
}: {
  toolName: string;
  provider: string;
  taken: string[] | null;
  /** Every account, for AJ-E1508's "Start with the same settings as …". */
  accounts: AccountInfo[];
  canGoBack: boolean;
  onBack: () => void;
  onImportFolder: () => void;
  onCreated: (signin: Signin | null) => void;
}): JSX.Element {
  const [label, setLabel] = useState<string | null>(null);
  const shown = label ?? (taken === null ? "" : freeLabel(toolName, taken));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [settings, setSettings] = useState<SettingsChoice>(null);
  // What couldn't be copied, shown before moving on.
  const [unfinished, setUnfinished] = useState<{ results: AppliedInfo[]; next: Signin | null; target: string } | null>(null);
  const sources = useMemo(() => accounts.filter((account) => account.provider === provider), [accounts, provider]);
  const create = async () => {
    setBusy(true);
    setError(null);
    try {
      const account = await createAccount(provider, shown.trim(), null);
      const next = signedIn(account.login_state)
        ? null
        : { accountId: account.id, label: account.label, loginNote: null, colour: colourAt(taken?.length ?? 0), provider };
      if (settings) {
        const results = await accountSettingsApply(account.id, settings.source, settings.picked).catch((cause) => [
          { offer: "", record: null, message: errorText(cause), note: null },
        ]);
        // What couldn't be copied, and what the user should know about
        // what was (a status line that runs a script from the other
        // account's folder), is said before moving on.
        if (results.some((result) => result.record === null || result.note !== null)) {
          setUnfinished({ results, next, target: account.label });
          setBusy(false);
          return;
        }
      }
      onCreated(next);
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };
  if (unfinished) {
    return (
      <div className="flex flex-col gap-6" data-testid="first-run-fresh-settings">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">{unfinished.target} is ready.</h1>
        <SwitchSetupResults target={unfinished.target} results={unfinished.results} />
        <div className="flex justify-end">
          <Button variant="primary" onClick={() => onCreated(unfinished.next)} data-autofocus>
            Continue
          </Button>
        </div>
      </div>
    );
  }
  return (
    <form
      className="flex flex-col gap-6"
      data-testid="first-run-fresh"
      onSubmit={(event) => {
        event.preventDefault();
        // Enter waits for what the account offers, as the button does
        // (re-review of AJ-E1508: Enter copied nothing).
        if (shown.trim().length > 0 && !busy && settings?.pending !== true) void create();
      }}
    >
      <div className="flex flex-col gap-2">
        <h1 className="text-[28px] font-semibold leading-tight tracking-tight">Add a fresh account.</h1>
        <p className="max-w-2xl text-[15px] leading-relaxed text-muted">
          AI Juggler sets it up in its own folder, apart from anything you already use. Next you sign in with{" "}
          {ownSignIn(provider)}; AI Juggler never sees your password.
        </p>
      </div>
      <label className="flex max-w-md flex-col gap-1.5 text-sm font-medium text-ink">
        Name
        <input
          autoFocus
          value={shown}
          onChange={(event) => setLabel(event.target.value)}
          placeholder="e.g. Work, Personal"
          className="rounded-lg border border-hairline bg-surface px-3 py-2.5 text-[15px] font-normal text-ink outline-none placeholder:text-faint focus:border-accent/60"
          data-testid="first-run-fresh-name"
        />
        <span className="text-xs font-normal text-faint">What it's for — you can rename it and set its plan later in Accounts.</span>
      </label>
      <StartWithSettings sources={sources} onChange={setSettings} />
      {error && (
        <p className="text-xs text-danger" role="alert" data-testid="first-run-fresh-error">
          {error}
        </p>
      )}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-5 text-sm">
          {canGoBack && (
            <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onBack}>
              Back
            </button>
          )}
          <button type="button" className="text-muted hover:text-ink hover:underline" onClick={onImportFolder}>
            Import from a folder instead…
          </button>
        </div>
        <Button variant="primary" type="submit" disabled={busy || shown.trim().length === 0 || settings?.pending === true} data-testid="first-run-fresh-create">
          {busy ? "Setting it up…" : "Create and sign in"}
        </Button>
      </div>
    </form>
  );
}
