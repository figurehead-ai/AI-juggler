/**
 * AJ-E3511 (D-78): Re-create the office? — a serious choice explained:
 * what changes, what doesn't, an example, and the guide. It acts only
 * after the confirm.
 */
import type { JSX } from "react";
import { Modal } from "../../components/form";
import { Button } from "../../components/primitives";
import { trustOpen } from "../../lib/ipc/trust";

export function RecreateSheet({ example, onCancel, onConfirm }: { example: string | null; onCancel: () => void; onConfirm: () => void }): JSX.Element {
  return (
    <Modal title="Re-create the office?" subtitle="Every area gets a new look and desk layout. The order stays, so every area stays where you know it." onClose={onCancel} testId="floor-recreate-sheet">
      <dl className="flex flex-col gap-3 text-body" data-modal-body>
        <div className="flex flex-col gap-0.5">
          <dt className="text-meta font-semibold text-ink">Changes</dt>
          <dd className="text-ink-2">Each area's floor, walls, desks, corner and layout, chosen again within its tool's style.</dd>
        </div>
        <div className="flex flex-col gap-0.5">
          <dt className="text-meta font-semibold text-ink">Stays</dt>
          <dd className="text-ink-2">Your Apps, sessions, history, accounts and the main window. Where each area is.</dd>
        </div>
      </dl>
      <p className="mt-3 text-meta text-ink-2" data-testid="floor-recreate-example">
        For example: {example ?? "your first App"} keeps its place on floor 1 and keeps its style; its desks may stand in a new pattern, its floor and walls change.
      </p>
      <div className="mt-4 flex flex-wrap items-center gap-2">
        <button type="button" className="mr-auto text-meta text-accent-hi hover:underline" onClick={() => void trustOpen({ kind: "guide", id: "the-floor" })}>
          How the Floor works ↗
        </button>
        <Button size="sm" variant="secondary" onClick={onCancel} data-autofocus>
          Cancel
        </Button>
        <Button size="sm" variant="primary" onClick={onConfirm} data-testid="floor-recreate-confirm">
          Re-create
        </Button>
      </div>
    </Modal>
  );
}
