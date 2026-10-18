import { useEffect, useRef, useState } from "react";
import { Button, Chip } from "./primitives";
import { IconTerminal } from "./icons";
import {
  accountLoginCommand,
  accountLoginState,
  errorText,
  loginOpenWeb,
  signedIn,
  terminalSpawnLogin,
} from "../lib/bridge";

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
 * screen readers hear "● Signed in ✓ auto-detected" the moment it
 * lands.
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
      return { headline: "Signed in", detail: "Ready — launch anytime." };
    case "partial":
      return {
        headline: "Sign-in didn't finish",
        detail: "The sign-in was abandoned halfway — reopen it and follow the vendor prompts.",
        reopenLabel: "Reopen sign-in",
      };
    case "expired":
      return {
        headline: "Sign-in expired",
        detail: "The saved login expired — sign in once more and it stays fresh.",
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
        detail: "This account runs on an API key — no vendor sign-in needed.",
      };
    case "unknown":
      return {
        headline: "Can't check right now",
        detail: "The login check couldn't run — sign in when online; nothing was changed.",
      };
    default:
      return { headline: "Waiting…", detail: "Complete the vendor's sign-in; we detect it here." };
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

  return (
    <div
      className="flex flex-col gap-3 rounded-xl border border-hairline px-4 py-3.5"
      data-testid="login-queue-card"
      data-state={state ?? "unknown"}
    >
      <div className="flex flex-wrap items-center gap-2">
        <span
          className={
            "size-2 shrink-0 rounded-full " +
            (ready ? "bg-success" : "bg-live animate-breathe")
          }
        />
        <p className="text-xs font-medium text-ink" aria-live="polite" data-testid="login-status">
          {ready ? "● Signed in ✓ auto-detected" : `○ ${copy.headline}`}
        </p>
        <span className="text-[11px] text-muted">·</span>
        <span className="text-[11px] text-muted">{label}</span>
        {state === "api_key" && <Chip tone="accent">API key account</Chip>}
        <span className="flex-1" />
        {onDone && ready && (
          <Button size="sm" variant="primary" onClick={onDone}>
            Continue
          </Button>
        )}
      </div>

      <p className="text-[11px] leading-relaxed text-muted" data-testid="login-detail">
        {copy.detail}
        {loginNote ? ` ${loginNote}` : ""}
      </p>

      {!ready && state !== "api_key" && (
        <>
          <p className="text-[11px] leading-relaxed text-muted">
            This is {providerName}'s own sign-in. AI Juggler never sees your
            credentials. 1. Sign in → 2. follow the vendor prompts → 3. return;
            we detect it.
          </p>
          {!compact && (
            <div className="rounded-lg border border-hairline px-3 py-2 font-mono text-[11px] leading-relaxed text-success">
              {command || "…"}
            </div>
          )}
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="primary" onClick={() => void spawn()}>
              <IconTerminal width={14} height={14} />
              {copy.reopenLabel ?? "Sign in"}
            </Button>
            <Button size="sm" variant="secondary" onClick={() => void loginOpenWeb(providerName)}>
              Open vendor page
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
        <p className="text-[11px] text-danger" role="alert">
          {spawnError}
        </p>
      )}
    </div>
  );
}
