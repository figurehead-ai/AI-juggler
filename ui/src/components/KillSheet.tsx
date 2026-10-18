import { useEffect, useRef, useState } from "react";
import { Button } from "./primitives";
import { errorText } from "../lib/bridge";

/**
 * AJ-E507: the verify-before-kill sheet — the ONLY manual kill path
 * (pass2 §9.2/§9.3). Three steps, no one-click kill anywhere and no
 * "Don't ask again":
 *
 *   1. Verify (automatic on open): re-run the E503 identity check
 *      NOW, re-read exe/cmdline/cwd/ports, list the PGID collateral.
 *      Failed verification becomes the copy-exact explanation with
 *      the kill button REMOVED from the DOM — never
 *      disabled-with-tooltip (Req.2).
 *   2. Scope: this process only / process + helpers (PGID) / whole
 *      Instance tree — defaulting to the narrowest scope that covers
 *      the stated goal (`free :5173` ⇒ process-only).
 *   3. Confirm with a consequence-named button, then the visible
 *      grace countdown (cancellable, aria-live at 1 s), per-process
 *      exit ticks, and the honest outcome report (`already gone` is
 *      success; a still-held port is reported, never claimed freed).
 *
 * Keyboard: `Esc` cancels any step including mid-grace; `⌘⏎`
 * confirms only on the confirm step; focus returns to the
 * originating row on close (WCAG 2.4.3).
 */

export interface KillMemberInfo {
  pid: number;
  pgid: number | null;
  exe: string;
  cmdline: string;
  role_guess: string;
  unsaved_state_warning: string | null;
  from_ledger: boolean;
}

export interface KillVerifyData {
  /** AJ-E511: the machine that owns the target (`This Mac`
   * locally — every kill payload names its machine). */
  machine_id?: string;
  pid: number;
  session: string;
  verified: boolean;
  refusal: string | null;
  exe_path: string;
  cmdline: string;
  cwd: string;
  ports: number[];
  pgid: number | null;
  escaped_group: boolean;
  members: KillMemberInfo[];
  tree_pids: number[];
  platform: string;
  grace_secs: number;
  provider: string;
  account_id: string | null;
  project_id: string | null;
}

export interface StillHoldingInfo {
  port: number;
  pid: number;
  exe: string;
}

export interface KillMemberOutcomeInfo {
  pid: number;
  outcome: string;
}

export interface KillExecuteData {
  refusal: string | null;
  members: KillMemberOutcomeInfo[];
  cancelled: boolean;
  all_succeeded: boolean;
  still_holding: StillHoldingInfo[];
  reoffer_pid: number | null;
}

export type KillScope = "pid" | "pgid" | "tree";

export interface KillBridge {
  verify: (session: string, pid: number) => Promise<KillVerifyData>;
  execute: (
    session: string,
    pid: number,
    scope: KillScope,
    attempt: string,
    goalPort: number | null,
  ) => Promise<KillExecuteData>;
  cancel: (attempt: string) => Promise<void>;
  liveness: (pids: number[]) => Promise<boolean[]>;
}

// Dynamic import (the vi.mock TDZ rule).
export const defaultKillBridge: KillBridge = {
  verify: (session, pid) =>
    import("../lib/bridge").then((bridge) => bridge.killVerify(session, pid)),
  execute: (session, pid, scope, attempt, goalPort) =>
    import("../lib/bridge").then((bridge) =>
      bridge.killExecute(session, pid, scope, attempt, goalPort),
    ),
  cancel: (attempt) =>
    import("../lib/bridge").then((bridge) => bridge.killCancel(attempt)),
  liveness: (pids) =>
    import("../lib/bridge").then((bridge) => bridge.killLiveness(pids)),
};

export const KILL_SCOPES: { id: KillScope; label: string; hint: string }[] = [
  { id: "pid", label: "This process only", hint: "the narrowest scope" },
  { id: "pgid", label: "This process and its helpers", hint: "its whole process group" },
  { id: "tree", label: "Whole Instance tree", hint: "every live process of this Instance" },
];

