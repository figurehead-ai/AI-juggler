/**
 * The switch dialog's simpler bodies (SwitchFlow holds the flow): the
 * Target step, the Conflicts and Rules steps, and the stages while the
 * switch runs.
 */
import type { JSX } from "react";
import type { PreflightInfoData } from "../../lib/bridge";
import { badgeWords, stageWords } from "../../lib/switch-words";
import { outsideWords } from "../SwitchFreezeChoice";
import { risky, RoomNote } from "./TargetRoom";

export { risky, useTargetRoom } from "./TargetRoom";
import type { TargetRoom } from "../../lib/room-targets";

/** Step 1: where it goes, what goes along, its room, and only real risks. */
export function TargetStep({ preflight, target, room }: { preflight: PreflightInfoData; target: string; room: TargetRoom | null }): JSX.Element {
  return (
    <div className="flex flex-col gap-3" data-testid="switch-step-target">
      <h3 className="text-title text-ink">Switch to {target}</h3>
      <p className="text-meta text-ink">
        <span className="rounded-md border border-accent/40 px-1.5 py-0.5">{badgeWords(preflight.fidelity_words, preflight.validated)}</span>
      </p>
      <p className="text-body text-ink" data-testid="switch-move-line">
        {preflight.move_line}
      </p>
      <RoomNote room={room} target={target} />
      {preflight.wizard_reasons.length > 0 && (
        <ul className="flex flex-col gap-1.5" data-testid="switch-wizard-reasons">
          {preflight.wizard_reasons.map((reason) => (
            <li key={reason} className="rounded-lg border border-warning/40 px-3 py-2 text-body text-warning">
              {reason}
            </li>
          ))}
        </ul>
      )}
      {preflight.source_outside && (
        <p className="text-body text-ink" data-testid="switch-outside-target">
          {outsideWords(preflight.source_app, preflight.source_live, target)}
        </p>
      )}
      <p className="text-meta text-ink">Nothing happens until you choose {risky(room) ? "Switch anyway" : "Switch now"} on the last step.</p>
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
 * end; and, after an Esc, that it's finishing (§4.9). */
export function Executing({ target, holding }: { target: string; holding: boolean }): JSX.Element {
  return (
    <div className="flex flex-col gap-3" aria-live="polite" data-testid="switch-executing">
      <h3 className="text-title text-ink">Switching to {target}…</h3>
      <ol className="flex list-decimal flex-col gap-1 pl-5 text-body text-ink">
        {stageWords(target).map((stage) => (
          <li key={stage}>{stage}</li>
        ))}
      </ol>
      <p className="text-body text-ink" data-testid="switch-elapsed">
        The original stays safe until the new session is running.
      </p>
      {holding && (
        <p className="text-body font-semibold text-ink" role="status" data-testid="switch-holding">
          Finishing copy… this closes by itself when it's done.
        </p>
      )}
    </div>
  );
}
