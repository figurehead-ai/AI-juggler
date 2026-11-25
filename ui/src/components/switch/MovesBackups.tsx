/**
 * AJ-E2325 — the backups kept for Undo, together. Settings › Desktop apps
 * › Moves shows the moves in rich mode: a sticky header (how many moves
 * and backups) over the rows and a sticky footer with the total and, when
 * the disk is nearly full or a move was refused for room, the offer to
 * clear backups older than a day, naming what it frees. Only the person's
 * click on the offer's sheet clears anything. The Processes page's clean-up
 * view says the backups the header's "N to clean up" counts, and opens
 * the Moves section.
 */
import { useState, type JSX } from "react";
import { Modal } from "../form";
import { Button } from "../primitives";
import { errorText } from "../../lib/bridge";
import { switchMoveClearOld, type MoveBackupsInfo, type MoveRecordInfo } from "../../lib/ipc/switch-move";
import { useMoveBackups } from "../../lib/move-backups";
import { MoveRows, freesWords } from "./MovesDoor";

/** "2 move backups" / "1 move backup". */
function backupsWords(count: number): string {
  return `${count} move backup${count === 1 ? "" : "s"}`;
}

function ClearOldSheet({ info, busy, onCancel, onConfirm }: { info: MoveBackupsInfo; busy: boolean; onCancel: () => void; onConfirm: () => void }): JSX.Element {
  const many = info.old_count !== 1;
  return (
    <Modal
      title={`Clear ${backupsWords(info.old_count)} older than a day?`}
      subtitle={`${many ? "Those moves" : "That move"} can no longer be undone.`}
      onClose={onCancel}
      testId="moves-clear-sheet"
      footer={
        <>
          <Button size="sm" variant="secondary" onClick={onCancel} data-autofocus>
            Keep
          </Button>
          <Button size="sm" variant="danger" disabled={busy} onClick={onConfirm} data-testid="moves-clear-confirm">
            {busy ? "Clearing…" : "Clear backups"}
          </Button>
        </>
      }
    >
      <p className="text-body text-ink-2" data-modal-body>
        {`It ${freesWords(info.old_bytes, info.exact)}. The conversations stay where they moved; newer backups and moves stopped part way are kept.`}
      </p>
    </Modal>
  );
}

/** The footer: the total, and the low-disk offer when room is short. */
function BackupsFoot({ info, onChanged }: { info: MoveBackupsInfo; onChanged: () => void }): JSX.Element {
  const [asking, setAsking] = useState(false);
  const [busy, setBusy] = useState(false);
  const [said, setSaid] = useState<{ words: string; failed: boolean } | null>(null);
  // F12 (D-127): an offer that frees nothing (untouched clones) isn't shown.
  const offer = info.low_room && info.old_count > 0 && info.old_bytes > 0;
  const clear = () => {
    setBusy(true);
    switchMoveClearOld()
      .then(
        (words) => setSaid({ words, failed: false }),
        (cause) => setSaid({ words: errorText(cause), failed: true }),
      )
      .finally(() => {
        setBusy(false);
        setAsking(false);
        onChanged();
      });
  };
  return (
    <div className="sticky bottom-0 z-20 flex flex-col gap-1.5 bg-surface px-4 py-2.5 shadow-[0_-1px_0_var(--color-hairline)]" data-testid="moves-foot">
      <span className="text-meta text-ink-2" data-testid="moves-total">
        {info.count === 0 ? "No backups kept" : `${backupsWords(info.count)} kept for Undo: ${freesWords(info.bytes, info.exact)} in all`}
      </span>
      {offer && (
        <div className="flex flex-wrap items-center gap-2" data-testid="moves-offer">
          <span className="text-meta text-ink">{`The disk is nearly full. Backups older than a day: ${freesWords(info.old_bytes, info.exact)}.`}</span>
          <Button size="sm" variant="secondary" disabled={busy} onClick={() => setAsking(true)} data-testid="moves-clear">
            Clear older than a day…
          </Button>
        </div>
      )}
      {said && (
        <output className={`text-meta ${said.failed ? "text-warning" : "text-ink-2"}`} data-testid="moves-cleared">
          {said.words}
        </output>
      )}
      {asking && <ClearOldSheet info={info} busy={busy} onCancel={() => setAsking(false)} onConfirm={clear} />}
    </div>
  );
}

/** Settings' Moves, rich: a sticky header, the rows, a sticky footer. */
export function MovesPanel({ moves, onChanged }: { moves: MoveRecordInfo[]; onChanged: () => void }): JSX.Element {
  const info = useMoveBackups();
  return (
    <div className="max-h-[32rem] overflow-y-auto" data-testid="moves-panel">
      <div className="sticky top-0 z-20 flex items-baseline justify-between gap-3 bg-surface px-4 py-2.5 shadow-[0_1px_0_var(--color-hairline)]" data-testid="moves-head">
        <span className="text-body font-medium text-ink">{`${moves.length} move${moves.length === 1 ? "" : "s"}`}</span>
        {info && info.count > 0 && <span className="text-meta text-ink-2">{backupsWords(info.count)}</span>}
      </div>
      <div className="px-4">
        <MoveRows moves={moves} onChanged={onChanged} />
      </div>
      {info && <BackupsFoot info={info} onChanged={onChanged} />}
    </div>
  );
}

/** The clean-up view's line for the backups the header counts: how many,
 * what they free, and the way to them. None kept: nothing. */
export function BackupsCleanupRow(): JSX.Element | null {
  const info = useMoveBackups();
  if (!info || info.count === 0) return null;
  const open = () => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "moves" } }));
  return (
    <div className="flex flex-wrap items-center gap-2 text-meta text-ink-2" data-testid="cleanup-backups">
      <span>{`Also to clean up: ${backupsWords(info.count)} kept for Undo, ${freesWords(info.bytes, info.exact)}.`}</span>
      <Button size="sm" variant="ghost" onClick={open} data-testid="cleanup-backups-open">
        Review backups
      </Button>
    </div>
  );
}
