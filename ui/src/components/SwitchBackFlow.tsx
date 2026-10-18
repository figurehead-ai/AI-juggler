/**
 * AJ-E207 — the switch-back surface: one suggested card's review
 * (pass2/02 §6.1-6.2, D-05). The return RESUMES the original native
 * session and delivers the away work as exactly ONE "While you were
 * away" delta turn — never a copy-back. Both-sides-progressed
 * conflicts render as per-file radio trios (Keep original / Take
 * away / Keep both — away saved as an `…away-<date>` sibling);
 * nothing is merged silently, and the return cannot execute until
 * every conflict has a choice (S-F8 keeps the work forked). The
 * away instance gets the freeze-or-fork choice when still live
 * (resuming while it runs is legal — and said out loud). Focus is
 * trapped; Esc cancels; ⏎ confirms when armed; conflicts are
 * keyboard-completable end to end.
 */
import { useEffect, useRef, useState } from "react";
import {
  switchBackPlan,
  switchBackExecute,
  type SwitchBackPlanData, errorText,} from "../lib/bridge";

type Phase =
  | { kind: "plan" }
  | { kind: "review"; plan: SwitchBackPlanData }
  | { kind: "executing" }
  | { kind: "done"; sessionId: string }
  | { kind: "failed"; message: string };

const STAGES = ["Stop or keep away", "Apply choices", "Resume original", "Record"] as const;

const CHOICES = [
  { value: "keep-original", label: "Keep original" },
  { value: "take-away", label: "Take the away version" },
  { value: "keep-both", label: "Keep both (away saved as a dated sibling)" },
] as const;

