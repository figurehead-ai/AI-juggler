/**
 * The switch dialog's simpler bodies (SwitchFlow holds the flow): the
 * Target step, the Conflicts and Rules steps, and the stages while the
 * switch runs. 0.5.0 (D-142): each in rich mode, its facts on the tinted
 * summary panel, its alerts at their level, never a grey paragraph.
 */
import { useEffect, useState, type JSX } from "react";
import { onSwitchProgress, type PreflightInfoData } from "../../lib/ipc/switch-preflight";
import { moveBadge, stageWords } from "../../lib/switch-words";
import { useTargetApp } from "../../lib/apps";
import { fateOf, originalFate, outsideStays } from "../SwitchFreezeChoice";
import { Alert } from "../Alert";
import { Section } from "../Section";
import { IconComputer, IconPage } from "../icons-actions";
import { IconMessage } from "../icons-canvas";
import { IconCheck } from "../icons-controls";
import { IconGitBranch } from "../icons";
import { IconHistory } from "../icons-places";
import { ProgressBar } from "../UpdateProgress";
import { Fact, FactPanel } from "./Facts";
import { BadgeTag, RoomTag, TargetLogo } from "./TargetCard";
import { RoomFact, RoomNote } from "./TargetRoom";

export { risky, useTargetRoom } from "./TargetRoom";
import type { TargetRoom } from "../../lib/room-targets";

/** Step 1: where it goes (the App, its account, its room, how much
 * carries), then what happens as facts, and only real risks, each said
 * once (0.3.5); where it continues (Desktop or CLI) is picked on the last
 * step, and this one says so. */
export function TargetStep({ preflight, target, targetFolder, room }: { preflight: PreflightInfoData; target: string; targetFolder: string; room: TargetRoom | null }): JSX.Element {
  const app = useTargetApp()({ folder: targetFolder, tool: preflight.target_tool, label: target });
  return (
    <div className="flex flex-col gap-6" data-testid="switch-step-target">
      <Section title="Where it goes">
        <div className="flex min-w-0 items-center gap-3.5 rounded-card bg-surface-2 px-4 py-3.5" data-testid="switch-target-card">
          <TargetLogo tool={preflight.target_tool} kind={app.kind} place={app.mark} folderId={targetFolder} label={target} />
          <span className="flex min-w-0 flex-1 flex-col gap-0.5">
            <span className="truncate text-body font-semibold text-ink">{app.name}</span>
            <span className="truncate text-meta text-ink-2">On {room?.account ?? target}</span>
            <span className="mt-1 flex flex-wrap items-center gap-1.5">
              <RoomTag room={room} />
              <BadgeTag badge={moveBadge(preflight)} validated={preflight.validated === true} />
            </span>
          </span>
        </div>
      </Section>
      <RoomNote room={room} target={target} />
      <Section title="What happens">
        <FactPanel testId="switch-target-facts" label="What happens">
          <Fact icon={IconMessage} label="Carries">
            <span data-testid="switch-move-line">{preflight.move_line}</span>
          </Fact>
          {/* What happens to the original, said once, in the words the
              confirm and done screens use. */}
          <Fact icon={IconHistory} label="Original">
            <span data-testid={preflight.source_outside ? "switch-outside-target" : "switch-target-original"}>
              {preflight.source_outside ? outsideStays(preflight.source_app, preflight.source_live) : originalFate("before", fateOf(preflight))}
            </span>
          </Fact>
          <RoomFact room={room} target={target} />
          <Fact icon={IconComputer} label="Where">
            <span data-testid="switch-target-next">Nothing changes until the last step, where you pick Desktop or CLI and switch.</span>
          </Fact>
        </FactPanel>
      </Section>
      {preflight.wizard_reasons.length > 0 && (
        <Alert level="info" live={null} title="Why it asks first" stack testId="switch-wizard-reasons">
          <ul className="mt-1 flex list-disc flex-col gap-0.5 pl-4">
            {preflight.wizard_reasons.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
        </Alert>
      )}
    </div>
  );
}

/** The Conflicts and Rules steps (shown only when they apply, §8): what
 * happens, as a fact, and that nothing merges unseen. */
export function OtherStep({ step }: { step: "Conflicts" | "Rules & diff" }): JSX.Element {
  const conflicts = step === "Conflicts";
  return (
    <div className="flex flex-col gap-4" data-testid={conflicts ? "switch-step-conflicts" : "switch-step-rules"}>
      <Section title={conflicts ? "Both sides changed" : "Rules and changes differ"} as="h3">
        <FactPanel label={conflicts ? "Both sides changed" : "Rules and changes differ"}>
          {conflicts ? (
            <Fact icon={IconGitBranch} label="Keeps">
              The original's version. AI Juggler can't merge the two versions yet, so it records where they split.
            </Fact>
          ) : (
            <Fact icon={IconPage} label="Carries">
              A copy of the changes. The project's rules or its uncommitted changes differ between the two apps, and the difference is recorded.
            </Fact>
          )}
        </FactPanel>
      </Section>
    </div>
  );
}

/** While it runs: the stages, NAMED (never a bare spinner), none claiming
 * completion the engine hasn't reported — the one command reports only its
 * end; and, after an Esc, that it's finishing (§4.9). `app`: the desktop
 * app it opens in, when it continues on Desktop. */
/** The pipeline's step words (`switch://progress`) to the list's place:
 * 0 is pausing the original, done before the copy starts. */
export const STEP_AT: Record<string, number> = { copying: 1, checking: 2, starting: 3, confirming: 4 };

/** 0.3.9: where the switch is, as the pipeline says it, and how long it's
 * taken (a large conversation's copy took seconds with nothing moving). */
export function useSwitchStep(): { at: number; seconds: number } {
  const [at, setAt] = useState(1);
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    let off: (() => void) | null = null;
    let alive = true;
    void onSwitchProgress((stage) => setAt((was) => Math.max(was, STEP_AT[stage] ?? was))).then(
      (unlisten) => (alive ? (off = unlisten) : unlisten()),
      () => undefined,
    );
    const started = Date.now();
    const tick = window.setInterval(() => setSeconds(Math.floor((Date.now() - started) / 1000)), 1000);
    return () => {
      alive = false;
      off?.();
      window.clearInterval(tick);
    };
  }, []);
  return { at, seconds };
}

