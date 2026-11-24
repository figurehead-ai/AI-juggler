/**
 * ADR-0031 (D-93): Move to account, a switch by move (one thin file per
 * feature). In the preview, the seams `__AIJ_MOVE_OFFER__` (the offer;
 * none by default, so the switch screens the gallery shows stay Copy),
 * `__AIJ_MOVE_DONE__` / `__AIJ_MOVE_ERROR__` (what a move returns),
 * `__AIJ_MOVE_PROGRESS__` (how it goes, told before it returns),
 * `__AIJ_MOVE_HOLD__` (it never returns) and `__AIJ_MOVE_UNDONE__` stand
 * in.
 */
import { call, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import { osPromptRequest } from "./os-prompts";
import type { MoveBackupsInfo } from "../../types/generated/MoveBackupsInfo";
import type { MoveDoneInfo } from "../../types/generated/MoveDoneInfo";
import type { MoveFinishedInfo } from "../../types/generated/MoveFinishedInfo";
import type { MoveOfferInfo } from "../../types/generated/MoveOfferInfo";
import type { MoveProgressInfo } from "../../types/generated/MoveProgressInfo";
import type { MoveRecordInfo } from "../../types/generated/MoveRecordInfo";
import type { MoveStopInfo } from "../../types/generated/MoveStopInfo";
import type { MoveUndoneInfo } from "../../types/generated/MoveUndoneInfo";

export type { MoveBackupsInfo, MoveDoneInfo, MoveFinishedInfo, MoveOfferInfo, MoveProgressInfo, MoveRecordInfo, MoveStopInfo, MoveUndoneInfo };

/** 0.4.9: how a running move goes (`move://progress`). */
const MOVE_PROGRESS = "move://progress";

declare global {
  interface Window {
    __AIJ_MOVE_OFFER__?: MoveOfferInfo | null;
    __AIJ_MOVE_DONE__?: MoveDoneInfo;
    __AIJ_MOVE_ERROR__?: string | { code: string; retryable: boolean; user_message: string; detail?: string; stop?: MoveStopInfo };
    __AIJ_MOVE_PROGRESS__?: MoveProgressInfo[];
    __AIJ_MOVE_HOLD__?: boolean;
    __AIJ_MOVE_UNDONE__?: MoveUndoneInfo;
    __AIJ_MOVES__?: MoveRecordInfo[];
    __AIJ_MOVE_FINISHED__?: MoveFinishedInfo;
    __AIJ_MOVE_BACKUP_DELETED__?: string;
    __AIJ_MOVE_BACKUPS__?: MoveBackupsInfo;
    __AIJ_MOVE_OLD_CLEARED__?: string;
  }
}

/** Whether and how this conversation can be moved to the target (reads
 * only); null where moving doesn't apply. What stands in its way says
 * whether the user can clear it (D-127; the switch screen decides). */
export async function switchMoveOffer(sessionId: string, targetProfileId: string, targetPlaceId: string | null = null): Promise<MoveOfferInfo | null> {
  if (!hasLiveBackend()) return window.__AIJ_MOVE_OFFER__ ?? null;
  const offer = await call("switch_move_offer", { sessionId, targetProfileId, targetPlaceId });
  return offer.applies ? offer : null;
}

/** The preview's listener (no backend to tell it). */
let previewProgress: ((progress: MoveProgressInfo) => void) | null = null;

/** 0.4.9: each step of a running move as it is told (waiting for an app
 * to close, for the files to hold still, the backup's bytes, moving,
 * starting again). Returns the unlisten. */
export function onMoveProgress(handler: (progress: MoveProgressInfo) => void): () => void {
  if (!hasLiveBackend()) {
    previewProgress = handler;
    return () => {
      if (previewProgress === handler) previewProgress = null;
    };
  }
  let stop: (() => void) | null = null;
  let disposed = false;
  listen<MoveProgressInfo>(MOVE_PROGRESS, (event) => handler(event.payload))
    .then((unlisten) => {
      if (disposed) unlisten();
      else stop = unlisten;
    })
    .catch(() => {});
  return () => {
    disposed = true;
    stop?.();
  };
}

/** Moves the conversation and its family (the user's click) into the App
 * picked (AJ-E2317: its place id, end to end; none: a terminal's folder). */
export function switchMoveExecute(sessionId: string, targetProfileId: string, takeNotes: boolean, targetPlaceId: string | null = null): Promise<MoveDoneInfo> {
  return changingBackups(moveCall(sessionId, targetProfileId, takeNotes, targetPlaceId));
}

async function moveCall(sessionId: string, targetProfileId: string, takeNotes: boolean, targetPlaceId: string | null): Promise<MoveDoneInfo> {
  if (!hasLiveBackend()) {
    for (const step of window.__AIJ_MOVE_PROGRESS__ ?? []) previewProgress?.(step);
    if (window.__AIJ_MOVE_HOLD__) return new Promise<MoveDoneInfo>(() => {});
    const failure = window.__AIJ_MOVE_ERROR__;
    if (failure) throw failure;
    const done = window.__AIJ_MOVE_DONE__;
    if (!done) throw "No move in the preview";
    return done;
  }
  return call("switch_move_execute", { sessionId, targetProfileId, targetPlaceId, takeNotes });
}

/** 0.5.1 F93 (PAT-OSP): a move's (or its Undo's) system prompts, the
 * offer's or the record's `prompts`, each asked through the one sheet in
 * turn before anything quits: the move itself never asks. Resolves once
 * each is answered, Not now included (the move still runs and says how to
 * open the conversation). */
export async function askFirst(prompts: readonly string[] | undefined): Promise<void> {
  for (const id of prompts ?? []) await osPromptRequest(id).catch(() => undefined);
}

/** AJ-E2319: a move's stop, typed (its class, what holds the files, its
 * move's id); null for a failure that isn't a move's. */
export function stopOf(cause: unknown): MoveStopInfo | null {
  if (cause === null || typeof cause !== "object" || !("stop" in cause)) return null;
  return isStop(cause.stop) ? cause.stop : null;
}

function isStop(value: unknown): value is MoveStopInfo {
  return value !== null && typeof value === "object" && "class" in value && "holders" in value && "unchanged" in value;
}

/** Undoes a move (the user's click). */
export async function switchMoveUndo(moveId: string): Promise<MoveUndoneInfo> {
  if (!hasLiveBackend()) return changingBackups(Promise.resolve(window.__AIJ_MOVE_UNDONE__ ?? { words: "Undone: the conversation is back where it was.", kept: "", notes: [] }));
  return changingBackups(call("switch_move_undo", { moveId }));
}

/** The newest moves AI Juggler knows of (the first page, newest first). */
export async function switchMoves(): Promise<MoveRecordInfo[]> {
  if (!hasLiveBackend()) return window.__AIJ_MOVES__ ?? [];
  const page = await call("switch_moves", { page: null });
  return page.items;
}

/** AJ-E2321: finishes an interrupted move that passed its checks (the
 * Moves door's Finish, by its id). */
export async function switchMoveFinish(moveId: string): Promise<MoveFinishedInfo> {
  if (!hasLiveBackend()) return changingBackups(Promise.resolve(window.__AIJ_MOVE_FINISHED__ ?? { words: "Finished: the conversation stays where it moved, checked once more." }));
  return changingBackups(call("switch_move_finish", { moveId }));
}

/** AJ-E2325: deletes a finished move's backup (its Undo goes with it), by
 * its id, after the sheet says so. Returns the words. */
export async function switchMoveDeleteBackup(moveId: string): Promise<string> {
  if (!hasLiveBackend()) return changingBackups(Promise.resolve(window.__AIJ_MOVE_BACKUP_DELETED__ ?? "Backup deleted: this move can no longer be undone."));
  return changingBackups(call("switch_move_delete_backup", { moveId }));
}

/** AJ-E2325: told when backups change (a delete, a clear, an undo), so
 * the header's count and the door's total read again. 0.5.1 F93: told
 * by every call here that makes or lets go of one, whatever it returned
 * (a move commits one; a refused move's attempt goes). */
export const BACKUPS_CHANGED = "aij:move-backups-changed";

/** `call`, then the backups told to read again (0.5.1 F93). */
export function changingBackups<T>(pending: Promise<T>): Promise<T> {
  const told = () => window.dispatchEvent(new CustomEvent(BACKUPS_CHANGED));
  return pending.then(
    (done) => {
      told();
      return done;
    },
    (cause: unknown) => {
      told();
      throw cause;
    },
  );
}

/** AJ-E2325: no backups kept (the preview's default). */
const NO_BACKUPS: MoveBackupsInfo = { count: 0, bytes: 0, exact: true, old_count: 0, old_bytes: 0, low_room: false };

/** AJ-E2325: the backups kept for Undo together: the door's total, the
 * header's count, and whether room is short (the offer). Reads only. */
export async function switchMoveBackups(): Promise<MoveBackupsInfo> {
  if (!hasLiveBackend()) return window.__AIJ_MOVE_BACKUPS__ ?? NO_BACKUPS;
  return call("switch_move_backups", {});
}

/** AJ-E2325: the low-disk offer accepted (the person's click, after its
 * sheet): clears backups of moves made more than a day ago. Returns the
 * words. */
export async function switchMoveClearOld(): Promise<string> {
  if (!hasLiveBackend()) return changingBackups(Promise.resolve(window.__AIJ_MOVE_OLD_CLEARED__ ?? "Cleared 1 backup (up to 1.2 GB); that move can no longer be undone."));
  return changingBackups(call("switch_move_clear_old", {}));
}
