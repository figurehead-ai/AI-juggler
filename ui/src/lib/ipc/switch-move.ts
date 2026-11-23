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
import type { MoveDoneInfo } from "../../types/generated/MoveDoneInfo";
import type { MoveFinishedInfo } from "../../types/generated/MoveFinishedInfo";
import type { MoveOfferInfo } from "../../types/generated/MoveOfferInfo";
import type { MoveProgressInfo } from "../../types/generated/MoveProgressInfo";
import type { MoveRecordInfo } from "../../types/generated/MoveRecordInfo";
import type { MoveStopInfo } from "../../types/generated/MoveStopInfo";
import type { MoveUndoneInfo } from "../../types/generated/MoveUndoneInfo";

export type { MoveDoneInfo, MoveFinishedInfo, MoveOfferInfo, MoveProgressInfo, MoveRecordInfo, MoveStopInfo, MoveUndoneInfo };

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
export async function switchMoveExecute(sessionId: string, targetProfileId: string, takeNotes: boolean, targetPlaceId: string | null = null): Promise<MoveDoneInfo> {
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
  if (!hasLiveBackend()) return window.__AIJ_MOVE_UNDONE__ ?? { words: "Undone: the conversation is back where it was.", kept: "", notes: [] };
  return call("switch_move_undo", { moveId });
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
  if (!hasLiveBackend()) return window.__AIJ_MOVE_FINISHED__ ?? { words: "Finished: the conversation stays where it moved, checked once more." };
  return call("switch_move_finish", { moveId });
}

/** AJ-E2325: deletes a finished move's backup (its Undo goes with it), by
 * its id, after the sheet says so. Returns the words. */
export async function switchMoveDeleteBackup(moveId: string): Promise<string> {
  if (!hasLiveBackend()) return window.__AIJ_MOVE_BACKUP_DELETED__ ?? "Backup deleted: this move can no longer be undone.";
  return call("switch_move_delete_backup", { moveId });
}
