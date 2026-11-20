/**
 * D-62 decision 18 (AJ-E3008 req. 1), AJ-E3011: the empty canvas says
 * what's next in one calm card: add who's in it. The setup's order lives
 * in the inspector's checklist beside it (it never vanishes when members
 * are added, and no card sits over it), so this card teaches only the
 * first move, and offers the guided setup for a pool of yours. Nothing
 * here starts a session.
 *
 * Walk K1 (AJ-E3306, rc24): a project's pool of 500 ended sessions drew no
 * node: who's in it is its sessions (`PoolInfo.members`, those a broad
 * member brings in too) or a member that brings them.
 */
import type { JSX } from "react";

import { Button } from "../../components/primitives";

/** Whether a pool has who's in it (step 2 done): any session in it, or a
 * member that brings sessions in as they start. */
export function hasWhoIsIn(members: number, hasRules: boolean): boolean {
  return members > 0 || hasRules;
}

export function CanvasTeach({
  scope,
  own,
  onAdd,
  onSetup,
  inset,
  quiet = false,
}: {
  /** The setup's checklist beside it holds the view's one primary. */
  quiet?: boolean;
  /** A project's own pool: its setup is its project's, so no guided setup. */
  own: boolean;
  /** Where it works now, in words. */
  scope: string;
  onAdd: () => void;
  /** The guided setup (a pool of yours). */
  onSetup: () => void;
  /** Room left for the open palette. */
  inset: boolean;
}): JSX.Element {
  return (
    // Centred when there's room; scrolls (never clips) on a short canvas.
    <div className={"pointer-events-none absolute inset-0 flex overflow-y-auto p-4" + (inset ? " pl-[16.5rem]" : "")} data-testid="canvas-empty" data-canvas-scroll>
      <div className="pointer-events-auto m-auto flex w-full max-w-md flex-col gap-2.5 rounded-2xl bg-surface p-4 shadow-card">
        <h3 className="text-section font-semibold text-ink">No one in this pool yet</h3>
        <p className="text-meta text-ink-2">Add an App (its sessions in {scope} join as they start) or a session open now. The checklist on the right walks you through the rest, in order.</p>
        <span className="flex flex-wrap items-center gap-2">
          <Button size="sm" variant={quiet ? "secondary" : "primary"} onClick={onAdd} data-testid="canvas-teach-add">
            Add a session or an App
          </Button>
          {!own && (
            <Button size="sm" variant="ghost" onClick={onSetup} data-testid="canvas-teach-template">
              Open the guided setup
            </Button>
          )}
        </span>
      </div>
    </div>
  );
}
