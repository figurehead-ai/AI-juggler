/**
 * AJ-E205/E206 — the switch execution surface. The door (pass2/02
 * §8) picks the shape at preflight: a proven pair renders the
 * compact 1-click popover (carry one-liner + cost + the §5.2 freeze
 * choice — exactly Finish-turn or Stop-now, never "copy live and
 * hope"), everything else opens the wizard, which renders ONLY the
 * steps whose data is non-trivial ("Step 2 of 3", never phantom
 * steps). No handover executes from a preview the user has not seen:
 * the confirm click lives on the wizard's last step. Executing shows
 * determinate labelled stages (never a bare spinner); failure shows
 * the §7.1 source-safe banner; a completed 1-click switch offers a
 * 10-second undo toast (stop target + resume source + compensating
 * edge). Copy never says "we switched you" (SPEC:26) — the user
 * clicked, the user switched.
 */
import { useEffect, useRef, useState } from "react";
import {
  switchPreflight,
  switchExecute,
  switchUndo,
  redactionPolicySetGlobal,
  type PreflightInfoData, errorText,} from "../lib/bridge";

/** The plain-words picker options (same words as Settings). */
const REDACTION_OPTIONS = [
  { value: "strip", label: "Strip — replace keys with •••" },
  { value: "keep", label: "Keep — pack stays on this Mac" },
  { value: "ask", label: "Ask each time" },
] as const;

type Phase =
  | { kind: "preflight" }
  | { kind: "confirm"; preflight: PreflightInfoData }
  | { kind: "executing"; startedAt: number }
  | { kind: "done"; sessionId: string }
  | { kind: "failed"; message: string };

const STAGES = [
  "Freeze",
  "Snapshot",
  "Stage & verify",
  "Launch",
  "Record",
] as const;

/** The carry-list one-liner for the 1-click popover (AJ-E206 §8.2). */
function carryOneLiner(carry: PreflightInfoData["carry"]): string {
  const parts = [
    `${carry.carried.length} carried`,
    `${carry.regenerated.length} rebuilt`,
  ];
  // §8: 1-click requires zero drops — say so in words, never "some
  // context".
  parts.push(carry.dropped.length === 0 ? "nothing dropped" : `${carry.dropped.length} dropped`);
  return parts.join(" · ");
}

/** AJ-E208: the redaction surface both confirm doors render — the
 * verbatim policy line, the first-run chooser (required, no
 * default — it blocks the first pack until the user chooses), the
 * per-switch Ask answer, and the per-item un-redact list (this
 * switch only — never a remembered-policy change). */
