/**
 * ADR-0031 (D-93): Move to account, a switch by move (one thin file per
 * feature). In the preview, the seams `__AIJ_MOVE_OFFER__` (the offer;
 * none by default, so the switch screens the gallery shows stay Copy),
 * `__AIJ_MOVE_DONE__` / `__AIJ_MOVE_ERROR__` (what a move returns) and
 * `__AIJ_MOVE_UNDONE__` stand in.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { MoveDoneInfo } from "../../types/generated/MoveDoneInfo";
import type { MoveOfferInfo } from "../../types/generated/MoveOfferInfo";
import type { MoveRecordInfo } from "../../types/generated/MoveRecordInfo";

export type { MoveDoneInfo, MoveOfferInfo, MoveRecordInfo };

declare global {
  interface Window {
    __AIJ_MOVE_OFFER__?: MoveOfferInfo | null;
    __AIJ_MOVE_DONE__?: MoveDoneInfo;
    __AIJ_MOVE_ERROR__?: string;
    __AIJ_MOVE_UNDONE__?: string;
  }
}

/** Whether and how this conversation can be moved to the target (reads
 * only); null where moving doesn't apply. */
export async function switchMoveOffer(sessionId: string, targetProfileId: string): Promise<MoveOfferInfo | null> {
  if (!hasLiveBackend()) return window.__AIJ_MOVE_OFFER__ ?? null;
  const offer = await call("switch_move_offer", { sessionId, targetProfileId });
  return offer.applies ? offer : null;
}

/** Moves the conversation and its family (the user's click). */
export async function switchMoveExecute(sessionId: string, targetProfileId: string, takeNotes: boolean): Promise<MoveDoneInfo> {
  if (!hasLiveBackend()) {
    const failure = window.__AIJ_MOVE_ERROR__;
    if (failure) throw failure;
    const done = window.__AIJ_MOVE_DONE__;
    if (!done) throw "No move in the preview";
    return done;
  }
  return call("switch_move_execute", { sessionId, targetProfileId, takeNotes });
}

/** Undoes a move (the user's click). */
export async function switchMoveUndo(moveId: string): Promise<string> {
  if (!hasLiveBackend()) return window.__AIJ_MOVE_UNDONE__ ?? "Undone: the conversation is back where it was.";
  return call("switch_move_undo", { moveId });
}

/** The moves AI Juggler knows of, newest first. */
export async function switchMoves(): Promise<MoveRecordInfo[]> {
  if (!hasLiveBackend()) return [];
  return call("switch_moves", {});
}
