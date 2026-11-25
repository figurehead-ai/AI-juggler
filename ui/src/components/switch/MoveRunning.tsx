/**
 * 0.4.9: Move to account while it runs and when it stops (the maintainer's
 * 406 MB move: refused with a path in its words, then a dialog that
 * wouldn't close).
 *
 * - **Running**: one line saying what it is doing now (waiting for a
 *   profile to finish closing, for the files to hold still, backing up,
 *   moving, starting again, checking), and a bar while there are bytes to
 *   count. A long backup never looks frozen.
 * - **Stopped with nothing changed**: said calmly (an info Alert under
 *   "Nothing was moved"), the paths only in Copy details. A move left part
 *   way keeps the warning look and offers Undo of that very move (its id
 *   from the error, AJ-E2319).
 * - **AJ-E2319**: what holds the files is named; for a stop only the person
 *   can clear, each app holding them has its door ("Go to Claude app · Work").
 */
import { useState, type JSX } from "react";
import { stopOf, type MoveProgressInfo, type MoveStopInfo, type MoveUndoneInfo } from "../../lib/ipc/switch-move";
import { HolderDoors } from "./HolderDoors";
import { ProgressBar } from "../UpdateProgress";
import { Alert } from "../Alert";
import { FailedStep, failureOf } from "./FailedStep";
import { StepList } from "./StepParts";
import { MoveUndo, MoveUndone } from "./MoveDone";
import { sentence } from "./MoveScreen";

/** Whether a move's failure left both accounts as they were (a refusal):
 * said calmly. */
function nothingMoved(cause: unknown): boolean {
  const code = cause !== null && typeof cause === "object" ? (cause as { code?: unknown }).code : null;
  return code === "refused" || code === "move_refused";
}

/** AJ-E2319: what a stopped move offers next, by its class: Retry only for
 * one that can clear by itself and changed nothing; Check again (reads the
 * offer, quits nothing) for one only the person can clear; Copy for a setup
 * that can't move; none for a bug (Copy details and Report are there). A
 * move left part way offers Undo instead. */
export type NextStep = "retry" | "check" | "copy" | "none";

export function nextStep(cause: unknown): NextStep {
  const stop = stopOf(cause);
  if (!stop) return nothingMoved(cause) && failureOf(cause).retryable ? "retry" : "none";
  if (!stop.unchanged) return "none";
  return ({ transient: "retry", needs_you: "check", setup: "copy", bug: "none" } as const)[stop.class];
}

/** The move a failure left part way, by the error's own id; never a guess
 * from the moves list (it could be another conversation's). */
export function leftPartWay(cause: unknown): string | null {
  const stop = stopOf(cause);
  return stop && !stop.unchanged ? stop.move_id : null;
}

/** What holds the files, each with its door when only the person can
 * clear it. */
function Holders({ stop }: { stop: MoveStopInfo }): JSX.Element | null {
  if (stop.holders.length === 0) return null;
  return (
    <div className="flex flex-col gap-2" data-testid="switch-move-holders">
      <p className="text-body text-ink">
        <span className="text-muted">Holding the files: </span>
        {stop.holders.map((holder) => holder.name).join(", ")}
      </p>
      {stop.class === "needs_you" && <HolderDoors holders={stop.holders} testId="switch-move-holder-doors" />}
    </div>
  );
}

/** How far the step is, in percent; null when it has nothing to count. */
function percentOf(progress: MoveProgressInfo | null): number | null {
  if (!progress || progress.total <= 0) return null;
  return Math.min(100, Math.floor((progress.done / progress.total) * 100));
}

/** §8's three steps, and where each of the backend's stages falls. */
const STEPS = ["Quit both apps", "Move the conversation", "Reopen both"] as const;
const STEP_OF: Record<string, number> = { closing: 0, still: 0, backing_up: 1, moving: 1, starting: 2, checking: 2 };

/** While the move runs (DLG-10): what it does now and how far, on the
 * tinted panel, then the three steps, done, now and next. */
export function Moving({ target, holding, progress }: { target: string; holding: boolean; progress: MoveProgressInfo | null }): JSX.Element {
  const percent = percentOf(progress);
  const at = STEP_OF[progress?.stage ?? ""] ?? 0;
  return (
    <div className="flex flex-col gap-4" data-testid="switch-moving" data-stage={progress?.stage ?? "starting"}>
      <h3 className="type-section">Moving to {target}…</h3>
      <div className="flex flex-col gap-2 rounded-card bg-accent-soft px-4 py-3.5">
        <p className="flex items-baseline gap-2 text-body font-semibold text-ink">
          <span aria-live="polite" data-testid="switch-moving-now">
            {progress ? progress.words : `Moving to ${target}: both profiles quit, the conversation moves, and both start again.`}
          </span>
          {percent != null && (
            <span className="ml-auto shrink-0 text-meta font-normal tabular-nums text-ink-2" data-testid="switch-moving-percent">
              {percent}%
            </span>
          )}
        </p>
        {percent != null && <ProgressBar label="Backing up" percent={percent} testId="switch-moving-bar" />}
      </div>
      <StepList steps={STEPS} at={at} />
      {holding && (
        <Alert level="info" live="status">
          It can&apos;t be stopped half way, so it finishes first.
        </Alert>
      )}
    </div>
  );
}

/** A move that stopped: calm when nothing changed, else the warning and
 * Undo for the move left part way (`left`). */
/** A refusal's words under the heading "Nothing was moved": its own
 * "…, so nothing was moved" left out, so it isn't said twice (CNT-5). */
function withoutNothingMoved(message: string): string {
  return message.replace(/\s*[,;]\s*(so\s+)?nothing (was|has been) moved(?=[.;,]|$)/gi, "");
}

export function MoveFailed({ target, message, cause, left, source = null }: { target: string; message: string; cause: unknown; left: string | null; source?: string | null }): JSX.Element {
  const calm = nothingMoved(cause) && !left;
  const stop = stopOf(cause);
  const [undone, setUndone] = useState<MoveUndoneInfo | null>(null);
  // R4: undone, the move's failure is behind it: its own end state.
  if (undone) return <MoveUndone undone={undone} source={source} />;
  return (
    <div className="flex flex-col gap-4">
      <FailedStep target={target} message={sentence(calm ? withoutNothingMoved(message) : message)} cause={cause} results={[]} calm={calm ? { title: "Nothing was moved" } : undefined} />
      {stop && <Holders stop={stop} />}
      {left && <MoveUndo moveId={left} onUndone={setUndone} />}
    </div>
  );
}
