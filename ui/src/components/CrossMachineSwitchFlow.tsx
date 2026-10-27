/**
 * AJ-E706 — the switch wizard with a paired machine as the destination
 * (D-03: cross-machine is always a brief wizard, never one click).
 *
 * Target lists every precondition before anything runs: T2 (the same
 * conversation, transplanted) needs the same harness version and the same
 * project path, or a project mapping the user confirms; otherwise T2 is
 * blocked with its reasons and T3 (full story, fresh start) is OFFERED —
 * never a silent downgrade. "What's carried" lists what will drop in
 * plain words before Switch enables. Nothing crosses the link until the
 * final "Switch to <machine>…" click; the push shows progress and can be
 * cancelled. Keyboard: radio groups with descriptions, Esc closes, Tab
 * stays in the dialog.
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
  const [mechanism, setMechanism] = useState<"transplant" | "replay">(t2Possible ? "transplant" : "replay");
  const [projectId, setProjectId] = useState<string | null>(target.same_path_project);
  const [confirmed, setConfirmed] = useState(false);
  const [secrets, setSecrets] = useState<"strip" | "keep" | null>(null);
  const [freeze, setFreeze] = useState<"finish-turn" | "stop-now">("finish-turn");
  const [phase, setPhase] = useState<Phase>({ kind: "wizard" });
  const [step, setStep] = useState(0);
  const dialogRef = useRef<HTMLDivElement | null>(null);

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
  const stepReady =
    current === "Project" ? projectReady : current === "What's carried" ? secretsReady : true;
  const armed = current === "Confirm" && projectReady && secretsReady;

  useEffect(() => onFleetSwitchProgress((update) => {
    if (update.machine_id !== target.machine_id) return;
    setPhase((now) => (now.kind === "pushing" ? { kind: "pushing", progress: update } : now));
  }), [target.machine_id]);

  useEffect(() => {
    const node = dialogRef.current;
    node?.querySelector<HTMLElement>("input:checked, button, input")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        if (phase.kind === "pushing") void fleetSwitchCancel(target.machine_id);
        else onClose();
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
  }, [onClose, phase.kind, step, target.machine_id]);

  const execute = async () => {
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

  const machine = target.machine_label;
  const projectName =
    target.projects.find((project) => project.project_id === projectId)?.name ?? "the project";
  const orderedProjects = [...target.projects].sort(
    (a, b) => Number(b.same_path) - Number(a.same_path) || Number(b.same_name) - Number(a.same_name),
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="cross-switch-title"
        data-testid="cross-switch-flow"
        className="flex max-h-[90vh] w-full max-w-lg flex-col gap-3 overflow-y-auto rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        <h3 id="cross-switch-title" className="flex flex-wrap items-center gap-2 text-sm font-semibold">
          Switch to {target.account_label} on {machine}
          <span className="rounded border border-warning/60 px-1.5 py-px text-label font-normal text-warning">
            {mechanism === "transplant" ? target.badge : "Full story, fresh start · Not proven yet"}
          </span>
        </h3>

        {phase.kind === "wizard" && (
          <>
            <div className="flex flex-col gap-1" aria-live="polite" data-testid="cross-switch-steps">
              <p className="text-section font-semibold text-ink">
                Step {step + 1} of {steps.length}: {current}
              </p>
              <ol className="flex flex-wrap gap-1">
                {steps.map((title, index) => (
                  <li
                    key={title}
                    className={
                      "rounded-full border px-2 py-0.5 text-label " +
                      (index === step ? "border-accent/60 bg-accent-soft text-accent-hi" : "border-hairline text-muted")
                    }
                  >
                    {index < step ? "✓ " : ""}
                    {title}
                  </li>
                ))}
              </ol>
            </div>

            {current === "Target" && (
              <div className="flex flex-col gap-2" data-testid="cross-switch-target">
                <ul className="flex flex-col gap-1 text-label" data-testid="cross-switch-preconditions">
                  {target.provider === "" ? null : (
                    <li className={target.version_there && target.version_here === target.version_there ? "text-ink" : "text-muted"}>
                      <span aria-hidden="true">
                        {target.version_there && target.version_here === target.version_there ? "✓ " : "✗ "}
                      </span>
                      {target.provider} {target.version_here ?? "unknown"} here,{" "}
                      {target.version_there ?? "not installed"} on {machine}
                    </li>
                  )}
                  <li className={target.same_path_project ? "text-ink" : "text-muted"}>
                    <span aria-hidden="true">{target.same_path_project ? "✓ " : "✗ "}</span>
                    {target.same_path_project
                      ? `The project is at the same path on ${machine}`
                      : `The project isn't at the same path on ${machine}`}
                  </li>
                </ul>
                {target.blockers.length > 0 && (
                  <ul className="flex flex-col gap-1" data-testid="cross-switch-blockers">
                    {target.blockers.map((reason) => (
                      <li key={reason} className="rounded border border-warning/40 px-2 py-1 text-label text-ink">
                        <span aria-hidden="true">⚠ </span>
                        {reason}
                      </li>
                    ))}
                  </ul>
                )}
                <div role="radiogroup" aria-label="How the work moves" className="flex flex-col gap-1.5">
                  <label
                    className={
                      "flex items-start gap-2 rounded-lg border px-2.5 py-2 text-label " +
                      (mechanism === "transplant" ? "border-accent/60 bg-accent-soft" : "border-hairline") +
                      (t2Possible ? "" : " opacity-70")
                    }
                  >
                    <input
                      type="radio"
                      name="cross-mechanism"
                      checked={mechanism === "transplant"}
                      disabled={!t2Possible}
                      onChange={() => setMechanism("transplant")}
                      aria-describedby="cross-t2-desc"
                      data-testid="cross-switch-t2"
                    />
                    <span>
                      <span className="block font-medium text-ink">Same conversation</span>
                      <span id="cross-t2-desc" className="text-muted">
                        {t2Possible
                          ? `${machine} resumes this conversation — its files are copied there.`
                          : "Not possible here — see the reasons above."}
                      </span>
                    </span>
                  </label>
                  <label
                    className={
                      "flex items-start gap-2 rounded-lg border px-2.5 py-2 text-label " +
                      (mechanism === "replay" ? "border-accent/60 bg-accent-soft" : "border-hairline") +
                      (target.replay_available ? "" : " opacity-70")
                    }
                  >
                    <input
                      type="radio"
                      name="cross-mechanism"
                      checked={mechanism === "replay"}
                      disabled={!target.replay_available}
                      onChange={() => setMechanism("replay")}
                      aria-describedby="cross-t3-desc"
                      data-testid="cross-switch-t3"
                    />
                    <span>
                      <span className="block font-medium text-ink">Full story, fresh start</span>
                      <span id="cross-t3-desc" className="text-muted">
                        A new session on {machine} starts from the distilled turns (secrets handled as you choose).
                      </span>
                    </span>
                  </label>
                </div>
              </div>
            )}

            {current === "Project" && (
              <div className="flex flex-col gap-2" data-testid="cross-switch-project">
                <p className="text-label text-ink">
                  The project isn't at the same path on {machine}. Pick its copy there — paths inside the
                  conversation are mapped to it only with your confirmation.
                </p>
                {orderedProjects.length === 0 ? (
                  <p className="text-label text-muted" data-testid="cross-switch-no-projects">
                    {machine} knows no projects yet — open the project there once, then try again.
                  </p>
                ) : (
                  <div role="radiogroup" aria-label={`Project on ${machine}`} className="flex flex-col gap-1">
                    {orderedProjects.map((project) => (
                      <label
                        key={project.project_id}
                        className={
                          "flex items-center gap-2 rounded-lg border px-2.5 py-1.5 text-label " +
                          (projectId === project.project_id ? "border-accent/60 bg-accent-soft" : "border-hairline")
                        }
                      >
                        <input
                          type="radio"
                          name="cross-project"
                          checked={projectId === project.project_id}
                          onChange={() => {
                            setProjectId(project.project_id);
                            setConfirmed(false);
                          }}
                          data-testid="cross-switch-project-option"
                        />
                        <span className="font-medium text-ink">{project.name}</span>
                        {project.same_name && <span className="text-muted">— same name (suggested)</span>}
                      </label>
                    ))}
                  </div>
                )}
                {projectId !== null && mapped && (
                  <label className="flex items-start gap-2 text-label text-ink">
                    <input
                      type="checkbox"
                      checked={confirmed}
                      onChange={(event) => setConfirmed(event.target.checked)}
                      data-testid="cross-switch-confirm-mapping"
                    />
                    <span>
                      Continue in <span className="font-medium">{projectName}</span> on {machine} — this
                      conversation's paths are mapped to it.
                    </span>
                  </label>
                )}
              </div>
            )}

            {current === "What's carried" && (
              <div className="flex flex-col gap-2 text-label" data-testid="cross-switch-carry">
                {target.carried.length > 0 && (
                  <p className="text-ink">
                    <span className="font-medium">Carries:</span> {target.carried.join(", ")}
                  </p>
                )}
                {target.regenerated.length > 0 && (
                  <p className="text-ink">
                    <span className="font-medium">Rebuilt there:</span> {target.regenerated.join(", ")}
                  </p>
                )}
                <p className="text-ink" data-testid="cross-switch-drops">
                  <span className="font-medium">Will drop:</span>{" "}
                  {target.dropped.length > 0 ? target.dropped.join(", ") : "nothing"}
                </p>
                {mechanism === "transplant" ? (
                  <p className="text-muted">
                    The conversation's files are copied to {machine}; this Mac keeps its own copy unchanged.
                  </p>
                ) : (
                  <div role="radiogroup" aria-label="Keys and passwords in this conversation" className="flex flex-col gap-1">
                    <p className="text-ink">Keys and passwords in this conversation:</p>
                    {(
                      [
                        ["strip", "Hide them (recommended)", "They're replaced with ••• and never leave this Mac. If the other session needs a key, it asks you."],
                        ["keep", "Leave them in", `They travel to ${machine} exactly as written. Use this only if the work needs them.`],
                      ] as const
                    ).map(([value, title, body]) => (
                      <label
                        key={value}
                        className={
                          "flex items-start gap-2 rounded-lg border px-2.5 py-1.5 " +
                          (secrets === value ? "border-accent/60 bg-accent-soft" : "border-hairline")
                        }
                      >
                        <input
                          type="radio"
                          name="cross-secrets"
                          checked={secrets === value}
                          onChange={() => setSecrets(value)}
                          data-testid={`cross-switch-secrets-${value}`}
                        />
                        <span>
                          <span className="block font-medium text-ink">{title}</span>
                          <span className="text-muted">{body}</span>
                        </span>
                      </label>
                    ))}
                  </div>
                )}
              </div>
            )}

            {current === "Confirm" && (
              <div className="flex flex-col gap-2 text-label" data-testid="cross-switch-confirm">
                <p className="text-ink">
                  {mechanism === "transplant" ? "Same conversation" : "Full story, fresh start"} under{" "}
                  {target.account_label} in {projectName} on {machine}.
                  {target.dropped.length > 0 ? ` Drops: ${target.dropped.join(", ")}.` : ""}
                </p>
                <div role="radiogroup" aria-label="This session" className="flex flex-col gap-1">
                  {(
                    [
                      ["finish-turn", "Let the current turn finish, then switch"],
                      ["stop-now", "Stop now and switch"],
                    ] as const
                  ).map(([value, title]) => (
                    <label key={value} className="flex items-center gap-2 text-ink">
                      <input
                        type="radio"
                        name="cross-freeze"
                        checked={freeze === value}
                        onChange={() => setFreeze(value)}
                      />
                      {title}
                    </label>
                  ))}
                </div>
              </div>
            )}

            <div className="flex items-center justify-end gap-2">
              <button
                type="button"
                onClick={() => (step === 0 ? onClose() : setStep(step - 1))}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
                data-testid="cross-switch-back"
              >
                {step === 0 ? "Cancel" : "Back"}
              </button>
              {current !== "Confirm" ? (
                <button
                  type="button"
                  disabled={!stepReady}
                  onClick={() => setStep(step + 1)}
                  className={
                    "rounded-lg border px-3 py-1.5 text-xs " +
                    (stepReady ? "border-accent/40 bg-accent-soft text-accent-hi" : "border-hairline text-muted")
                  }
                  data-testid="cross-switch-next"
                >
                  Next
                </button>
              ) : (
                <button
                  type="button"
                  disabled={!armed}
                  onClick={() => void execute()}
                  className={
                    "rounded-lg border px-3 py-1.5 text-xs " +
                    (armed ? "border-accent/40 bg-accent-soft text-accent-hi" : "border-hairline text-muted")
                  }
                  data-testid="cross-switch-go"
                >
                  Switch to {machine}…
                </button>
              )}
            </div>
          </>
        )}

        {phase.kind === "pushing" && (
          <div className="flex flex-col gap-2" data-testid="cross-switch-pushing" aria-live="polite">
            <p className="text-label text-ink">
              {phase.progress?.phase === "installing"
                ? `${machine} is installing and starting it…`
                : `Sending to ${machine}…`}
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
                  <div
                    className="h-full bg-accent"
                    style={{
                      width: `${Math.min(100, (phase.progress.received_bytes / phase.progress.total_bytes) * 100)}%`,
                    }}
                  />
                </div>
                <p className="text-label text-muted">
                  {byteSize(phase.progress.received_bytes)} of {byteSize(phase.progress.total_bytes)}
                </p>
              </>
            )}
            <div className="flex justify-end">
              <button
                type="button"
                onClick={() => void fleetSwitchCancel(target.machine_id)}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
                data-testid="cross-switch-cancel"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {phase.kind === "done" && (
          <div className="flex flex-col gap-2" data-testid="cross-switch-done" aria-live="polite">
            <p className="text-label text-ink">
              <span aria-hidden="true">✓ </span>
              {phase.outcome.message} — on {machine}. This session rests here as handed over.
            </p>
            <div className="flex justify-end">
              <button
                type="button"
                onClick={onClose}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
              >
                Close
              </button>
            </div>
          </div>
        )}

        {phase.kind === "failed" && (
          <div className="flex flex-col gap-2" data-testid="cross-switch-failed" role="alert">
            <p className="text-label text-ink">
              <span aria-hidden="true">✗ </span>
              {phase.message}
            </p>
            <p className="text-label text-muted">Nothing changed on this Mac.</p>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setPhase({ kind: "wizard" })}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
              >
                Back
              </button>
              <button
                type="button"
                onClick={onClose}
                className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
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