/** Interpreters whose OWN name is never the user's name for the
 * process (`node …/vite dev` is "vite", not "node"). */
const INTERPRETER_NAMES = new Set([
  "node",
  "npm",
  "npx",
  "pnpm",
  "yarn",
  "bun",
  "bunx",
  "deno",
  "python",
  "python3",
  "ruby",
  "sh",
  "bash",
  "zsh",
  "fish",
  "dotnet",
  "java",
]);

/** The short process name for the consequence-named button (Req.1's
 * `Terminate vite (PID 8123) and free :5173`). */
export function killDisplayName(cmdline: string, exePath: string): string {
  const basename = (token: string) =>
    token.replaceAll("\\", "/").split("/").pop() ?? "";
  for (const token of cmdline.split(/\s+/)) {
    const base = basename(token);
    if (base !== "" && !INTERPRETER_NAMES.has(base.toLowerCase())) {
      return base;
    }
  }
  return basename(cmdline.split(/\s+/)[0] ?? "") || basename(exePath) || "process";
}

type Phase = "verifying" | "ready" | "refused" | "running" | "report";

export function KillSheet(props: {
  session: string;
  pid: number;
  goalPort?: number | null;
  /** The originating row's element — focus returns here on close. */
  trigger?: HTMLElement | null;
  onClose: (refresh: boolean) => void;
  bridge?: KillBridge;
}) {
  const bridge = props.bridge ?? defaultKillBridge;
  const goalPort = props.goalPort ?? null;
  const [phase, setPhase] = useState<Phase>("verifying");
  const [verify, setVerify] = useState<KillVerifyData | null>(null);
  const [report, setReport] = useState<KillExecuteData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [scope, setScope] = useState<KillScope>("pid");
  const [countdown, setCountdown] = useState(0);
  const [aliveMap, setAliveMap] = useState<Record<number, boolean>>({});
  const attemptRef = useRef<string>("");
  const confirmRef = useRef<HTMLButtonElement>(null);

  // Step 1 — verify NOW (fresh on open and on every re-offer target).
  useEffect(() => {
    let alive = true;
    setPhase("verifying");
    setVerify(null);
    setReport(null);
    setError(null);
    bridge
      .verify(props.session, props.pid)
      .then((result) => {
        if (!alive) return;
        setVerify(result);
        // The narrowest scope covering the stated goal: freeing a
        // port is a one-process job (pass2 §9.2 step 2).
        setScope("pid");
        setPhase(result.verified ? "ready" : "refused");
      })
      .catch((cause: unknown) => {
        if (alive) setError(errorText(cause));
      });
    return () => {
      alive = false;
    };
  }, [props.session, props.pid, bridge]);

  // The grace countdown: 1 s steps, announced politely (WCAG 2.2 —
  // not per-frame), while the ladder runs.
  useEffect(() => {
    if (phase !== "running" || verify === null) return;
    setCountdown(verify.grace_secs);
    const timer = setInterval(() => {
      setCountdown((left) => Math.max(0, left - 1));
    }, 1_000);
    return () => clearInterval(timer);
  }, [phase, verify]);

  // Per-process exit ticks: cheap liveness polls while the ladder
  // runs (the report stays the authority).
  useEffect(() => {
    if (phase !== "running" || verify === null) return;
    const memberPids =
      scope === "pid"
        ? [props.pid]
        : scope === "pgid"
          ? verify.members.map((member) => member.pid)
          : verify.tree_pids;
    let alive = true;
    const poll = () => {
      bridge
        .liveness(memberPids)
        .then((flags) => {
          if (!alive) return;
          const next: Record<number, boolean> = {};
          memberPids.forEach((pid, index) => {
            next[pid] = flags[index] ?? false;
          });
          setAliveMap(next);
        })
        .catch(() => {
          /* a failed tick degrades to the last known state */
        });
    };
    poll();
    const timer = setInterval(poll, 500);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [phase, verify, scope, props.pid, bridge]);

  // Esc cancels any step (incl. mid-grace); ⌘⏎ confirms ONLY the
  // confirm step. Focus lands INSIDE the sheet from mount (the root
  // container while verifying — otherwise a keyboard user cannot Esc
  // the loading step), then on the confirm button when ready.
  const rootRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    rootRef.current?.focus();
  }, []);
  useEffect(() => {
    if (phase === "ready") confirmRef.current?.focus();
  }, [phase]);

  const close = (refresh: boolean) => {
    props.onClose(refresh);
    if (props.trigger !== null && props.trigger !== undefined) {
      props.trigger.focus();
    }
  };

  const onKey = (event: React.KeyboardEvent) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      if (phase === "running") {
        void bridge.cancel(attemptRef.current).finally(() => close(false));
      } else {
        close(phase === "report");
      }
      return;
    }
    if (event.key === "Enter" && event.metaKey && phase === "ready") {
      event.preventDefault();
      event.stopPropagation();
      confirm();
    }
  };

  const confirm = () => {
    if (verify === null || phase !== "ready") return;
    const attempt = `kill-${props.pid}-${Date.now()}`;
    attemptRef.current = attempt;
    setPhase("running");
    setAliveMap({});
    bridge
      .execute(props.session, props.pid, scope, attempt, goalPort)
      .then((result) => {
        setReport(result);
        setPhase("report");
      })
      .catch((cause: unknown) => {
        setError(errorText(cause));
        setPhase("report");
      });
  };

  const cancelGrace = () => {
    void bridge.cancel(attemptRef.current);
    // The command resolves on its own with cancelled=true; the sheet
    // stays on the running phase (honest: TERM already went out)
    // until then.
  };

  const title =
    phase === "report"
      ? "Termination report"
      : phase === "refused"
        ? "Verification failed"
        : `Terminate PID ${props.pid}?`;

  return (
    <div
      ref={rootRef}
      tabIndex={-1}
      className="fixed inset-0 z-50 flex items-center justify-center bg-canvas/70 p-6 backdrop-blur-sm outline-none"
        onKeyDown={onKey}
        data-testid={`kill-sheet-${props.pid}`}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="sheen flex max-h-[80vh] w-full max-w-lg flex-col overflow-hidden rounded-card border border-hairline bg-surface shadow-[0_24px_80px_-24px_rgba(0,0,0,0.8)]"
      >
        <div className="flex items-start justify-between gap-4 border-b border-hairline p-4">
          <div>
            <h2 className="text-sm font-semibold">{title}</h2>
            <p className="mt-0.5 font-mono text-[10.5px] text-muted">
              Instance {props.session}
              {verify?.machine_id ? ` · ${verify.machine_id}` : ""}
              {verify !== null && verify.account_id !== null
                ? ` · ${verify.account_id}`
                : ""}
              {verify !== null && verify.project_id !== null
                ? ` · ${verify.project_id}`
                : ""}
            </p>
          </div>
          <button
            onClick={() => close(phase === "report")}
            className="rounded-lg p-1 text-faint transition-colors hover:bg-surface-2 hover:text-ink"
            aria-label="Close"
            data-testid={`kill-close-${props.pid}`}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-4 text-[11.5px] text-muted">
          {error !== null && (
            <p role="alert" className="mb-2 text-danger" data-testid={`kill-error-${props.pid}`}>
              {error}
            </p>
          )}

          {phase === "verifying" && (
            <p data-testid={`kill-verifying-${props.pid}`}>
              Re-verifying this process against the kernel now…
            </p>
          )}

          {phase === "refused" && verify !== null && (
            <div data-testid={`kill-refused-${props.pid}`}>
              <p className="rounded-lg border border-danger/40 bg-danger/5 p-2.5 text-ink">
                {verify.refusal}
              </p>
              <p className="mt-2">
                The verification facts, for the record — nothing was signalled:
              </p>
              <dl className="mt-1 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 font-mono text-[10.5px]">
                <dt className="text-faint">recorded exe</dt>
                <dd>{verify.exe_path}</dd>
                <dt className="text-faint">ports</dt>
                <dd>{verify.ports.length > 0 ? verify.ports.map((p) => `:${p}`).join(" ") : "—"}</dd>
              </dl>
            </div>
          )}

          {phase === "ready" && verify !== null && (
            <div data-testid={`kill-ready-${props.pid}`}>
              <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 font-mono text-[10.5px]">
                <dt className="text-faint">command</dt>
                <dd className="break-all text-ink" data-testid={`kill-cmdline-${props.pid}`}>
                  {verify.cmdline}
                </dd>
                <dt className="text-faint">cwd</dt>
                <dd className="break-all">{verify.cwd}</dd>
                <dt className="text-faint">ports now</dt>
                <dd>
                  {verify.ports.length > 0
                    ? verify.ports.map((p) => `:${p}`).join(" ")
                    : "none"}
                </dd>
                {goalPort !== null && (
                  <>
                    <dt className="text-faint">goal</dt>
                    <dd className="text-accent">free :{goalPort}</dd>
                  </>
                )}
              </dl>

              <fieldset className="mt-3">
                <legend className="text-[11px] font-medium text-ink">Scope</legend>
                <div className="mt-1 flex flex-col gap-1" role="radiogroup" aria-label="Kill scope">
                  {KILL_SCOPES.map((entry) => (
                    <label
                      key={entry.id}
                      className="flex cursor-pointer items-center gap-2 rounded-lg border border-hairline px-2 py-1.5"
                    >
                      <input
                        type="radio"
                        name={`kill-scope-${props.pid}`}
                        checked={scope === entry.id}
                        onChange={() => setScope(entry.id)}
                        data-testid={`kill-scope-${entry.id}-${props.pid}`}
                      />
                      <span className="text-ink">{entry.label}</span>
                      <span className="text-muted">— {entry.hint}</span>
                    </label>
                  ))}
                </div>
              </fieldset>

              {verify.members.length > 1 && (
                <div className="mt-3" data-testid={`kill-members-${props.pid}`}>
                  <p className="text-[11px] font-medium text-ink">
                    Shares the process group ({verify.members.length} live members)
                  </p>
                  <ul className="mt-1 flex flex-col gap-1">
                    {verify.members.map((member) => (
                      <li
                        key={member.pid}
                        className="flex items-center gap-2 rounded-lg border border-hairline px-2 py-1 font-mono text-[10px]"
                        data-testid={`kill-member-${member.pid}`}
                      >
                        <span className="text-ink">{member.exe}</span>
                        <span>PID {member.pid}</span>
                        <span className="text-faint">{member.role_guess} (guess)</span>
                        {!member.from_ledger && (
                          <span className="text-warning">not Juggler-managed</span>
                        )}
                      </li>
                    ))}
                  </ul>
                  {verify.members
                    .filter((member) => member.unsaved_state_warning !== null)
                    .map((member) => (
                      <p
                        key={member.pid}
                        className="mt-1 text-warning"
                        data-testid={`kill-unsaved-${member.pid}`}
                      >
                        {member.unsaved_state_warning}
                      </p>
                    ))}
                </div>
              )}

              {verify.platform === "TerminateProcess only" ? (
                <p className="mt-3 text-[11px]" data-testid={`kill-platform-${props.pid}`}>
                  Windows: TerminateProcess only — there is no grace window here.
                </p>
              ) : (
                <p className="mt-3 text-[11px]" data-testid={`kill-platform-${props.pid}`}>
                  Ladder: {verify.platform} — {verify.grace_secs}s of grace you can cancel, then
                  KILL only what ignored SIGTERM.
                </p>
              )}
            </div>
          )}

          {phase === "running" && verify !== null && (
            <div data-testid={`kill-running-${props.pid}`}>
              <p aria-live="polite" data-testid={`kill-countdown-${props.pid}`}>
                {countdown > 0
                  ? `Waiting for clean exit — ${countdown}s of grace left`
                  : "Escalating to KILL — survivors ignored SIGTERM"}
              </p>
              <ul className="mt-2 flex flex-col gap-1 font-mono text-[10.5px]">
                {(scope === "pid"
                  ? [props.pid]
                  : scope === "pgid"
                    ? verify.members.map((member) => member.pid)
                    : verify.tree_pids
                ).map((pid) => (
                  <li key={pid} className="flex items-center gap-2" data-testid={`kill-tick-${pid}`}>
                    <span aria-hidden="true">{aliveMap[pid] === false ? "✓" : "…"}</span>
                    <span>PID {pid}</span>
                    <span>{aliveMap[pid] === false ? "exited" : "running"}</span>
                  </li>
                ))}
              </ul>
              <Button size="sm" variant="secondary" className="mt-3" onClick={cancelGrace}>
                Cancel — stop the escalation
              </Button>
            </div>
          )}

          {phase === "report" && report !== null && (
            <div data-testid={`kill-report-${props.pid}`}>
              {report.refusal !== null && (
                <p className="rounded-lg border border-danger/40 bg-danger/5 p-2.5 text-ink">
                  {report.refusal}
                </p>
              )}
              {report.cancelled && (
                <p className="text-ink">Cancelled during grace — TERM was sent; outcomes unknown.</p>
              )}
              <ul className="mt-2 flex flex-col gap-1 font-mono text-[10.5px]">
                {report.members.map((member) => (
                  <li key={member.pid} data-testid={`kill-outcome-${member.pid}`}>
                    PID {member.pid} — {member.outcome}
                  </li>
                ))}
              </ul>
              {report.still_holding.length > 0 && (
                <div className="mt-2" data-testid={`kill-still-holding-${props.pid}`}>
                  <p className="text-warning">
                    Still holding (never claimed freed):
                  </p>
                  <ul className="font-mono text-[10.5px]">
                    {report.still_holding.map((held) => (
                      <li key={`${held.pid}-${held.port}`}>
                        :{held.port} by {held.exe} (PID {held.pid})
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              {!report.all_succeeded && report.refusal === null && !report.cancelled && (
                <p className="mt-1 text-danger">A process refused even SIGKILL — it is still running.</p>
              )}
              {report.all_succeeded && report.still_holding.length === 0 && (
                <p className="mt-1 text-ink" data-testid={`kill-success-${props.pid}`}>
                  {goalPort !== null ? `:${goalPort} is free.` : "Done — every process is gone."}
                </p>
              )}
              {report.reoffer_pid !== null && report.reoffer_pid !== props.pid && (
                <p className="mt-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      // The pid-only re-offer: FRESH verification of
                      // the escapee (never an automatic signal).
                      setPhase("verifying");
                      setVerify(null);
                      setReport(null);
                      bridge
                        .verify(props.session, report.reoffer_pid as number)
                        .then((next) => {
                          setVerify(next);
                          setScope("pid");
                          setPhase(next.verified ? "ready" : "refused");
                        })
                        .catch((cause: unknown) => setError(errorText(cause)));
                    }}
                    data-testid={`kill-reoffer-${props.pid}`}
                  >
                    Re-verify PID {report.reoffer_pid} — it still holds the port
                  </Button>
                </p>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-hairline p-3">
          {phase !== "running" && (
            <Button variant="secondary" onClick={() => close(phase === "report")}>
              {phase === "report" ? "Done" : "Cancel"}
            </Button>
          )}
          {/* Req.1/Req.2: the confirm exists ONLY behind a passed
              verification — the refused state renders no button at
              all (not disabled-with-tooltip). */}
          {phase === "ready" && verify !== null && (
            <button
              type="button"
              ref={confirmRef}
              onClick={confirm}
              className="rounded-xl border border-danger bg-danger px-3 py-1.5 text-[11.5px] font-medium text-canvas transition-opacity hover:opacity-90 outline-none focus-visible:ring-1 focus-visible:ring-danger focus-visible:ring-offset-1 focus-visible:ring-offset-surface"
              data-testid={`kill-confirm-${props.pid}`}
            >
              Terminate {killDisplayName(verify.cmdline, verify.exe_path)} (PID {props.pid})
              {goalPort !== null ? ` and free :${goalPort}` : ""}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