export function Executing({ target, holding, app = null }: { target: string; holding: boolean; app?: string | null }): JSX.Element {
  const { at, seconds } = useSwitchStep();
  const stages = stageWords(target, app);
  const percent = Math.min(100, Math.round(((at + 0.5) / stages.length) * 100));
  return (
    // DLG-10: the stage it's at, a bar, and the steps done and to come.
    <div className="flex flex-col gap-4" aria-live="polite" data-testid="switch-executing">
      <h3 className="type-section">Switching to {target}…</h3>
      <div className="flex flex-col gap-2 rounded-card bg-accent-soft px-4 py-3.5 inset-ring-1 inset-ring-accent/15">
        <p className="flex items-baseline gap-2 text-body font-semibold text-ink" data-testid="switch-stage-now">
          {stages[Math.min(at, stages.length - 1)]}…
          <span className="ml-auto shrink-0 text-meta font-normal tabular-nums text-ink-2">
            Step {Math.min(at, stages.length - 1) + 1} of {stages.length}
          </span>
        </p>
        <ProgressBar label="Switching" percent={percent} testId="switch-progress" />
      </div>
      <ol className="flex flex-col gap-1.5 text-body">
        {stages.map((stage, index) => (
          <li key={stage} className={"flex items-center gap-2.5 " + (index < at ? "text-ink-2" : index === at ? "font-semibold text-ink" : "text-muted")} data-testid="switch-stage" data-state={index < at ? "done" : index === at ? "now" : "next"}>
            <span aria-hidden="true" className="flex size-5 shrink-0 items-center justify-center">
              {index < at ? (
                <IconCheck width={14} height={14} strokeWidth={2.5} className="text-success" />
              ) : index === at ? (
                <span className="inline-block size-3.5 rounded-full border-2 border-accent border-t-transparent motion-safe:animate-spin" />
              ) : (
                <span className="size-2 rounded-full inset-ring-[1.5px] inset-ring-faint" />
              )}
            </span>
            {stage}
          </li>
        ))}
      </ol>
      <p className="text-body text-ink-2" data-testid="switch-elapsed">
        {seconds >= 3 ? `${seconds} s so far. A long conversation takes a little longer to copy. ` : ""}The original stays safe until the new session is running.
      </p>
      {holding && (
        <Alert level="info" live="status" testId="switch-holding">
          Finishing copy… this closes by itself when it's done.
        </Alert>
      )}
    </div>
  );
}
