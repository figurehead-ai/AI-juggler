import { useEffect, useRef, useState } from "react";
import { Button } from "./primitives";
import { IconTerminal } from "./icons";
import { accountLoginCommand, accountLoginState, loginOpenWeb } from "../lib/ipc/accounts";
import { errorText, signedIn } from "../lib/bridge";
import { terminalSpawnLogin } from "../lib/ipc/terminal-io";

/**
 * AJ-E308 Req.2/3/4: the ONE login-queue card shared by all three
 * doors (wizard, ReadyHome, Profiles modal) — identical states,
 * copy, and error handling. No door uses `window.alert` or leaves a
 * rejection unhandled.
 *
 * While mounted it polls the account's login state every 2 s (never
 * stealing focus) and re-probes immediately when the login terminal
 * exits (`terminalEnded` flips true); [Re-check] stays as the
 * fallback, never the only path. The status line is aria-live so
 * screen readers hear "Signed in" the moment it lands.
 *
 * 0.5.1 F82: one status mark (its colour carries the state), the steps
 * as a short numbered list, the tool named instead of "vendor", and a
 * tonal card (no outline, PAT-SURF-1).
 */

/** Per-state headline + action copy (Req.1/4) — words, never codes. */
export function loginStateCopy(state: string | null | undefined): {
  headline: string;
  detail: string;
  reopenLabel?: string;
} {
  switch (state) {
    case "signed_in":
    case "logged_in":
      return { headline: "Signed in", detail: "Detected here. Ready to launch anytime." };
    case "partial":
      return {
        headline: "Sign-in didn't finish",
        detail: "The sign-in stopped halfway. Reopen it and finish the steps.",
        reopenLabel: "Reopen sign-in",
      };
    case "expired":
      return {
        headline: "Sign-in expired",
        detail: "The saved sign-in expired. Sign in once more and it stays fresh.",
        reopenLabel: "Sign in again",
      };
    case "sso_pending":
      return {
        headline: "Finish in your browser",
        detail: "Finish in your browser/IdP window, then return here — we detect it.",
      };
    case "api_key":
      return {
        headline: "API key account",
        detail: "This app runs on an API key, so it needs no sign-in.",
      };
    case "unknown":
      return {
        headline: "Can't check right now",
        detail: "The sign-in check couldn't run. Sign in when online; nothing was changed.",
      };
    default:
      return { headline: "Waiting…", detail: "Not signed in yet. We detect it here once you finish." };
  }
}

