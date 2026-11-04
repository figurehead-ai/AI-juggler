/**
 * The switch dialog's simpler bodies (SwitchFlow holds the flow): the
 * Target step, the Conflicts and Rules steps, and the stages while the
 * switch runs.
 */
import { useEffect, useState, type JSX } from "react";
import { listen } from "../../lib/ipc/transport";
import type { PreflightInfoData } from "../../lib/bridge";
import { moveBadge, stageWords } from "../../lib/switch-words";
import { outsideStays } from "../SwitchFreezeChoice";
import { FidelityBadge } from "../SwitchTargetRows";
import { IconAlert } from "../icons";

import { RoomNote } from "./TargetRoom";

export { risky, useTargetRoom } from "./TargetRoom";
import type { TargetRoom } from "../../lib/room-targets";

/** Step 1: where it goes, what goes along, its room, and only real
 * risks, each said once and short (0.3.5); where it continues (Desktop or
 * CLI) is picked on the last step, and this one says so. */
export function TargetStep({ preflight, target, room }: { preflight: PreflightInfoData; target: string; room: TargetRoom | null }): JSX.Element {
  return (
    <div className="flex flex-col gap-3" data-testid="switch-step-target">
      <div className="flex flex-col gap-1.5">
        <h3 className="text-title text-ink">Switch to {target}</h3>
        <p className="text-meta">
          <FidelityBadge badge={moveBadge(preflight)} validated={preflight.validated === true} />
        </p>
      </div>
      <p className="text-body text-ink" data-testid="switch-move-line">
        {preflight.move_line}
        {/* The move line says what goes along; where the original stays is
            said once, here, or by the outside line below. */}
        {!preflight.source_outside && " The original stays as it was."}
      </p>
      {preflight.source_outside && (
        <p className="text-body text-ink" data-testid="switch-outside-target">
          {outsideStays(preflight.source_app, preflight.source_live)}
        </p>
      )}
      <RoomNote room={room} target={target} />
      {preflight.wizard_reasons.length > 0 && (
        <ul className="flex flex-col gap-1.5" data-testid="switch-wizard-reasons">
          {preflight.wizard_reasons.map((reason) => (
            <li key={reason} className="flex items-start gap-2 rounded-lg border border-warning/35 bg-warning-soft px-3 py-1.5 text-meta text-ink">
              <IconAlert aria-hidden="true" width={14} height={14} className="mt-0.5 shrink-0 text-warning" />
              {reason}
            </li>
          ))}
        </ul>
      )}
      <p className="text-meta text-muted" data-testid="switch-target-next">
        Nothing changes until the last step, where you pick Desktop or CLI and switch.
      </p>
    </div>
  );
}

/** The Conflicts and Rules steps (shown only when they apply, §8). */
export function OtherStep({ step }: { step: "Conflicts" | "Rules & diff" }): JSX.Element {
  return step === "Conflicts" ? (
    <div className="flex flex-col gap-2" data-testid="switch-step-conflicts">
      <h3 className="text-title text-ink">Both sides changed</h3>
      <p className="text-body text-ink">
        AI Juggler can't merge the two versions yet, so this switch keeps the original's version and records where they split.
      </p>
    </div>
  ) : (
    <div className="flex flex-col gap-2" data-testid="switch-step-rules">
      <h3 className="text-title text-ink">Rules and changes differ</h3>
      <p className="text-body text-ink">
        The project's rules or its uncommitted changes differ between the two accounts. A copy of the changes goes along, and the difference is recorded.
      </p>
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
    void listen<string>("switch://progress", (event) => setAt((was) => Math.max(was, STEP_AT[event.payload] ?? was))).then(
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
  return (
    <div className="flex flex-col gap-3" aria-live="polite" data-testid="switch-executing">
      <h3 className="text-title text-ink">Switching to {target}…</h3>
      <div className="h-1.5 overflow-hidden rounded-full bg-surface-3" role="progressbar" aria-label="Switching" aria-valuemin={0} aria-valuemax={stages.length} aria-valuenow={at} data-testid="switch-progress">
        <div className="h-full rounded-full bg-accent transition-[width] duration-500" style={{ width: `${((at + 0.5) / stages.length) * 100}%` }} />
      </div>
      <ol className="flex flex-col gap-1 text-body">
        {stages.map((stage, index) => (
          <li key={stage} className={"flex items-center gap-2 " + (index < at ? "text-ink-2" : index === at ? "font-semibold text-ink" : "text-muted")} data-testid="switch-stage" data-state={index < at ? "done" : index === at ? "now" : "next"}>
            <span aria-hidden="true" className="w-4 shrink-0 text-center">
              {index < at ? "✓" : index === at ? <span className="inline-block size-3 rounded-full border-2 border-accent border-t-transparent motion-safe:animate-spin" /> : "·"}
            </span>
            {stage}
          </li>
        ))}
      </ol>
      <p className="text-body text-ink" data-testid="switch-elapsed">
        {seconds >= 3 ? `${seconds} s so far. A long conversation takes a little longer to copy. ` : ""}The original stays safe until the new session is running.
      </p>
      {holding && (
        <p className="text-body font-semibold text-ink" role="status" data-testid="switch-holding">
          Finishing copy… this closes by itself when it's done.
        </p>
      )}
    </div>
  );
}
