/**
 * AJ-E205/E206 — the switch execution surface. The door (pass2/02
 * §8) picks the shape at preflight: a proven move renders one confirm
 * screen, everything else opens the wizard, which renders ONLY the
 * steps whose data is non-trivial ("Step 2 of 3", never phantom
 * steps). No handover executes from a preview the user has not seen:
 * the Switch click lives on the last screen. Executing names its stages
 * in plain words (never a bare spinner); failure keeps the original safe
 * and offers Retry; a completed one-click switch offers a 10-second
 * undo toast. Copy never says "we switched you" (SPEC:26).
 *
 * The maintainer's walk of 0.1.8 (2026-10-27): Cancel on every step and
 * a close × (but while the copy runs, §4.9: Esc and × wait, "Finishing
 * copy…"); the keys question only when the words go somewhere new; the
 * carry list in plain words; and a confirm screen to read.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { outsideWords, type FreezeChoice } from "./SwitchFreezeChoice";
import { SwitchSetupResults } from "./SwitchSetupList";
import { useSwitchSetup } from "../lib/useSwitchSetup";
import { showToast } from "./Toast";
import { ConfirmBody } from "./switch/ConfirmBody";
import { CarryList } from "./switch/CarryList";
import { Executing, OtherStep, TargetStep } from "./switch/StepBodies";
import { keysChoice, KeysQuestion, keysRequired, type KeysAnswer } from "./switch/KeysQuestion";
import { toolSpec } from "../lib/tools";
import {
  switchPreflight,
  switchDoorCount,
  switchExecute,
  switchUndo,
  redactionPolicySetGlobal,
  type PreflightInfoData,
  errorText,
} from "../lib/bridge";

type Phase =
  | { kind: "preflight" }
  | { kind: "confirm"; preflight: PreflightInfoData }
  | { kind: "executing"; startedAt: number }
  | { kind: "done"; sessionId: string; preflight: PreflightInfoData }
  // A failure keeps what it failed on: Retry runs the switch again from
  // there (or checks it again when the check itself failed).
  | { kind: "failed"; message: string; preflight: PreflightInfoData | null };

const BUTTON = "min-h-8 rounded-lg border px-3.5 text-body";
const SECONDARY = `${BUTTON} border-hairline text-ink hover:bg-surface-2`;
const PRIMARY = `${BUTTON} border-accent/50 bg-accent-soft font-semibold text-accent-hi hover:border-accent/70`;

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
  const [attempt, setAttempt] = useState(0);
  const [freeze, setFreeze] = useState<FreezeChoice>("finish-turn");
  const [step, setStep] = useState(0);
  // The keys question: this switch's answer, whether to remember it, a
  // remembered choice being changed, and the ones shown anyway.
  const [answer, setAnswer] = useState<KeysAnswer | null>(null);
  const [remember, setRemember] = useState(false);
  const [changing, setChanging] = useState(false);
  const [unhide, setUnhide] = useState<Set<number>>(new Set());
  // §4.9: Esc (or ×) pressed while the copy runs: said, not obeyed.
  const [holding, setHolding] = useState(false);
  // AJ-E1542: what the target lacks, and what the user picked to set up.
  const setupCheck = useSwitchSetup(sessionId, targetProfileId);
  const { setup, picked, toggle: togglePicked } = setupCheck;
  const dialogRef = useRef<HTMLDivElement | null>(null);
  // AJ-E804 (WCAG 2.4.3): cancelling returns focus to the control that
  // opened the switch. (A completed switch hands focus to the undo
  // toast instead.)
  const invokerRef = useRef<Element | null>(null);
  // One switch per confirm: a second ⏎ before the phase re-renders would
  // read the stale "confirm" and start another (review of #132).
  const started = useRef(false);
  if (invokerRef.current === null) invokerRef.current = document.activeElement;
  const running = phase.kind === "executing";
  // Set the moment the switch starts, so an Esc that comes before the
  // next render is held too.
  const runningRef = useRef(false);
  runningRef.current = running || (runningRef.current && phase.kind !== "failed" && phase.kind !== "done");
  const cancel = () => {
    if (runningRef.current) {
      setHolding(true);
      return;
    }
    const invoker = invokerRef.current;
    onClose();
    requestAnimationFrame(() => {
      if (invoker instanceof HTMLElement && invoker.isConnected) invoker.focus();
    });
  };

  // A new target starts with no answers.
  useEffect(() => {
    setAnswer(null);
    setRemember(false);
    setChanging(false);
    setUnhide(new Set());
  }, [sessionId, targetProfileId]);

  useEffect(() => {
    let alive = true;
    setPhase({ kind: "preflight" });
    switchPreflight(sessionId, targetProfileId)
      .then((preflight) => {
        if (alive) {
          setStep(0);
          setPhase({ kind: "confirm", preflight });
          // The door shown, counted once per opening (local only).
          if (attempt === 0) void switchDoorCount(preflight.door, preflight.wizard_steps).catch(() => undefined);
        }
      })
      .catch((cause) => {
        if (alive) setPhase({ kind: "failed", message: errorText(cause), preflight: null });
      });
    return () => {
      alive = false;
    };
    // The door is counted for the first check only.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId, targetProfileId, attempt]);

  const steps = phase.kind === "confirm" && phase.preflight.door === "wizard" ? phase.preflight.wizard_steps : [];
  const keysPending = phase.kind === "confirm" && keysRequired(phase.preflight, answer, changing);
  // The Switch click lives on the one-click screen or the wizard's LAST
  // step only — never mid-wizard (deep/09 H-5).
  const executeArmed = phase.kind === "confirm" && (phase.preflight.door === "one-click" || step === steps.length - 1) && !keysPending;

  // Focus trap + Esc/⏎ handling (2.1.2/2.1.1 — the dialog is the
  // only operable surface while open).
  useEffect(() => {
    const node = dialogRef.current;
    // The phase's own default (Retry, Go to the new session) first; else
    // the first control, in the header, body, footer order.
    (node?.querySelector<HTMLElement>("[data-autofocus]") ?? node?.querySelector<HTMLElement>("button, input"))?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        cancel();
        return;
      }
      if (event.key === "Enter" && executeArmed && !(event.target instanceof HTMLButtonElement)) {
        event.preventDefault();
        void execute(phase.kind === "confirm" ? phase.preflight : null);
      }
      if (event.key === "Tab" && node) {
        const focusables = Array.from(node.querySelectorAll<HTMLElement>("button:not(:disabled), input, summary"));
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase.kind, step, executeArmed, onClose]);

  const toggleUnhide = (index: number) =>
    setUnhide((current) => {
      const next = new Set(current);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });

  const execute = async (preflight: PreflightInfoData | null) => {
    if (preflight === null || started.current) return;
    started.current = true;
    setHolding(false);
    // AJ-E2108: a session another app runs has nothing here to undo to
    // (it stayed as it was): the done step says so instead of the toast.
    const oneClick = preflight.door === "one-click" && !preflight.source_outside;
    const choice = keysChoice(preflight, answer);
    runningRef.current = true;
    setPhase({ kind: "executing", startedAt: Date.now() });
    try {
      // "Remember my choice": saved as the policy before the switch reads it.
      if (remember && answer !== null) await redactionPolicySetGlobal(answer);
      // AJ-E1542: what the user picked is set up first, so the target
      // starts with it.
      const applied = await setupCheck.applyPicked();
      const result = await switchExecute(sessionId, targetProfileId, freeze, choice, answer === "strip" ? [...unhide] : []);
      if (oneClick && applied.length === 0) {
        // The modal closes into the undo window (App renders the
        // toast); the graph keeps the edge as the undo handle.
        onSwitched(result.session_id, { edgeId: result.edge_id });
        return;
      }
      setPhase({ kind: "done", sessionId: result.session_id, preflight });
    } catch (cause) {
      started.current = false;
      runningRef.current = false;
      setPhase({ kind: "failed", message: errorText(cause), preflight });
    }
  };

  const retry = (failed: PreflightInfoData | null) => {
    if (failed) void execute(failed);
    else setAttempt((n) => n + 1);
  };

  const keys = (preflight: PreflightInfoData) => (
    <KeysQuestion
      preflight={preflight}
      answer={answer}
      onAnswer={setAnswer}
      remember={remember}
      onRemember={setRemember}
      changing={changing}
      onChange={() => setChanging(true)}
      unhide={unhide}
      onToggle={toggleUnhide}
    />
  );
  const confirmBody = (preflight: PreflightInfoData) => (
    <ConfirmBody
      preflight={preflight}
      target={targetLabel}
      freeze={freeze}
      onFreeze={setFreeze}
      setup={setup}
      picked={picked}
      onToggle={togglePicked}
      onRecheck={setupCheck.recheck}
      checking={setupCheck.checking}
      // One screen: the keys question (when it applies) is asked here.
      extra={preflight.door === "one-click" && preflight.secrets.asks ? keys(preflight) : undefined}
    />
  );
  const switchButton = (preflight: PreflightInfoData) => (
    <button
      key="switch-now"
      type="button"
      disabled={keysPending}
      // Continue and Switch now sit in one place: the second click of a
      // double-click on Continue never starts the switch from a screen
      // the user hasn't seen (deep/09 H-5).
      onClick={(event) => event.detail < 2 && void execute(preflight)}
      data-testid="switch-confirm"
      className={keysPending ? `${BUTTON} border-hairline text-muted` : PRIMARY}
      aria-keyshortcuts="Enter"
    >
      Switch now
    </button>
  );
  const cancelButton = (
    <button type="button" onClick={cancel} className={SECONDARY} data-testid="switch-cancel" aria-keyshortcuts="Escape">
      Cancel <span className="text-muted">(Esc)</span>
    </button>
  );

  // The dialog is a header (the step marks and ×), a body that scrolls,
  // and a footer with the buttons: the way on and the way out are always
  // on screen, at any window size, on every step and in every phase.
  const wizard = phase.kind === "confirm" && phase.preflight.door === "wizard" ? phase.preflight : null;
  const carriedPending = wizard !== null && steps[step] === "What's carried" && keysPending;

  const header = wizard && (
    // The step indicator lists ONLY the rendered steps — "Step 2 of 3",
    // never phantom steps (§8).
    <div className="flex flex-col gap-1.5" data-testid="switch-steps" aria-live="polite">
      <p className="text-meta text-ink">
        Step {step + 1} of {steps.length}: {steps[step]}
      </p>
      <ol className="flex flex-wrap gap-1">
        {steps.map((title, index) => (
          <li key={title} aria-current={index === step ? "step" : undefined} className={"rounded-full border px-2.5 py-0.5 text-label " + (index === step ? "border-accent/60 bg-accent-soft text-accent-hi" : "border-hairline text-ink")}>
            {index < step ? "✓ " : ""}
            {title}
          </li>
        ))}
      </ol>
    </div>
  );

  const body = (() => {
    switch (phase.kind) {
      case "preflight":
        return (
          <p className="text-body text-ink" data-testid="switch-preflighting">
            Checking the switch to {targetLabel}…
          </p>
        );
      case "confirm":
        if (phase.preflight.door === "one-click") {
          return (
            <div className="flex flex-col gap-5" data-testid="switch-oneclick">
              {confirmBody(phase.preflight)}
            </div>
          );
        }
        return (
          <div className="flex flex-col gap-5" data-testid="switch-wizard">
            {steps[step] === "Target" && <TargetStep preflight={phase.preflight} target={targetLabel} />}
            {steps[step] === "What's carried" && (
              <div className="flex flex-col gap-4" data-testid="switch-step-carried">
                <h3 className="text-title text-ink">What goes along</h3>
                <CarryList items={phase.preflight.carry_items} target={targetLabel} tool={toolSpec(phase.preflight.target_tool).display_name} />
                {keys(phase.preflight)}
              </div>
            )}
            {(steps[step] === "Conflicts" || steps[step] === "Rules & diff") && <OtherStep step={steps[step] as "Conflicts" | "Rules & diff"} />}
            {steps[step] === "Confirm" && confirmBody(phase.preflight)}
          </div>
        );
      case "executing":
        return <Executing target={targetLabel} holding={holding} />;
      case "done":
        return (
          <div className="flex flex-col gap-4" data-testid="switch-done">
            <h3 className="text-title text-ink">Switched to {targetLabel}</h3>
            <p className="text-body text-ink" data-testid="switch-done-words">
              {phase.preflight.source_outside
                ? outsideWords(phase.preflight.source_app, phase.preflight.source_live, targetLabel)
                : phase.preflight.source_keeps_running
                  ? `${targetLabel} is picking up the conversation. The original keeps going in its app.`
                  : `${targetLabel} is picking up the conversation. The original has stopped, and you can switch back to it any time.`}
            </p>
            <SwitchSetupResults target={targetLabel} results={setupCheck.results} />
          </div>
        );
      case "failed":
        return (
          <div className="flex flex-col gap-4" data-testid="switch-failed">
            <h3 className="text-title text-warning">The switch didn't complete</h3>
            <p className="rounded-lg border border-warning/40 px-3 py-2 text-body text-warning" data-testid="switch-failed-words">
              {phase.message}
            </p>
            <p className="text-body text-ink">
              {setupCheck.results.length > 0 ? "The original stays exactly as it was" : "Nothing was changed: the original stays exactly as it was"} — resume it where you left it, or try again.
            </p>
            {/* What was set up before the switch failed stays undoable. */}
            <SwitchSetupResults target={targetLabel} results={setupCheck.results} />
          </div>
        );
    }
  })();

  const footer = (() => {
    switch (phase.kind) {
      case "preflight":
        return cancelButton;
      case "confirm":
        if (phase.preflight.door === "one-click") {
          return (
            <>
              {cancelButton}
              {switchButton(phase.preflight)}
            </>
          );
        }
        return (
          <>
            {carriedPending && (
              <p className="mr-auto text-meta text-ink" data-testid="switch-keys-pending">
                Choose what happens to the keys first.
              </p>
            )}
            {cancelButton}
            {step > 0 && (
              <button type="button" onClick={() => setStep((current) => Math.max(0, current - 1))} className={SECONDARY} data-testid="switch-back">
                Back
              </button>
            )}
            {step < steps.length - 1 ? (
              <button key="continue" type="button" disabled={carriedPending} onClick={() => setStep((current) => current + 1)} data-testid="switch-continue" className={carriedPending ? `${BUTTON} border-hairline text-muted` : PRIMARY}>
                Continue
              </button>
            ) : (
              switchButton(phase.preflight)
            )}
          </>
        );
      case "executing":
        return null;
      case "done":
        return (
          <button type="button" data-autofocus onClick={() => onSwitched(phase.sessionId, null)} className={PRIMARY}>
            Go to the new session
          </button>
        );
      case "failed":
        return (
          <>
            {cancelButton}
            <button type="button" data-autofocus onClick={() => retry(phase.preflight)} className={PRIMARY} data-testid="switch-retry">
              Retry
            </button>
          </>
        );
    }
  })();

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={`Switch to ${targetLabel}`}
        data-testid="switch-flow"
        className="flex max-h-[min(92vh,760px)] w-full max-w-[680px] flex-col overflow-hidden rounded-xl border border-hairline bg-surface shadow-card"
      >
        <div className="flex shrink-0 items-start gap-3 px-6 pt-5" data-testid="switch-header">
          <div className="min-w-0 flex-1 pb-3">{header}</div>
          {!running && (
            <button type="button" onClick={cancel} aria-label="Close" title="Close (Esc)" className="-mr-3 -mt-2 flex size-8 shrink-0 items-center justify-center rounded-lg text-ink hover:bg-surface-2" data-testid="switch-close">
              <span aria-hidden="true" className="text-lg leading-none">×</span>
            </button>
          )}
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-6 pb-5" data-testid="switch-body">
          {body}
        </div>
        {footer && (
          <div className="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-hairline px-6 py-3" data-testid="switch-footer">
            {footer}
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
 * Graph (AJ-E209). Click-only: nothing auto-undoes. AJ-E1503: it is the
 * app's one toast; the switch dialog has just closed, so Undo takes the
 * focus that would otherwise fall to the page (WCAG 2.4.3).
 */
export function showSwitchUndoToast({
  edgeId,
  targetLabel,
  onUndone,
  onClose,
}: {
  edgeId: string;
  targetLabel: string;
  onUndone: (sessionId: string) => void;
  onClose?: () => void;
}): string {
  return showToast({
    id: "switch-undo",
    testId: "switch-undo",
    message: `Switched to ${targetLabel}.`,
    detail: "The original stays resumable.",
    focusWhenFree: true,
    onClose,
    undo: {
      seconds: 10,
      onUndo: async () => {
        const result = await switchUndo(edgeId);
        onUndone(result.session_id);
      },
      undoing: "Undoing — stopping the new session and resuming the original…",
      done: "Undone — back on the original session.",
      failed: (cause) => `${cause} — the new session keeps running; try again or stop it from its page.`,
    },
  });
}
