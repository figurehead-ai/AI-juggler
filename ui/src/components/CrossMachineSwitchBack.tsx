/**
 * AJ-E706 Req.4 / AJ-E708 Req.4–5 — switching an away Instance back to
 * the machine its work came from, and the conflict sheet when both sides
 * moved on.
 *
 * D-05 across machines: the original native session resumes there with
 * exactly ONE "While you were away" turn — previewed here verbatim before
 * the click — and is never overwritten. When the original's machine
 * reports that it moved on too, nothing runs: the conflict sheet shows
 * both sides side by side (counts and times, plain words) and offers
 * Keep mine / Keep theirs / Keep both (fork), each described. Default
 * focus is "Review later" (non-destructive); no auto-dismiss. The choices
 * are a radio group with descriptions — completable keyboard-only.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import {
  errorText,
  fleetDivergenceKeepTheirs,
  fleetSwitchBackExecute,
  type DivergenceInfo,
  type FleetSwitchBackPlanInfo,
} from "../lib/bridge";
import { Checkbox, Radio } from "./controls";

type Choice = "keep_mine" | "keep_theirs" | "keep_both";

type Phase =
  | { kind: "plan" }
  | { kind: "running" }
  | { kind: "diverged"; divergence: DivergenceInfo }
  | { kind: "done"; message: string }
  | { kind: "failed"; message: string };

function when(ms: number | null): string {
  return ms === null ? "time unknown" : new Date(ms).toLocaleString();
}

export function CrossMachineSwitchBack({
  sessionId,
  plan,
  divergence,
  onClose,
  onResolved,
}: {
  sessionId: string;
  plan: FleetSwitchBackPlanInfo;
  /** Open straight on the conflict sheet (a divergence already known). */
  divergence?: DivergenceInfo | null;
  onClose: () => void;
  onResolved?: () => void;
}): JSX.Element {
  const [phase, setPhase] = useState<Phase>(
    divergence ? { kind: "diverged", divergence } : { kind: "plan" },
  );
  const [keepAway, setKeepAway] = useState(false);
  const [choice, setChoice] = useState<Choice | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const laterRef = useRef<HTMLButtonElement | null>(null);
  const machine = plan.original_machine_label;

  useEffect(() => {
    if (phase.kind === "diverged") laterRef.current?.focus();
    else dialogRef.current?.querySelector<HTMLElement>("button, input")?.focus();
  }, [phase.kind]);

  useEffect(() => {
    const node = dialogRef.current;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && phase.kind !== "running") {
        event.preventDefault();
        onClose();
      }
      if (event.key === "Tab" && node) {
        const focusables = Array.from(node.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled)"));
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
  }, [onClose, phase.kind]);

  const run = async (pick: "" | "keep_mine" | "keep_both") => {
    setPhase({ kind: "running" });
    try {
      const outcome = await fleetSwitchBackExecute(sessionId, pick, keepAway);
      if (outcome.accepted) {
        setPhase({ kind: "done", message: outcome.message });
        onResolved?.();
      } else if (outcome.reason_code === "diverged" && outcome.divergence) {
        setChoice(null);
        setPhase({ kind: "diverged", divergence: outcome.divergence });
      } else {
        setPhase({ kind: "failed", message: outcome.message });
      }
    } catch (cause) {
      setPhase({ kind: "failed", message: errorText(cause) });
    }
  };

  const apply = async () => {
    if (choice === "keep_theirs") {
      try {
        await fleetDivergenceKeepTheirs(sessionId);
        setPhase({
          kind: "done",
          message: `Kept as they are — the work stays here, ${machine}'s original continues there. Nothing was sent.`,
        });
        onResolved?.();
      } catch (cause) {
        setPhase({ kind: "failed", message: errorText(cause) });
      }
      return;
    }
    if (choice) void run(choice);
  };

  const options: Array<{ id: Choice; title: string; body: string }> =
    phase.kind === "diverged"
      ? [
          {
            id: "keep_mine",
            title: "Keep mine",
            body: `Resume the original on ${machine} with this work's summary — its ${phase.divergence.turns_since} newer ${
              phase.divergence.turns_since === 1 ? "turn stays" : "turns stay"
            }, the summary follows them.`,
          },
          {
            id: "keep_theirs",
            title: "Keep theirs",
            body: `Leave both as they are: ${machine}'s original continues there, this work stays here. Nothing is sent.`,
          },
          {
            id: "keep_both",
            title: "Keep both (fork)",
            body: `Start a copy of the original on ${machine} with this work's summary; the original's own continuation stays untouched.`,
          },
        ]
      : [];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="cross-back-title"
        data-testid="cross-switch-back"
        className="flex max-h-[90vh] w-full max-w-xl flex-col gap-3 overflow-y-auto rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        {phase.kind === "diverged" ? (
          <>
            <h3 id="cross-back-title" className="text-sm font-semibold">
              <span aria-hidden="true">⚠ </span>Diverged — both sides moved on
            </h3>
            <p className="text-label text-ink">
              Nothing was merged and nothing ran. Choose what to keep; switching back and cleaning up
              this session wait until you do.
            </p>
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2" data-testid="conflict-sides">
              <section className="rounded-lg border border-hairline p-2.5 text-label" aria-label="Here">
                <p className="font-medium text-ink">Here — {plan.away_account_label}</p>
                <p className="text-ink">
                  {plan.new_turns} new {plan.new_turns === 1 ? "turn" : "turns"} since the work arrived
                </p>
              </section>
              <section className="rounded-lg border border-hairline p-2.5 text-label" aria-label={`On ${machine}`}>
                <p className="font-medium text-ink">
                  On {machine} — {plan.original_account_label}
                </p>
                <p className="text-ink" data-testid="conflict-theirs">
                  {phase.divergence.turns_since} new {phase.divergence.turns_since === 1 ? "turn" : "turns"} in the
                  original since the work left · last active {when(phase.divergence.last_activity_ms)}
                </p>
              </section>
            </div>
            <div role="radiogroup" aria-labelledby="cross-back-title" className="flex flex-col gap-1.5" data-testid="conflict-choices">
              {options.map((option) => (
                <label
                  key={option.id}
                  className={
                    "flex items-start gap-2 rounded-lg border px-2.5 py-2 text-label " +
                    (choice === option.id ? "border-accent/60 bg-accent-soft" : "border-hairline")
                  }
                >
                  <Radio
                   
                    name="conflict-choice"
                    checked={choice === option.id}
                    onChange={() => setChoice(option.id)}
                    aria-describedby={`conflict-${option.id}-desc`}
                    data-testid={`conflict-${option.id}`}
                  />
                  <span>
                    <span className="block font-medium text-ink">{option.title}</span>
                    <span id={`conflict-${option.id}-desc`} className="text-muted">
                      {option.body}
                    </span>
                  </span>
                </label>
              ))}
            </div>
            <div className="flex items-center justify-end gap-2">
              <button
                ref={laterRef}
                type="button"
                onClick={onClose}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
                data-testid="conflict-later"
              >
                Review later
              </button>
              <button
                type="button"
                disabled={choice === null}
                onClick={() => void apply()}
                className={
                  "rounded-lg border px-3 py-1.5 text-xs " +
                  (choice ? "border-accent/40 bg-accent-soft text-accent-hi" : "border-hairline text-muted")
                }
                data-testid="conflict-apply"
              >
                {choice === "keep_theirs" ? "Keep both as they are" : choice ? `Apply on ${machine}` : "Choose one"}
              </button>
            </div>
          </>
        ) : (
          <>
            <h3 id="cross-back-title" className="text-sm font-semibold">
              Switch back to {plan.original_account_label} on {machine}
            </h3>
            {phase.kind === "plan" && (
              <>
                <p className="text-label text-ink">
                  The original conversation resumes on {machine} — never overwritten — with this one update
                  ({plan.new_turns} new {plan.new_turns === 1 ? "turn" : "turns"} from here):
                </p>
                <pre
                  className="max-h-48 overflow-y-auto whitespace-pre-wrap rounded-lg border border-hairline bg-canvas p-2 text-label text-ink"
                  data-testid="cross-back-delta"
                  tabIndex={0}
                  aria-label="The update the original receives"
                >
                  {plan.delta_preview}
                </pre>
                {plan.blockers.length > 0 && (
                  <ul className="flex flex-col gap-1" data-testid="cross-back-blockers">
                    {plan.blockers.map((reason) => (
                      <li key={reason} className="rounded border border-warning/40 px-2 py-1 text-label text-ink">
                        <span aria-hidden="true">⚠ </span>
                        {reason}
                      </li>
                    ))}
                  </ul>
                )}
                <label className="flex items-center gap-2 text-label text-ink">
                  <Checkbox checked={keepAway} onChange={(event) => setKeepAway(event.target.checked)} />
                  Keep this session running too
                </label>
                <div className="flex justify-end gap-2">
                  <button
                    type="button"
                    onClick={onClose}
                    className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    disabled={plan.blockers.length > 0}
                    onClick={() => void run("")}
                    className={
                      "rounded-lg border px-3 py-1.5 text-xs " +
                      (plan.blockers.length === 0
                        ? "border-accent/40 bg-accent-soft text-accent-hi"
                        : "border-hairline text-muted")
                    }
                    data-testid="cross-back-go"
                  >
                    Switch back to {machine}…
                  </button>
                </div>
              </>
            )}
            {phase.kind === "running" && (
              <p className="text-label text-ink" aria-live="polite" data-testid="cross-back-running">
                Asking {machine} to resume the original…
              </p>
            )}
            {phase.kind === "done" && (
              <div className="flex flex-col gap-2" aria-live="polite" data-testid="cross-back-done">
                <p className="text-label text-ink">
                  <span aria-hidden="true">✓ </span>
                  {phase.message}
                </p>
                <div className="flex justify-end">
                  <button type="button" onClick={onClose} className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink">
                    Close
                  </button>
                </div>
              </div>
            )}
            {phase.kind === "failed" && (
              <div className="flex flex-col gap-2" role="alert" data-testid="cross-back-failed">
                <p className="text-label text-ink">
                  <span aria-hidden="true">✗ </span>
                  {phase.message}
                </p>
                <div className="flex justify-end">
                  <button type="button" onClick={onClose} className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink">
                    Close
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
