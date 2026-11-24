/**
 * AJ-E2321 — the Moves door: every move of a conversation (its page) or
 * of every conversation (Settings › Desktop apps), each with what can be
 * done to it by its id: Undo… (a committed move, or one a crash left part
 * way), Finish (one that passed its checks before the apps reopened), and
 * Copy details (its timeline). No moves: nothing shown; one left part way
 * is never hidden (each surface shows the rows only when there are any).
 * Undo's sheet says what it restores first. AJ-E2325: a kept backup
 * says until when and what deleting it frees (exact on APFS, D-146; "up
 * to" where the disk can't say); Delete backup… says the move can no
 * longer be undone.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Modal } from "../form";
import { Button } from "../primitives";
import { errorText } from "../../lib/bridge";
import { memorySize } from "../../lib/compact";
import { BACKUPS_CHANGED, switchMoveDeleteBackup, switchMoveFinish, switchMoveUndo, switchMoves, type MoveRecordInfo } from "../../lib/ipc/switch-move";

/** "Nov 22". */
function dayOf(ms: number): string {
  return new Date(ms).toLocaleDateString([], { month: "short", day: "numeric" });
}

/** "“Fix the parser” · Claude app · A → Claude app · B". */
function moveTitle(move: MoveRecordInfo): string {
  return `“${move.title ?? "A conversation"}” · ${move.from} → ${move.to}`;
}

/** Its state in plain words. */
export function stateWords(move: MoveRecordInfo): string {
  if (move.state === "interrupted") return move.can_finish ? "Interrupted: finish or undo" : "Interrupted: undo puts it back";
  if (move.state === "undone") return "Undone";
  return "Moved";
}

/** What Copy details puts on the clipboard: the move, then its timeline. */
export function detailsOf(move: MoveRecordInfo): string {
  return [
    `Move ${move.move_id}: ${moveTitle(move)}`,
    `${new Date(move.at_ms).toISOString()} · ${stateWords(move)}`,
    ...(move.detail ? [move.detail] : []),
    "Timeline:",
    ...move.timeline.map((line, n) => `${n + 1}. ${line}`),
    ...(move.kept.length > 0 ? ["Kept:", ...move.kept] : []),
  ].join("\n");
}

/** The moves (of `conversation`, when given), newest first; `reload`
 * reads them again after an action. */
export function useMoves(conversation: string | null = null): [MoveRecordInfo[] | null, () => void] {
  const [moves, setMoves] = useState<MoveRecordInfo[] | null>(null);
  const reload = useCallback(() => {
    void switchMoves().then(
      (all) => setMoves(conversation === null ? all : all.filter((m) => m.conversation === conversation)),
      () => setMoves([]),
    );
  }, [conversation]);
  useEffect(reload, [reload]);
  return [moves, reload];
}

function UndoSheet({ move, busy, onCancel, onConfirm }: { move: MoveRecordInfo; busy: boolean; onCancel: () => void; onConfirm: () => void }): JSX.Element {
  return (
    <Modal
      title={`Undo the move of “${move.title ?? "this conversation"}”?`}
      subtitle={`It goes back to ${move.from}, as it was on ${dayOf(move.at_ms)}.`}
      onClose={onCancel}
      testId="move-undo-sheet"
      footer={
        <>
          <Button size="sm" variant="secondary" onClick={onCancel} data-autofocus>
            Cancel
          </Button>
          <Button size="sm" variant="primary" disabled={busy} onClick={onConfirm} data-testid="move-undo-confirm">
            {busy ? "Undoing…" : "Undo"}
          </Button>
        </>
      }
    >
      <dl className="flex flex-col gap-3 text-body" data-modal-body>
        <div className="flex flex-col gap-0.5">
          <dt className="text-meta font-semibold text-ink">Kept aside, never deleted</dt>
          <dd className="text-ink-2">Anything done in it since the move; Copy details says where.</dd>
        </div>
        <div className="flex flex-col gap-0.5">
          <dt className="text-meta font-semibold text-ink">Quits and starts again</dt>
          <dd className="text-ink-2">{`${move.from} and ${move.to}, if they're open.`}</dd>
        </div>
      </dl>
    </Modal>
  );
}

/** "frees 7.8 MB" (exact: APFS measured what the backup alone holds,
 * D-146), or "frees up to 7.8 MB" where clones may share blocks the disk
 * can't measure (what deleting gives back is at most that). */
export function freesWords(bytes: number, exact: boolean): string {
  return `frees ${exact ? "" : "up to "}${memorySize(bytes)}`;
}

/** "Backup kept until Nov 30" / "Backup kept until you delete it". */
export function keptWords(move: MoveRecordInfo): string | null {
  if (!move.has_backup) return null;
  return move.kept_until_ms === null ? "Backup kept until you delete it" : `Backup kept until ${dayOf(move.kept_until_ms)}`;
}