function RedactionBlock({
  preflight,
  askChoice,
  onAskChoice,
  chooserPick,
  onChooserPick,
  onGlobalChosen,
  showItems,
  unredact,
  onToggleItem,
}: {
  preflight: PreflightInfoData;
  askChoice: "strip" | "keep" | null;
  onAskChoice: (choice: "strip" | "keep") => void;
  chooserPick: string | null;
  onChooserPick: (pick: string) => void;
  onGlobalChosen: (pick: string) => void;
  showItems: boolean;
  unredact: Set<number>;
  onToggleItem: (index: number) => void;
}): JSX.Element {
  const { redaction, redaction_items: items } = preflight;
  if (redaction.effective === null) {
    return (
      <fieldset className="flex flex-col gap-1.5 rounded border border-warning/40 px-2 py-1.5" data-testid="switch-redaction-chooser">
        <legend className="text-[11px] uppercase tracking-wider text-warning">
          Choose how secrets are handled in packs (required, once)
        </legend>
        {REDACTION_OPTIONS.map((option) => (
          <label key={option.value} className="flex items-center gap-2 text-xs">
            <input
              type="radio"
              name="redaction-global"
              checked={chooserPick === option.value}
              onChange={() => onChooserPick(option.value)}
            />
            {option.label}
          </label>
        ))}
        <button
          type="button"
          disabled={chooserPick === null}
          onClick={() => chooserPick !== null && onGlobalChosen(chooserPick)}
          data-testid="switch-redaction-save"
          className={
            "self-start rounded-lg border px-3 py-1.5 text-xs " +
            (chooserPick === null
              ? "border-hairline text-muted"
              : "border-accent/40 bg-accent-soft text-accent-hi")
          }
        >
          Save my choice
        </button>
      </fieldset>
    );
  }
  return (
    <div className="flex flex-col gap-1" data-testid="switch-redaction">
      <p className="text-[11px] text-muted" data-testid="switch-redaction-line">
        {redaction.line}
      </p>
      {redaction.effective === "ask" && (
        <fieldset className="flex flex-col gap-1 rounded border border-hairline px-2 py-1.5">
          <legend className="text-[11px] uppercase tracking-wider text-muted">
            This switch
          </legend>
          <label className="flex items-center gap-2 text-xs">
            <input
              type="radio"
              name="redaction-ask"
              checked={askChoice === "strip"}
              onChange={() => onAskChoice("strip")}
            />
            Strip secrets this time
          </label>
          <label className="flex items-center gap-2 text-xs">
            <input
              type="radio"
              name="redaction-ask"
              checked={askChoice === "keep"}
              onChange={() => onAskChoice("keep")}
            />
            Keep secrets this time (pack stays on this Mac)
          </label>
        </fieldset>
      )}
      {showItems && items.length > 0 && (
        <div className="flex flex-col gap-1 rounded border border-hairline px-2 py-1.5" data-testid="switch-redaction-items">
          <p className="text-[11px] uppercase tracking-wider text-muted">
            Secrets detected — un-redact per item for this switch only
          </p>
          {items.map((item, index) => (
            <label key={`${item.kind}-${index}`} className="flex items-center gap-2 text-xs">
              <input
                type="checkbox"
                checked={unredact.has(index)}
                onChange={() => onToggleItem(index)}
              />
              <span className="font-mono text-[11px]">{item.preview}</span>
              <span className="text-muted">{item.kind}</span>
            </label>
          ))}
        </div>
      )}
      {items.length === 0 && (
        <p className="text-[10px] text-muted" data-testid="switch-redaction-none">
          No secrets detected in this conversation.
        </p>
      )}
    </div>
  );
}

