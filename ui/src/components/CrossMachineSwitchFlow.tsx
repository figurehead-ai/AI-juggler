/**
 * AJ-E706 — the switch wizard with a paired machine as the destination
 * (D-03: cross-machine is always a brief wizard, never one click).
 *
 * Target lists every precondition before anything runs: T2 (the same
 * conversation, transplanted) needs the same tool version and the same
 * project path, or a project mapping the user confirms; otherwise T2 is
 * blocked with its reasons and T3 (full story, fresh start) is OFFERED —
 * never a silent downgrade. "What goes along" lists what stays behind in
 * plain words before Switch now enables. Nothing crosses the link until
 * the final Switch now; the push shows progress and Stop sending stops it.
 *
 * It shares this Mac's switch dialog's frame (switch/SwitchDialogFrame):
 * the step marks and × in a header, a body that scrolls, Cancel ·
 * Back · the primary in a footer, on every step; Retry on a failure; Esc
 * waits while the push runs (§4.9), saying so.
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import {
  errorText,
  fleetSwitchCancel,
  fleetSwitchExecute,
  onFleetSwitchProgress,
  type FleetSwitchOutcomeInfo,
  type FleetSwitchProgressInfo,
  type RemoteSwitchTargetInfo,
} from "../lib/bridge";
import { byteSize } from "../lib/compact";
import { takeSwitchOpener } from "../lib/handover-focus";
import type { FreezeChoice } from "./SwitchFreezeChoice";
import { CrossCarried, CrossConfirm, CrossProject, CrossTarget, type Mechanism } from "./switch/CrossSteps";
import { BackButton, CancelButton, ContinueButton, PRIMARY, PRIMARY_WAITING, SECONDARY, StepMarks, SwitchDialogFrame, useDialogKeys } from "./switch/SwitchDialogFrame";

type Phase =
  | { kind: "wizard" }
  | { kind: "pushing"; progress: FleetSwitchProgressInfo | null }
  | { kind: "done"; outcome: FleetSwitchOutcomeInfo }
  | { kind: "failed"; message: string };

export function CrossMachineSwitchFlow({
  sessionId,
  target,
  onClose,
  onSwitched,
}: {
  sessionId: string;
  target: RemoteSwitchTargetInfo;
  onClose: () => void;
  onSwitched?: (outcome: FleetSwitchOutcomeInfo) => void;
}): JSX.Element {
  const t2Possible = target.readiness === "t2_ready" || target.readiness === "needs_mapping";
  const [mechanism, setMechanism] = useState<Mechanism>(t2Possible ? "transplant" : "replay");
  const [projectId, setProjectId] = useState<string | null>(target.same_path_project);
  const [confirmed, setConfirmed] = useState(false);
  const [secrets, setSecrets] = useState<"strip" | "keep" | null>(null);
  const [freeze, setFreeze] = useState<FreezeChoice>("finish-turn");
  const [phase, setPhase] = useState<Phase>({ kind: "wizard" });
  const [step, setStep] = useState(0);
  // §4.9: Esc (or ×) pressed while the push runs: said, not obeyed.
  const [holding, setHolding] = useState(false);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  // AJ-E804 (WCAG 2.4.3): closing returns focus to what opened it.
  const invokerRef = useRef<Element | null>(null);
  // From Continue on another account, what opened that dialog.
  if (invokerRef.current === null) invokerRef.current = takeSwitchOpener() ?? document.activeElement;

  const mapped = projectId !== null && projectId !== target.same_path_project;
  const steps = useMemo(() => {
    const list = ["Target"];
    if (target.same_path_project === null) list.push("Project");
    list.push("What's carried", "Confirm");
    return list;
  }, [target.same_path_project]);
  const current = steps[step];
  const projectReady = projectId !== null && (!mapped || confirmed);
  const secretsReady = mechanism === "transplant" || secrets !== null;
  const stepReady = current === "Project" ? projectReady : current === "What's carried" ? secretsReady : true;
  const armed = current === "Confirm" && projectReady && secretsReady;
  const pushing = phase.kind === "pushing";

  useEffect(() => onFleetSwitchProgress((update) => {
    if (update.machine_id !== target.machine_id) return;
    setPhase((now) => (now.kind === "pushing" ? { kind: "pushing", progress: update } : now));
  }), [target.machine_id]);

  const close = () => {
    if (pushing) {
      setHolding(true);
      return;
    }
    const invoker = invokerRef.current;
    onClose();
    requestAnimationFrame(() => {
      if (invoker instanceof HTMLElement && invoker.isConnected) invoker.focus();
    });
  };

  const execute = async () => {
    setHolding(false);
    setPhase({ kind: "pushing", progress: null });
    try {
      const outcome = await fleetSwitchExecute({
        session_id: sessionId,
        machine_id: target.machine_id,
        recipe_id: target.recipe_id,
        mechanism,
        project_id: projectId,
        mapping_confirmed: confirmed,
        freeze_choice: freeze,
        redaction_choice: mechanism === "replay" ? secrets : null,
      });
      if (outcome.accepted) {
        setPhase({ kind: "done", outcome });
        onSwitched?.(outcome);
      } else {
        setPhase({ kind: "failed", message: outcome.message });
      }
    } catch (cause) {
      setPhase({ kind: "failed", message: errorText(cause) });
    }
  };

  // Focus, Tab and Esc: the switch dialogs' shared rules.
  useDialogKeys(dialogRef, `${phase.kind}:${step}`, close);

  const machine = target.machine_label;
  const projectName = target.projects.find((project) => project.project_id === projectId)?.name ?? "the project";
  const cancel = <CancelButton onClick={close} testId="cross-switch-cancel" />;
  const badge = mechanism === "transplant" ? target.badge : "Full story, fresh start · Not proven yet";

  const header = phase.kind === "wizard" && <StepMarks steps={steps} step={step} testId="cross-switch-steps" />;

  const body = (() => {
    switch (phase.kind) {
      case "wizard":
        return (
          <div className="flex flex-col gap-4" data-testid="cross-switch-wizard">
            {current === "Target" && <CrossTarget target={target} badge={badge} mechanism={mechanism} onMechanism={setMechanism} t2Possible={t2Possible} />}
            {current === "Project" && (
              <CrossProject
                target={target}
                projectId={projectId}
                onProject={(id) => {
                  setProjectId(id);
                  setConfirmed(false);
                }}
                confirmed={confirmed}
                onConfirmed={setConfirmed}
              />
            )}
            {current === "What's carried" && <CrossCarried target={target} mechanism={mechanism} secrets={secrets} onSecrets={setSecrets} />}
            {current === "Confirm" && <CrossConfirm target={target} badge={badge} mechanism={mechanism} projectName={projectName} freeze={freeze} onFreeze={setFreeze} />}
          </div>
        );
      case "pushing":
        return (
          <div className="flex flex-col gap-3" data-testid="cross-switch-pushing" aria-live="polite">
            <h3 className="text-title text-ink">Switching to {target.account_label} on {machine}…</h3>
            <p className="text-body text-ink">
              {phase.progress?.phase === "installing" ? `${machine} is setting it up and starting it…` : `Sending it to ${machine}…`}
            </p>
            {phase.progress && phase.progress.total_bytes > 0 && (
              <>
                <div
                  role="progressbar"
                  aria-label={`Sent to ${machine}`}
                  aria-valuemin={0}
                  aria-valuemax={phase.progress.total_bytes}
                  aria-valuenow={phase.progress.received_bytes}
                  className="h-1.5 w-full overflow-hidden rounded-full bg-hairline"
                >
                  <div className="h-full bg-accent" style={{ width: `${Math.min(100, (phase.progress.received_bytes / phase.progress.total_bytes) * 100)}%` }} />
                </div>
                <p className="text-meta text-ink">
                  {byteSize(phase.progress.received_bytes)} of {byteSize(phase.progress.total_bytes)}
                </p>
              </>
            )}
            <p className="text-body text-ink">The original stays safe on this computer until the new session is running.</p>
            {holding && (
              <p className="text-body font-semibold text-ink" role="status" data-testid="cross-switch-holding">
                Still sending. Choose Stop sending to stop it; nothing changes on either computer.
              </p>
            )}
          </div>
        );
      case "done":
        return (
          <div className="flex flex-col gap-3" data-testid="cross-switch-done" aria-live="polite">
            <h3 className="text-title text-ink">Switched to {target.account_label} on {machine}</h3>
            <p className="text-body text-ink">
              {phase.outcome.message}. This session rests here, handed over; you can switch back to it any time.
            </p>
          </div>
        );
      case "failed":
        // A short title, then the failure's own words: what happened and
        // what to do next, each said once, the other computer named
        // (crossmachine/failure.rs). Nothing is added to them here.
        return (
          <div className="flex flex-col gap-4" data-testid="cross-switch-failed" role="alert">
            <h3 className="text-title text-warning">The switch didn't complete</h3>
            <p className="rounded-lg border border-warning/40 px-3 py-2 text-body text-warning" data-testid="cross-switch-failed-words">
              {phase.message}
            </p>
          </div>
        );
    }
  })();

  const footer = (() => {
    switch (phase.kind) {
      case "wizard":
        return (
          <>
            {cancel}
            {step > 0 && <BackButton onClick={() => setStep(step - 1)} testId="cross-switch-back" />}
            {current !== "Confirm" ? (
              <ContinueButton ready={stepReady} onClick={() => setStep(step + 1)} testId="cross-switch-next" />
            ) : (
              <button type="button" disabled={!armed} onClick={(event) => event.detail < 2 && void execute()} className={armed ? PRIMARY : PRIMARY_WAITING} data-testid="cross-switch-go">
                Switch now
              </button>
            )}
          </>
        );
      case "pushing":
        return (
          <button type="button" onClick={() => void fleetSwitchCancel(target.machine_id)} className={SECONDARY} data-testid="cross-switch-stop">
            Stop sending
          </button>
        );
      case "done":
        return (
          <button type="button" data-autofocus onClick={close} className={PRIMARY} data-testid="cross-switch-close">
            Close
          </button>
        );
      case "failed":
        return (
          <>
            {cancel}
            <button type="button" data-autofocus onClick={() => void execute()} className={PRIMARY} data-testid="cross-switch-retry">
              Retry
            </button>
          </>
        );
    }
  })();

  return (
    <SwitchDialogFrame dialogRef={dialogRef} label={`Switch to ${target.account_label} on ${machine}`} testId="cross-switch-flow" header={header} footer={footer} onClose={pushing ? null : close}>
      {body}
    </SwitchDialogFrame>
  );
}