export function LoginQueueCard({
  accountId,
  label,
  providerName,
  initialState,
  loginNote,
  terminalEnded = false,
  onSpawned,
  onReady,
  onSkip,
  onDone,
  compact = false,
}: {
  accountId: string;
  label: string;
  providerName: string;
  initialState: string | null | undefined;
  loginNote?: string | null;
  /** Flips true when the login terminal exited — an immediate
   * re-probe beats the next poll tick (Req.2). */
  terminalEnded?: boolean;
  onSpawned?: (terminalId: string) => void;
  /** Fired when the poll auto-detects a ready state — the queue
   * advances without any click (Req.2's "we detect it"). */
  onReady?: () => void;
  onSkip?: () => void;
  onDone?: () => void;
  compact?: boolean;
}) {
  const [state, setState] = useState(initialState);
  const [command, setCommand] = useState("");
  const [copied, setCopied] = useState(false);
  const [spawnError, setSpawnError] = useState<string | null>(null);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    void accountLoginCommand(accountId).then((found) => {
      if (alive.current) setCommand(found);
    });
    return () => {
      alive.current = false;
    };
  }, [accountId]);

  // The 2-second poll (Req.2): runs only while mounted, never
  // touches focus; stops itself once signed in.
  useEffect(() => {
    if (signedIn(state)) return;
    const tick = async () => {
      const next = await accountLoginState(accountId).catch(() => null);
      if (alive.current && next !== null) setState(next);
    };
    void tick();
    const timer = window.setInterval(() => void tick(), 2_000);
    return () => window.clearInterval(timer);
  }, [accountId, state]);

  // PTY-exit re-probe: the terminal ended — probe NOW.
  useEffect(() => {
    if (!terminalEnded) return;
    void accountLoginState(accountId).then((next) => {
      if (alive.current) setState(next);
    });
  }, [terminalEnded, accountId]);

  const ready = signedIn(state);
  const copy = loginStateCopy(state);

  // Auto-detect hands the moment to the queue (no click needed).
  useEffect(() => {
    if (ready) onReady?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready]);

  const spawn = async () => {
    setSpawnError(null);
    try {
      const terminal = await terminalSpawnLogin(accountId);
      onSpawned?.(terminal.id);
    } catch (cause) {
      // Handled inline — never an alert, never an unhandled
      // rejection (Req.3).
      setSpawnError(errorText(cause));
    }
  };

  // One mark before the headline; its colour carries the state: green
  // when ready, amber when the sign-in needs redoing, a quiet dot when
  // it can't be checked, the live breathing dot while we wait.
  const mark = ready
    ? "bg-success"
    : state === "expired" || state === "partial"
      ? "bg-warning"
      : state === "unknown"
        ? "bg-faint"
        : "bg-live animate-breathe";

  return (
    <div
      className="flex flex-col gap-3 rounded-xl bg-surface-2 px-4 py-3.5"
      data-testid="login-queue-card"
      data-state={state ?? "unknown"}
    >
      <div className="flex flex-wrap items-center gap-2">
        <span aria-hidden="true" className={"size-2 shrink-0 rounded-full " + mark} data-testid="login-mark" />
        <p className="text-xs font-medium text-ink" aria-live="polite" data-testid="login-status">
          {copy.headline}
        </p>
        <span className="text-label text-muted">·</span>
        <span className="text-label text-muted">{label}</span>
        <span className="flex-1" />
        {onDone && ready && (
          <Button size="sm" variant="primary" onClick={onDone}>
            Continue
          </Button>
        )}
      </div>

      <p className="text-label leading-relaxed text-muted" data-testid="login-detail">
        {copy.detail}
        {loginNote ? ` ${loginNote}` : ""}
      </p>

      {!ready && state !== "api_key" && (
        <>
          <ol className="flex list-decimal flex-col gap-0.5 pl-5 text-label leading-relaxed text-ink-2" data-testid="login-steps">
            <li>Choose {copy.reopenLabel ?? "Sign in"}: {providerName}'s own sign-in opens.</li>
            <li>Follow {providerName}'s prompts.</li>
            <li>Come back here: we detect it.</li>
          </ol>
          {!compact && (
            <div className="rounded-lg bg-surface-3 px-3 py-2 font-mono text-label leading-relaxed text-success break-all">
              {command || "…"}
            </div>
          )}
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="primary" onClick={() => void spawn()}>
              <IconTerminal width={14} height={14} />
              {copy.reopenLabel ?? "Sign in"}
            </Button>
            <Button size="sm" variant="secondary" onClick={() => void loginOpenWeb(providerName)}>
              Open {providerName}'s sign-in page
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={async () => {
                await navigator.clipboard.writeText(command).catch(() => null);
                setCopied(true);
                window.setTimeout(() => setCopied(false), 1500);
              }}
            >
              {copied ? "Copied" : "Copy command"}
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                void accountLoginState(accountId).then((next) => setState(next))
              }
            >
              Re-check
            </Button>
            {onSkip && (
              <Button size="sm" variant="secondary" onClick={onSkip} data-testid="signin-skip">
                Skip for now
              </Button>
            )}
          </div>
        </>
      )}

      {spawnError && (
        <p className="text-label text-danger" role="alert">
          {spawnError}
        </p>
      )}
    </div>
  );
}