export function SwitchFlow({
  sessionId,
  targetProfileId,
  targetLabel,
  onClose,
  onSwitched,
}: {
  sessionId: string;
  targetProfileId: string;
  targetLabel: string;
  onClose: () => void;
  onSwitched: (sessionId: string, undo: { edgeId: string } | null) => void;
}): JSX.Element {
  const [phase, setPhase] = useState<Phase>({ kind: "preflight" });
  const [freeze, setFreeze] = useState<"finish-turn" | "stop-now">("finish-turn");
  const [step, setStep] = useState(0);
  // AJ-E208: the redaction will for this switch (Ask answer, the
  // first-run chooser pick, and the per-item un-redact set).
  const [askChoice, setAskChoice] = useState<"strip" | "keep" | null>(null);
  const [chooserPick, setChooserPick] = useState<string | null>(null);
  const [unredact, setUnredact] = useState<Set<number>>(new Set());
  const dialogRef = useRef<HTMLDivElement | null>(null);

  // Reset the redaction state when the flow opens for a new target.
  useEffect(() => {
    setAskChoice(null);
    setChooserPick(null);
    setUnredact(new Set());
  }, [sessionId, targetProfileId]);

  useEffect(() => {
    let alive = true;
    switchPreflight(sessionId, targetProfileId)
      .then((preflight) => {
        if (alive) {
          setStep(0);
          setPhase({ kind: "confirm", preflight });
        }
      })
      .catch((cause) => {
        if (alive) setPhase({ kind: "failed", message: errorText(cause) });
      });
    return () => {
      alive = false;
    };
  }, [sessionId, targetProfileId]);

  const steps =
    phase.kind === "confirm" && phase.preflight.door === "wizard"
      ? phase.preflight.wizard_steps
      : [];
  // The execute click lives on the 1-click popover or the wizard's
  // LAST step only — never mid-wizard (deep/09 H-5).
  const redactionRequired =
    phase.kind === "confirm" &&
    (phase.preflight.redaction.effective === null
      ? chooserPick === null
      : phase.preflight.redaction.effective === "ask" && askChoice === null);
  const executeArmed =
    phase.kind === "confirm" &&
    (phase.preflight.door === "one-click" || step === steps.length - 1) &&
    !redactionRequired;

  // Focus trap + Esc/⏎ handling (2.1.2/2.1.1 — the dialog is the
  // only operable surface while open).
  useEffect(() => {
    const node = dialogRef.current;
    node?.querySelector<HTMLElement>("button, input")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key === "Enter" && executeArmed) {
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
  }, [phase.kind, step, executeArmed, onClose]);

  const toggleUnredact = (index: number) => {
    setUnredact((current) => {
      const next = new Set(current);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  };

  const chooseGlobal = async (pick: string) => {
    try {
      await redactionPolicySetGlobal(pick);
      // Refresh the plan — the backend rebuilds the verbatim line
      // (wizard state survives: step + freeze are untouched).
      const refreshed = await switchPreflight(sessionId, targetProfileId);
      setPhase({ kind: "confirm", preflight: refreshed });
    } catch {
      // The chooser stays up; nothing is guessed on failure.
    }
  };

  const execute = async () => {
    if (phase.kind !== "confirm") return;
    const oneClick = phase.preflight.door === "one-click";
    setPhase({ kind: "executing", startedAt: Date.now() });
    try {
      const result = await switchExecute(
        sessionId,
        targetProfileId,
        freeze,
        askChoice,
        [...unredact],
      );
      if (oneClick) {
        // The modal closes into the undo window (App renders the
        // toast); the graph keeps the edge as the undo handle.
        onSwitched(result.session_id, { edgeId: result.edge_id });
        return;
      }
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
        aria-label={`Switch to ${targetLabel}`}
        data-testid="switch-flow"
        className="w-full max-w-md rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        {phase.kind === "preflight" && (
          <p className="text-xs text-muted" data-testid="switch-preflighting">
            Checking the switch (preflight)…
          </p>
        )}

        {phase.kind === "confirm" && phase.preflight.door === "one-click" && (
          <div className="flex flex-col gap-3" data-testid="switch-oneclick">
            <h3 className="text-sm font-semibold">
              Switch to {targetLabel}
              <span className="ml-2 rounded border border-accent/50 px-1 text-[10px] text-accent">
                {phase.preflight.fidelity} · usual account · beta
              </span>
            </h3>
            <p className="text-[11px] text-muted" data-testid="switch-oneclick-carry">
              {carryOneLiner(phase.preflight.carry)} — cost: one switch hop.
            </p>
            <p className="text-[11px] text-muted">{phase.preflight.experimental_copy}</p>
            {phase.preflight.warnings.length > 0 && (
              <ul className="flex flex-col gap-1" data-testid="switch-warnings">
                {phase.preflight.warnings.map((warning) => (
                  <li key={warning} className="rounded border border-warning/40 px-2 py-1 text-[11px] text-warning">
                    {warning}
                  </li>
                ))}
              </ul>
            )}
            <RedactionBlock
              preflight={phase.preflight}
              askChoice={askChoice}
              onAskChoice={setAskChoice}
              chooserPick={chooserPick}
              onChooserPick={setChooserPick}
              onGlobalChosen={(pick) => void chooseGlobal(pick)}
              showItems={false}
              unredact={unredact}
              onToggleItem={toggleUnredact}
            />
            <fieldset className="flex flex-col gap-1.5">
              <legend className="text-[11px] uppercase tracking-wider text-muted">
                {phase.preflight.source_live ? "Freeze the source first" : "The source is at rest"}
              </legend>
              <label className="flex items-center gap-2 text-xs">
                <input
                  type="radio"
                  name="freeze"
                  checked={freeze === "finish-turn"}
                  onChange={() => setFreeze("finish-turn")}
                />
                Finish this turn, then switch <span className="text-muted">(default)</span>
              </label>
              <label className="flex items-center gap-2 text-xs">
                <input
                  type="radio"
                  name="freeze"
                  checked={freeze === "stop-now"}
                  onChange={() => setFreeze("stop-now")}
                />
                Stop now and switch <span className="text-muted">(partial turn marked truncated)</span>
              </label>
            </fieldset>
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
                disabled={redactionRequired}
                onClick={() => void execute()}
                data-testid="switch-confirm"
                className={
                  "rounded-lg border px-3 py-1.5 text-xs " +
                  (redactionRequired
                    ? "border-hairline text-muted"
                    : "border-accent/40 bg-accent-soft text-accent-hi hover:border-accent/60")
                }
              >
                Switch ▸ <span className="text-muted">(⏎)</span>
              </button>
            </div>
          </div>
        )}

        {phase.kind === "confirm" && phase.preflight.door === "wizard" && (
          <div className="flex flex-col gap-3" data-testid="switch-wizard">
            {/* The step indicator lists ONLY the rendered steps —
                "Step 2 of 3", never phantom steps (§8). */}
            <div
              className="flex flex-col gap-1"
              data-testid="switch-steps"
              aria-live="polite"
            >
              <p className="text-[11px] uppercase tracking-wider text-muted">
                Step {step + 1} of {steps.length}: {steps[step]}
              </p>
              <ol className="flex flex-wrap gap-1">
                {steps.map((title, index) => (
                  <li
                    key={title}
                    className={
                      "rounded-full border px-2 py-0.5 text-[10px] " +
                      (index === step
                        ? "border-accent/60 bg-accent-soft text-accent-hi"
                        : index < step
                          ? "border-hairline text-muted"
                          : "border-hairline text-muted")
                    }
                  >
                    {index < step ? "✓ " : ""}
                    {title}
                  </li>
                ))}
              </ol>
            </div>

            {steps[step] === "Target" && (
              <div className="flex flex-col gap-2" data-testid="switch-step-target">
                <h3 className="text-sm font-semibold">
                  Switch to {targetLabel}
                  <span className="ml-2 rounded border border-accent/50 px-1 text-[10px] text-accent">
                    {phase.preflight.fidelity} · beta
                  </span>
                </h3>
                <ul className="flex flex-col gap-1" data-testid="switch-wizard-reasons">
                  {phase.preflight.wizard_reasons.map((reason) => (
                    <li
                      key={reason}
                      className="rounded border border-warning/40 px-2 py-1 text-[11px] text-warning"
                    >
                      {reason}
                    </li>
                  ))}
                </ul>
                <p className="text-[11px] text-muted">
                  {phase.preflight.usual_account
                    ? "The usual account for this project — suggested first."
                    : "A suggestion only — you pick the target; nothing moves until you confirm."}
                </p>
              </div>
            )}

            {steps[step] === "What's carried" && (
              <div className="flex flex-col gap-2" data-testid="switch-step-carried">
                <h3 className="text-sm font-semibold">What's carried</h3>
                <p className="text-[11px] text-muted">
                  The pack contains exactly this list — nothing else moves.
                </p>
                <div className="flex flex-col gap-1 text-[11px]">
                  {phase.preflight.carry.carried.length > 0 && (
                    <p data-testid="switch-carry-carried">
                      <span className="text-muted">Carried: </span>
                      {phase.preflight.carry.carried.join(", ")}
                    </p>
                  )}
                  {phase.preflight.carry.regenerated.length > 0 && (
                    <p data-testid="switch-carry-rebuilt">
                      <span className="text-muted">Rebuilt by the target: </span>
                      {phase.preflight.carry.regenerated.join(", ")}
                    </p>
                  )}
                  {phase.preflight.carry.dropped.length > 0 && (
                    <p
                      className="rounded border border-warning/40 px-2 py-1 text-warning"
                      data-testid="switch-carry-dropped"
                    >
                      Not carried (stays on the source): {phase.preflight.carry.dropped.join(", ")}
                    </p>
                  )}
                </div>
                <RedactionBlock
                  preflight={phase.preflight}
                  askChoice={askChoice}
                  onAskChoice={setAskChoice}
                  chooserPick={chooserPick}
                  onChooserPick={setChooserPick}
                  onGlobalChosen={(pick) => void chooseGlobal(pick)}
                  showItems
                  unredact={unredact}
                  onToggleItem={toggleUnredact}
                />
              </div>
            )}

            {steps[step] === "Conflicts" && (
              <div className="flex flex-col gap-2" data-testid="switch-step-conflicts">
                <h3 className="text-sm font-semibold">Conflicts</h3>
                <p className="text-[11px] text-muted">
                  Both sides changed. Three-way reconciliation arrives with the switch-back
                  epic (AJ-E207) — until then this switch keeps the source's version and
                  records the fork.
                </p>
              </div>
            )}

            {steps[step] === "Rules & diff" && (
              <div className="flex flex-col gap-2" data-testid="switch-step-rules">
                <h3 className="text-sm font-semibold">Rules &amp; diff</h3>
                <p className="text-[11px] text-muted">
                  Rules or the worktree diverged between accounts — the uncommitted diff
                  rides the pack as a snapshot and the divergence is recorded.
                </p>
              </div>
            )}

            {steps[step] === "Confirm" && (
              <div className="flex flex-col gap-2" data-testid="switch-step-confirm">
                <h3 className="text-sm font-semibold">Confirm the switch</h3>
                <p className="text-[11px] text-muted" data-testid="switch-cost">
                  Cost: one switch hop · {phase.preflight.fidelity} fidelity ·{" "}
                  {phase.preflight.mechanism}
                  {phase.preflight.usual_account ? " · usual account" : ""}
                </p>
                <RedactionBlock
                  preflight={phase.preflight}
                  askChoice={askChoice}
                  onAskChoice={setAskChoice}
                  chooserPick={chooserPick}
                  onChooserPick={setChooserPick}
                  onGlobalChosen={(pick) => void chooseGlobal(pick)}
                  showItems={false}
                  unredact={unredact}
                  onToggleItem={toggleUnredact}
                />
                <p className="text-[11px] text-muted">{phase.preflight.experimental_copy}</p>
                {phase.preflight.warnings.length > 0 && (
                  <ul className="flex flex-col gap-1" data-testid="switch-warnings">
                    {phase.preflight.warnings.map((warning) => (
                      <li
                        key={warning}
                        className="rounded border border-warning/40 px-2 py-1 text-[11px] text-warning"
                      >
                        {warning}
                      </li>
                    ))}
                  </ul>
                )}
                <fieldset className="flex flex-col gap-1.5">
                  <legend className="text-[11px] uppercase tracking-wider text-muted">
                    {phase.preflight.source_live
                      ? "Freeze the source first"
                      : "The source is at rest"}
                  </legend>
                  <label className="flex items-center gap-2 text-xs">
                    <input
                      type="radio"
                      name="freeze"
                      checked={freeze === "finish-turn"}
                      onChange={() => setFreeze("finish-turn")}
                    />
                    Finish this turn, then switch <span className="text-muted">(default)</span>
                  </label>
                  <label className="flex items-center gap-2 text-xs">
                    <input
                      type="radio"
                      name="freeze"
                      checked={freeze === "stop-now"}
                      onChange={() => setFreeze("stop-now")}
                    />
                    Stop now and switch{" "}
                    <span className="text-muted">(partial turn marked truncated)</span>
                  </label>
                </fieldset>
              </div>
            )}

            <div className="flex justify-end gap-2">
              {step > 0 && (
                <button
                  type="button"
                  onClick={() => setStep((current) => Math.max(0, current - 1))}
                  className="rounded-lg border border-hairline px-3 py-1.5 text-xs hover:bg-surface-2"
                >
                  Back
                </button>
              )}
              {step < steps.length - 1 ? (
                <button
                  type="button"
                  onClick={() => setStep((current) => current + 1)}
                  data-testid="switch-continue"
                  className="rounded-lg border border-accent/40 bg-accent-soft px-3 py-1.5 text-xs text-accent-hi hover:border-accent/60"
                >
                  Continue ▸
                </button>
              ) : (
                <button
                  type="button"
                  disabled={redactionRequired}
                  onClick={() => void execute()}
                  data-testid="switch-confirm"
                  className={
                    "rounded-lg border px-3 py-1.5 text-xs " +
                    (redactionRequired
                      ? "border-hairline text-muted"
                      : "border-accent/40 bg-accent-soft text-accent-hi hover:border-accent/60")
                  }
                >
                  Switch ▸ <span className="text-muted">(⏎)</span>
                </button>
              )}
            </div>
          </div>
        )}

        {phase.kind === "executing" && (
          <div className="flex flex-col gap-2" aria-live="polite" data-testid="switch-executing">
            <h3 className="text-sm font-semibold">Switching to {targetLabel}…</h3>
            {/* Honest determinism: the stages are NAMED as they run
                (never a bare spinner), but no stage claims completion
                the engine has not reported — the single command
                reports only its terminal state. */}
            <ol className="flex flex-col gap-1">
              {STAGES.map((stage) => (
                <li key={stage} className="text-[11px] text-muted">
                  · {stage}
                </li>
              ))}
            </ol>
            <p className="text-[10px] text-muted" data-testid="switch-elapsed">
              Running — freeze → stage → verify → launch in progress. The source stays resumable
              throughout.
            </p>
          </div>
        )}

        {phase.kind === "done" && (
          <div className="flex flex-col gap-3" data-testid="switch-done">
            <h3 className="text-sm font-semibold">Switched to {targetLabel}</h3>
            <p className="text-[11px] text-muted">
              You switched — the continuation instance is starting. The source rests and stays
              resumable from the graph.
            </p>
            <div className="flex justify-end">
              <button
                type="button"
                onClick={() => onSwitched(phase.sessionId, null)}
                className="rounded-lg border border-accent/40 bg-accent-soft px-3 py-1.5 text-xs text-accent-hi"
              >
                Go to the new instance
              </button>
            </div>
          </div>
        )}

        {phase.kind === "failed" && (
          <div className="flex flex-col gap-3" data-testid="switch-failed">
            <h3 className="text-sm font-semibold text-warning">Switch didn’t complete</h3>
            <p className="rounded border border-warning/40 px-2 py-1 text-[11px] text-warning">
              {phase.message}
            </p>
            <p className="text-[11px] text-muted">
              The source is safe — resume it exactly where you left it.
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

/**
 * AJ-E206 §8.2 — the 10-second undo window after a 1-click switch:
 * "Switched — Undo" = stop the continuation instance + resume the
 * source's native session + record the compensating edge. After the
 * window closes the same actions remain available from the Switch
 * Graph (AJ-E209). Click-only: nothing auto-undoes.
 */
export function SwitchUndoToast({
  edgeId,
  targetLabel,
  onDismiss,
  onUndone,
}: {
  edgeId: string;
  targetLabel: string;
  onDismiss: () => void;
  onUndone: (sessionId: string) => void;
}): JSX.Element {
  const [secondsLeft, setSecondsLeft] = useState(10);
  const [state, setState] = useState<
    { kind: "counting" } | { kind: "undoing" } | { kind: "failed"; message: string }
  >({ kind: "counting" });

  useEffect(() => {
    if (state.kind !== "counting") return;
    if (secondsLeft <= 0) {
      onDismiss();
      return;
    }
    const timer = window.setTimeout(() => setSecondsLeft((left) => left - 1), 1000);
    return () => window.clearTimeout(timer);
  }, [secondsLeft, state.kind, onDismiss]);

  const undo = async () => {
    setState({ kind: "undoing" });
    try {
      const result = await switchUndo(edgeId);
      onUndone(result.session_id);
    } catch (cause) {
      setState({ kind: "failed", message: errorText(cause) });
    }
  };

  return (
    <div
      className="fixed inset-x-0 bottom-4 z-50 mx-auto flex w-fit max-w-[520px] items-center gap-3 rounded-xl border border-hairline bg-surface px-4 py-3 shadow-card"
      role="status"
      aria-live="polite"
      data-testid="switch-undo-toast"
    >
      {state.kind === "counting" && (
        <>
          <p className="text-xs">
            Switched to {targetLabel}.
            <span className="text-muted"> The source stays resumable — undo for </span>
            {secondsLeft}s.
          </p>
          <button
            type="button"
            onClick={() => void undo()}
            data-testid="switch-undo"
            className="rounded-lg border border-accent/40 bg-accent-soft px-3 py-1.5 text-xs text-accent-hi hover:border-accent/60"
          >
            Undo ({secondsLeft}s)
          </button>
          <button
            type="button"
            onClick={onDismiss}
            aria-label="Dismiss"
            className="rounded-lg border border-hairline px-2 py-1.5 text-xs text-muted hover:bg-surface-2"
          >
            ✕
          </button>
        </>
      )}
      {state.kind === "undoing" && (
        <p className="text-xs" data-testid="switch-undoing">
          Undoing — stopping the continuation and resuming the source…
        </p>
      )}
      {state.kind === "failed" && (
        <>
          <div className="flex flex-col">
            <p className="text-xs text-warning">Undo didn’t complete</p>
            <p className="text-[11px] text-muted">
              {state.message} — the continuation keeps running; retry or stop it from Sessions.
            </p>
          </div>
          <button
            type="button"
            onClick={onDismiss}
            className="rounded-lg border border-hairline px-3 py-1.5 text-xs hover:bg-surface-2"
          >
            Close
          </button>
        </>
      )}
    </div>
  );
}
