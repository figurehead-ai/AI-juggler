/**
 * One App's row on the sign-in step (first run's and Add account's): its
 * tool's icon, a Terminal chip, its sign-in state, Sign in, and the rest
 * behind More; once started, its sign-in card (AJ-E1545 R8, AJ-E1553).
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../components/primitives";
import { SignInCard } from "../components/SignInCard";
import { TerminalChip } from "../components/SignInPanel";
import { ToolLogo } from "../components/ToolLogo";
import { Check } from "../components/StepCheck";
import { signInsDismiss } from "../lib/ipc/sign-ins";
import { useSignIns } from "../lib/sign-ins-store";
import { accountLoginCommand, accountLoginState } from "../lib/ipc/accounts";
import { signedIn } from "../lib/bridge";
import type { Signin } from "./FirstRun";

/** One account's sign-in: its tool's icon, a Terminal chip, status, Sign
 * in, and the rest behind More; once started, its sign-in card (AJ-E1545
 * R8: the tool's page and code, the terminal under Show details). */
export function SigninRow({
  entry,
  active,
  terminal,
  onStart,
  onReady,
  onSkip,
  lead = true,
}: {
  onSkip?: () => void;
  /** The step's one primary (BTN-1): the row's Sign in is filled only
   * when it is the only one left; with several, "Sign in to all" is. */
  lead?: boolean;
  entry: Signin;
  active: boolean;
  /** The sign-in's terminal, once it started: shown under the row. */
  terminal: string | null;
  onStart: () => void;
  onReady: () => void;
}): JSX.Element {
  const [state, setState] = useState<string | null>("logged_out");
  const [more, setMore] = useState(false);
  const [command, setCommand] = useState<string | null>(null);
  const [skipped, setSkipped] = useState(false);
  // AJ-E1553: the sign-in's own entry (the backend sees it finish).
  const signIn = useSignIns().find((one) => one.id === terminal) ?? null;
  const ready = signedIn(state) || signIn?.state === "signed_in";
  // F29: once it ended, the row says so; the card below has Try again.
  const ended = active && signIn?.state === "ended";
  // Every 2 s while waiting (the login-queue card's rule): flips the row
  // the moment the vendor's sign-in completes; never steals focus.
  useEffect(() => {
    if (ready) return undefined;
    let alive = true;
    const tick = () =>
      void accountLoginState(entry.accountId).then(
        (next) => alive && setState(next),
        () => null,
      );
    tick();
    const timer = window.setInterval(tick, 2000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [entry.accountId, ready]);
  useEffect(() => {
    if (ready) onReady();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready]);
  return (
    <li className="flex flex-col border-t border-hairline/60 first:border-t-0" data-testid="first-run-signin-row">
      <div className="flex flex-wrap items-center gap-4 px-5 py-3">
        <ToolLogo tool={entry.provider} kind="cli" size={24} />
        <span className="flex min-w-0 flex-1 items-center gap-2">
          <span className="truncate text-section font-medium text-ink">{entry.label}</span>
          <TerminalChip />
        </span>
        <output className="text-xs">
          {ready ? (
            <span className="flex items-center gap-1.5 text-success">
              <Check />
              {state === "api_key" ? "Signed in with an API key" : "Signed in"}
            </span>
          ) : skipped ? (
            <span className="text-muted">Later</span>
          ) : ended ? (
            <span className="text-muted">Didn't finish</span>
          ) : active ? (
            <span className="text-live">Finish signing in below</span>
          ) : (
            <span className="text-muted">Not signed in yet</span>
          )}
        </output>
        {!ready && (
          <>
            {!ended && (
              <Button size="sm" variant={active || !lead ? "secondary" : "primary"} onClick={onStart}>
                {active ? "Open again" : "Sign in"}{" "}
                <span className="sr-only">{entry.label}</span>
              </Button>
            )}
            <button
              type="button"
              className="rounded-md px-2 py-1 text-muted hover:bg-surface-2 hover:text-ink"
              aria-expanded={more}
              aria-label={`More for ${entry.label}`}
              onClick={() => {
                setMore((open) => !open);
                if (command === null) void accountLoginCommand(entry.accountId).then(setCommand, () => setCommand(""));
              }}
            >
              ⋯
            </button>
          </>
        )}
      </div>
      {active && terminal && !ready && (
        <div className="mx-5 mb-4 rounded-xl bg-surface-2 p-4" data-testid="first-run-signin-terminal">
          <SignInCard
            signIn={signIn}
            terminalId={terminal}
            tool={entry.provider}
            title={`Sign in · ${entry.label}`}
            onTryAgain={() => {
              void signInsDismiss(terminal).catch(() => null);
              onStart();
            }}
          />
        </div>
      )}
      {more && !ready && (
        <div className="flex flex-col gap-2 px-5 pb-3 pl-11 text-xs text-muted">
          {entry.loginNote && <span>{entry.loginNote}</span>}
          {command && (
            <span>
              Or run it yourself: <span className="font-mono text-ink">{command}</span>
            </span>
          )}
          {/* F29: one app alone has the step's own Sign in later; a row's
              own is for one of several, worded apart from it. */}
          {onSkip && (
            <button type="button" className="self-start text-accent-hi hover:underline" onClick={() => {
              if (!skipped) onSkip();
              setSkipped(true);
            }}>
              Later for this app
            </button>
          )}
        </div>
      )}
    </li>
  );
}