function DeleteSheet({ move, busy, onCancel, onConfirm }: { move: MoveRecordInfo; busy: boolean; onCancel: () => void; onConfirm: () => void }): JSX.Element {
  return (
    <Modal
      title={`Delete the backup of “${move.title ?? "this conversation"}”?`}
      subtitle="You can no longer undo this move."
      onClose={onCancel}
      testId="move-delete-sheet"
      footer={
        <>
          <Button size="sm" variant="secondary" onClick={onCancel} data-autofocus>
            Keep
          </Button>
          <Button size="sm" variant="danger" disabled={busy} onClick={onConfirm} data-testid="move-delete-confirm">
            {busy ? "Deleting…" : "Delete backup"}
          </Button>
        </>
      }
    >
      <p className="text-body text-ink-2" data-modal-body>
        The conversation stays in {move.to}; only the copy Undo restores from goes.
      </p>
    </Modal>
  );
}

function MoveRow({ move, onChanged }: { move: MoveRecordInfo; onChanged: () => void }): JSX.Element {
  const [asking, setAsking] = useState<"undo" | "delete" | null>(null);
  const [busy, setBusy] = useState(false);
  const [said, setSaid] = useState<{ words: string; failed: boolean } | null>(null);
  const act = (run: () => Promise<{ words: string }>) => {
    setBusy(true);
    setSaid(null);
    run()
      .then(
        (done) => setSaid({ words: done.words, failed: false }),
        (cause) => setSaid({ words: errorText(cause), failed: true }),
      )
      .finally(() => {
        setBusy(false);
        setAsking(null);
        onChanged();
        window.dispatchEvent(new CustomEvent(BACKUPS_CHANGED));
      });
  };
  const copy = () => void navigator.clipboard?.writeText(detailsOf(move)).catch(() => {});
  return (
    <li className="flex flex-col gap-1.5 py-2.5" data-testid="move-row" data-state={move.state} data-move-id={move.move_id}>
      <span className="text-body font-medium text-ink">{moveTitle(move)}</span>
      <span className="text-meta text-ink-2" data-testid="move-row-state">
        {`${dayOf(move.at_ms)} · ${stateWords(move)}`}
      </span>
      {move.state === "interrupted" && move.detail && <span className="text-meta text-ink-2">{`${move.detail}.`}</span>}
      {move.undo_why && <span className="text-meta text-muted">{`Can't be undone: ${move.undo_why}.`}</span>}
      {keptWords(move) && (
        <span className="text-meta text-ink-2" data-testid="move-kept-until">
          {move.backup_bytes === null ? keptWords(move) : `${keptWords(move)} · ${freesWords(move.backup_bytes, move.backup_exact)}`}
        </span>
      )}
      <div className="flex flex-wrap items-center gap-2">
        {move.can_finish && (
          <Button size="sm" variant="primary" disabled={busy} onClick={() => act(() => switchMoveFinish(move.move_id))} data-testid="move-finish">
            {busy ? "Finishing…" : "Finish"}
          </Button>
        )}
        {move.can_undo && (
          <Button size="sm" variant="secondary" disabled={busy} onClick={() => setAsking("undo")} data-testid="move-undo">
            Undo…
          </Button>
        )}
        {move.has_backup && (
          <Button size="sm" variant="secondary" disabled={busy} onClick={() => setAsking("delete")} data-testid="move-delete">
            Delete backup…
          </Button>
        )}
        <Button size="sm" variant="ghost" onClick={copy} data-testid="move-copy">
          Copy details
        </Button>
      </div>
      {said && (
        <output className={`text-meta ${said.failed ? "text-warning" : "text-ink-2"}`} data-testid="move-said">
          {said.words}
        </output>
      )}
      {asking === "undo" && <UndoSheet move={move} busy={busy} onCancel={() => setAsking(null)} onConfirm={() => act(() => switchMoveUndo(move.move_id))} />}
      {asking === "delete" && <DeleteSheet move={move} busy={busy} onCancel={() => setAsking(null)} onConfirm={() => act(() => switchMoveDeleteBackup(move.move_id).then((words) => ({ words })))} />}
    </li>
  );
}

/** The moves as rows, each with its actions. */
export function MoveRows({ moves, onChanged }: { moves: MoveRecordInfo[]; onChanged: () => void }): JSX.Element {
  return (
    <ul className="flex flex-col divide-y divide-hairline" data-testid="moves-door">
      {moves.map((move) => (
        <MoveRow key={move.move_id} move={move} onChanged={onChanged} />
      ))}
    </ul>
  );
}