export function SwitchBackFlow({
  sessionId,
  onClose,
  onSwitched,
}: {
  sessionId: string;
  onClose: () => void;
  onSwitched: (sessionId: string) => void;
}): JSX.Element {
  const [phase, setPhase] = useState<Phase>({ kind: "plan" });
  const [choices, setChoices] = useState<Record<string, string>>({});
  const [keepAwayRunning, setKeepAwayRunning] = useState(false);
  const dialogRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let alive = true;
    setChoices({});
    setKeepAwayRunning(false);
    switchBackPlan(sessionId)
      .then((plan) => {
        if (alive) setPhase({ kind: "review", plan });
      })
      .catch((cause) => {
        if (alive) setPhase({ kind: "failed", message: errorText(cause) });
      });
    return () => {
      alive = false;
    };
  }, [sessionId]);

  const plan = phase.kind === "review" ? phase.plan : null;
  const unresolved = plan
    ? plan.conflicts.filter((conflict) => !choices[conflict.rel_path]).length
    : 0;
  const armed = phase.kind === "review" && unresolved === 0;

  // Focus trap + Esc/⏎ (2.1.2/2.1.1 — the dialog is the only
  // operable surface while open).
  useEffect(() => {
    const node = dialogRef.current;
    node?.querySelector<HTMLElement>("button, input")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key === "Enter" && armed) {
        event.preventDefault();
        void execute();
      }
      if (event.key === "Tab" && node) {
        const focusables = Array.from(
          node.querySelectorAll<HTMLElement>("button, input"),
        );
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase.kind, armed, onClose]);

  const execute = async () => {
    if (phase.kind !== "review") return;
    setPhase({ kind: "executing" });
    try {
      const result = await switchBackExecute(
        sessionId,
        Object.entries(choices).map(([rel_path, choice]) => ({ rel_path, choice })),
        keepAwayRunning,
      );
      setPhase({ kind: "done", sessionId: result.session_id });
    } catch (cause) {
      setPhase({ kind: "failed", message: errorText(cause) });
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label="Switch back"
        data-testid="switch-back-flow"
        className="flex max-h-[86vh] w-full max-w-md flex-col overflow-y-auto rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        {phase.kind === "plan" && (
          <p className="text-xs text-muted" data-testid="switch-back-planning">
            Reading the switch lineage (plan)…
          </p>
        )}

        {phase.kind === "review" && plan && (
          <div className="flex flex-col gap-3" data-testid="switch-back-review">
            <h3 className="text-sm font-semibold">
              Switch back to {plan.original_account}
              <span className="ml-2 rounded border border-accent/50 px-1 text-[10px] text-accent">
                {plan.original_alive ? "resume + one delta turn" : "fresh start · F0"}
              </span>
            </h3>
            {/* Req.3 verbatim — suggested, never executed. */}
            <p className="text-[11px]" data-testid="switch-back-suggestion">
              {plan.suggestion}
            </p>
            {!plan.original_alive && (
              <p className="rounded border border-warning/40 px-2 py-1 text-[11px] text-warning" data-testid="switch-back-sf7">
                {plan.sf7_copy}
                {plan.sf7_fallback.startsWith("nearest-epoch") &&
                  " — the nearest surviving conversation resumes instead."}
                {plan.sf7_fallback === "fresh-f0" &&
                  " — a fresh instance starts, pack-primed honestly at F0."}
              </p>
            )}
            <p className="text-[11px] text-muted" data-testid="switch-back-divergence">
              {plan.divergence}
            </p>
            <div
              className="rounded border border-hairline bg-surface-2 px-2 py-1.5 text-[11px] whitespace-pre-wrap"
              data-testid="switch-back-delta"
            >
              {plan.delta_preview}
            </div>

            {plan.conflicts.length > 0 && (
              <div className="flex flex-col gap-2" data-testid="switch-back-conflicts">
                <p className="text-[11px] uppercase tracking-wider text-muted">
                  Conflicts — nothing merges silently
                </p>
                {plan.conflicts.map((conflict) => (
                  <fieldset key={conflict.rel_path} className="flex flex-col gap-1 rounded border border-hairline px-2 py-1.5">
                    <legend className="text-[11px] font-medium">{conflict.rel_path}</legend>
                    {CHOICES.map((option) => (
                      <label key={option.value} className="flex items-center gap-2 text-xs">
                        <input
                          type="radio"
                          name={`conflict-${conflict.rel_path}`}
                          checked={choices[conflict.rel_path] === option.value}
                          onChange={() =>
                            setChoices((current) => ({
                              ...current,
                              [conflict.rel_path]: option.value,
                            }))
                          }
                        />
                        {option.label}
                      </label>
                    ))}
                  </fieldset>
                ))}
              </div>
            )}

            {plan.away_live && (
              <fieldset className="flex flex-col gap-1.5" data-testid="switch-back-freeze-or-fork">
                <legend className="text-[11px] uppercase tracking-wider text-muted">
                  The away instance is still running
                </legend>
                <label className="flex items-center gap-2 text-xs">
                  <input
                    type="radio"
                    name="away"
                    checked={!keepAwayRunning}
                    onChange={() => setKeepAwayRunning(false)}
                  />
                  Stop it, then return <span className="text-muted">(default)</span>
                </label>
                <label className="flex items-center gap-2 text-xs">
                  <input
                    type="radio"
                    name="away"
                    checked={keepAwayRunning}
                    onChange={() => setKeepAwayRunning(true)}
                  />
                  Keep it running
                </label>
                {keepAwayRunning && (
                  <p className="rounded border border-warning/40 px-2 py-1 text-[11px] text-warning" data-testid="switch-back-both-live">
                    {plan.both_live_copy}
                  </p>
                )}
              </fieldset>
            )}

            <div className="flex items-center justify-between gap-2">
              <p className="text-[11px] text-muted" data-testid="switch-back-unresolved">
                {unresolved > 0
                  ? `Choose for ${unresolved} conflict${unresolved === 1 ? "" : "s"} to continue — the work stays forked until you do.`
                  : "One click returns the work — the original conversation resumes."}
              </p>
            </div>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={onClose}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs hover:bg-surface-2"
              >
                Cancel <span className="text-muted">(Esc)</span>
              </button>
              <button
                type="button"
                disabled={unresolved > 0}
                onClick={() => void execute()}
                data-testid="switch-back-confirm"
                className={
                  "rounded-lg border px-3 py-1.5 text-xs " +
                  (unresolved > 0
                    ? "border-hairline text-muted"
                    : "border-accent/40 bg-accent-soft text-accent-hi hover:border-accent/60")
                }
              >
                Resume {plan.original_account} ▸ {armed && <span className="text-muted">(⏎)</span>}
              </button>
            </div>
          </div>
        )}

        {phase.kind === "executing" && (
          <div className="flex flex-col gap-2" aria-live="polite" data-testid="switch-back-executing">
            <h3 className="text-sm font-semibold">Returning to the original conversation…</h3>
            <ol className="flex flex-col gap-1">
              {STAGES.map((stage) => (
                <li key={stage} className="text-[11px] text-muted">
                  · {stage}
                </li>
              ))}
            </ol>
            <p className="text-[10px] text-muted">
              The original transcript is never written — the away work arrives as one update turn.
            </p>
          </div>
        )}

        {phase.kind === "done" && (
          <div className="flex flex-col gap-3" data-testid="switch-back-done">
            <h3 className="text-sm font-semibold">You returned to the original conversation</h3>
            <p className="text-[11px] text-muted">
              It resumed exactly where it rested — the away work arrived as one
              “while you were away” update. {phase.sessionId}
            </p>
            <div className="flex justify-end">
              <button
                type="button"
                onClick={() => onSwitched(phase.sessionId)}
                className="rounded-lg border border-accent/40 bg-accent-soft px-3 py-1.5 text-xs text-accent-hi"
              >
                Go to the instance
              </button>
            </div>
          </div>
        )}

        {phase.kind === "failed" && (
          <div className="flex flex-col gap-3" data-testid="switch-back-failed">
            <h3 className="text-sm font-semibold text-warning">Return didn’t complete</h3>
            <p className="rounded border border-warning/40 px-2 py-1 text-[11px] text-warning">
              {phase.message}
            </p>
            <p className="text-[11px] text-muted">
              Nothing was merged silently — both sides stay resumable exactly as they are.
            </p>
            <div className="flex justify-end">
              <button
                type="button"
                onClick={onClose}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs hover:bg-surface-2"
              >
                Close
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
