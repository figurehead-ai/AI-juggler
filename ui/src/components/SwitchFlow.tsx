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
 * carry list in plain words; and a confirm screen to read. 0.3: done,
 * what the other tool was told (`switch/DoneStep`).
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { BackButton, CancelButton, ContinueButton, PRIMARY, PRIMARY_WAITING, StepMarks, SwitchDialogFrame, useDialogKeys } from "./switch/SwitchDialogFrame";
import type { FreezeChoice } from "./SwitchFreezeChoice";
import { SwitchSetupResults } from "./SwitchSetupList";
import { takeSwitchOpener } from "../lib/handover-focus";
import { useSwitchSetup } from "../lib/useSwitchSetup";
import { showToast } from "./Toast";
import { ConfirmBody } from "./switch/ConfirmBody";
import { CarryList } from "./switch/CarryList";
import { DoneStep } from "./switch/DoneStep";
import { Executing, OtherStep, risky, TargetStep, useTargetRoom } from "./switch/StepBodies";
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
  | { kind: "done"; sessionId: string; edgeId: string; preflight: PreflightInfoData }
  // A failure keeps what it failed on: Retry runs the switch again from
  // there (or checks it again when the check itself failed).
  | { kind: "failed"; message: string; preflight: PreflightInfoData | null };

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
  const room = useTargetRoom(phase.kind === "confirm" ? phase.preflight.target_account : undefined); // its room: the one runway (F1)
  const { setup, picked, toggle: togglePicked } = setupCheck;
  const dialogRef = useRef<HTMLDivElement | null>(null);
  // AJ-E804 (WCAG 2.4.3): cancelling returns focus to the control that
  // opened the switch. (A completed switch hands focus to the undo
  // toast instead.)
  const invokerRef = useRef<Element | null>(null);
  // One switch per confirm: a second ⏎ before the phase re-renders would
  // read the stale "confirm" and start another (review of #132).
  const started = useRef(false);
  // From Continue on another account, what opened that dialog.
  if (invokerRef.current === null) invokerRef.current = takeSwitchOpener() ?? document.activeElement;
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

  // Focus, Tab, Esc and ⏎: the switch dialogs' shared rules (2.1.2/2.1.1
  // — the dialog is the only operable surface while open).
  useDialogKeys(dialogRef, `${phase.kind}:${step}`, cancel, executeArmed ? () => void execute(phase.kind === "confirm" ? phase.preflight : null) : undefined);

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
      const result = await switchExecute(sessionId, targetProfileId, freeze, choice, answer === "strip" ? [...unhide] : [], setupCheck.approved());
      if (oneClick && applied.length === 0) {
        // The modal closes into the undo window (App renders the
        // toast); the graph keeps the edge as the undo handle.
        onSwitched(result.session_id, { edgeId: result.edge_id });
        return;
      }
      setPhase({ kind: "done", sessionId: result.session_id, edgeId: result.edge_id, preflight });
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
      room={room}
      freeze={freeze}
      onFreeze={setFreeze}
      setup={setup}
      picked={picked}
      onToggle={togglePicked}
      onCheckAgain={setupCheck.recheck}
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
      className={keysPending ? PRIMARY_WAITING : PRIMARY}
      aria-keyshortcuts="Enter"
    >
      {/* 0.3.2: from another app's session, the button says where it goes on. */}
      {preflight.source_outside ? (risky(room) ? "Continue here anyway" : "Continue here in AI Juggler") : risky(room) ? "Switch anyway" : "Switch now"}
    </button>
  );
  const cancelButton = <CancelButton onClick={cancel} testId="switch-cancel" />;

  // A header (step marks, ×), a body that scrolls, a footer with the
  // buttons: the way on and out always on screen (any size, step, phase).
  const wizard = phase.kind === "confirm" && phase.preflight.door === "wizard" ? phase.preflight : null;
  const carriedPending = wizard !== null && steps[step] === "What's carried" && keysPending;

  // The step marks list ONLY the rendered steps — "Step 2 of 3", never
  // phantom steps (§8).
  const header = wizard && <StepMarks steps={steps} step={step} testId="switch-steps" />;

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
            {steps[step] === "Target" && <TargetStep preflight={phase.preflight} target={targetLabel} room={room} />}
            {steps[step] === "What's carried" && (
              <div className="flex flex-col gap-4" data-testid="switch-step-carried">
                <h3 className="text-title text-ink">What goes along</h3>
                <CarryList items={phase.preflight.carry_items} target={targetLabel} tool={toolSpec(phase.preflight.target_tool).display_name} setup={setup} picked={picked} />
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
        return <DoneStep preflight={phase.preflight} target={targetLabel} edgeId={phase.edgeId} results={setupCheck.results} />;
      case "failed":
        // A short title, then the failure's own words: what happened and
        // what to do next, each said once (`SwitchFailure::words`; any
        // other error's words say what to do too). Nothing is added to
        // them here, so nothing is said twice.
        return (
          <div className="flex flex-col gap-4" data-testid="switch-failed">
            <h3 className="text-title text-warning">The switch didn't complete</h3>
            <p className="rounded-lg border border-warning/40 px-3 py-2 text-body text-warning" data-testid="switch-failed-words">
              {phase.message}
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
            {step > 0 && <BackButton onClick={() => setStep((current) => Math.max(0, current - 1))} testId="switch-back" />}
            {step < steps.length - 1 ? (
              <ContinueButton ready={!carriedPending} onClick={() => setStep((current) => current + 1)} testId="switch-continue" />
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
    <SwitchDialogFrame dialogRef={dialogRef} label={`Switch to ${targetLabel}`} testId="switch-flow" header={header} footer={footer} onClose={running ? null : cancel}>
      {body}
    </SwitchDialogFrame>
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
