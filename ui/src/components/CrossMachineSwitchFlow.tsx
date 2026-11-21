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
import { errorText } from "../lib/bridge";
import { fleetSwitchCancel, fleetSwitchExecute, onFleetSwitchProgress } from "../lib/ipc/fleet-switch";
import type { FleetSwitchOutcomeInfo } from "../types/generated/FleetSwitchOutcomeInfo";
import type { FleetSwitchProgressInfo } from "../types/generated/FleetSwitchProgressInfo";
import type { RemoteSwitchTargetInfo } from "../types/generated/RemoteSwitchTargetInfo";
import { useSwitchInvoker } from "../lib/handover-focus";
import type { FreezeChoice } from "./SwitchFreezeChoice";
import { CrossCarried, CrossConfirm, CrossDone, CrossFailed, CrossHeader, CrossProject, CrossSending, CrossTarget, type Mechanism } from "./switch/CrossSteps";
import { BackButton, CancelButton, ContinueButton, StepMarks, SwitchDialogFrame, useDialogKeys } from "./switch/SwitchDialogFrame";
import { Button } from "./primitives";

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
  // AJ-E804 (WCAG 2.4.3): closing returns focus to what opened it; from
  // Continue on another account, what opened that dialog.
  const invokerRef = useSwitchInvoker();

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
  // D-127: a switch to another Mac is listed only with beta switches on.
  const badge = mechanism === "transplant" ? target.badge : "Full story, fresh start · Beta";

  const header = <CrossHeader target={target} marks={phase.kind === "wizard" && <StepMarks steps={steps} step={step} testId="cross-switch-steps" />} />;

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
        return <CrossSending target={target} progress={phase.progress} holding={holding} />;
      case "done":
        return <CrossDone target={target} message={phase.outcome.message} />;
      case "failed":
        return <CrossFailed message={phase.message} />;
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
              <Button variant="primary" type="button" disabled={!armed} onClick={(event) => event.detail < 2 && void execute()} data-testid="cross-switch-go">
                {/* 0.3.5: the button says where it continues, as on this Mac. */}
                Switch to CLI on {target.machine_label}
              </Button>
            )}
          </>
        );
      case "pushing":
        return (
          <Button variant="ghost" type="button" onClick={() => void fleetSwitchCancel(target.machine_id)} data-testid="cross-switch-stop">
            Stop sending
          </Button>
        );
      case "done":
        return (
          <Button variant="primary" type="button" data-autofocus onClick={close} data-testid="cross-switch-close">
            Close
          </Button>
        );
      case "failed":
        return (
          <>
            {cancel}
            <Button variant="primary" type="button" data-autofocus onClick={() => void execute()} data-testid="cross-switch-retry">
              Retry
            </Button>
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
