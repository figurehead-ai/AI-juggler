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
 *   way keeps the warning look and offers Undo.
 */
import type { JSX } from "react";
import type { MoveProgressInfo } from "../../lib/ipc/switch-move";
import { ProgressBar } from "../UpdateProgress";
import { FailedStep } from "./FailedStep";
import { MoveUndo } from "./MoveDone";
import { sentence } from "./MoveChoice";

/** Whether a move's failure left both accounts as they were (a refusal):
 * said calmly, and trying again can work. */
export function nothingMoved(cause: unknown): boolean {
  const code = cause !== null && typeof cause === "object" ? (cause as { code?: unknown }).code : null;
  return code === "refused" || code === "move_refused";
}

/** How far the step is, in percent; null when it has nothing to count. */
function percentOf(progress: MoveProgressInfo | null): number | null {
  if (!progress || progress.total <= 0) return null;
  return Math.min(100, Math.floor((progress.done / progress.total) * 100));
}

/** While the move runs: what it does now, and how far, when it counts. */
export function Moving({ target, holding, progress }: { target: string; holding: boolean; progress: MoveProgressInfo | null }): JSX.Element {
  const percent = percentOf(progress);
  return (
    <div className="flex flex-col gap-2" data-testid="switch-moving" data-stage={progress?.stage ?? "starting"}>
      <p className="flex items-baseline gap-2 text-body text-ink">
        <span aria-live="polite" data-testid="switch-moving-now">
          {progress ? progress.words : `Moving to ${target}: both profiles quit, the conversation moves, and both start again.`}
        </span>
        {percent != null && (
          <span className="ml-auto shrink-0 tabular-nums text-meta text-ink-2" data-testid="switch-moving-percent">
            {percent}%
          </span>
        )}
      </p>
      {percent != null && <ProgressBar label="Backing up" percent={percent} testId="switch-moving-bar" />}
      {holding && <p className="text-meta text-muted">It can&apos;t be stopped half way, so it finishes first.</p>}
    </div>
  );
}

/** A move that stopped: calm when nothing changed, else the warning and
 * Undo for the move left part way (`left`). */
export function MoveFailed({ target, message, cause, left }: { target: string; message: string; cause: unknown; left: string | null }): JSX.Element {
  const calm = nothingMoved(cause) && !left;
  return (
    <div className="flex flex-col gap-4">
      <FailedStep target={target} message={sentence(message)} cause={cause} results={[]} calm={calm ? { title: "Nothing was moved" } : undefined} />
      {left && <MoveUndo moveId={left} />}
    </div>
  );
}
