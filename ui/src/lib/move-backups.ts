/**
 * 0.5.1 F93 — the move backups kept for Undo, one read for every surface
 * that says them: the header's "N to clean up", Settings › Moves' head
 * and footer, and the clean-up view's line. Before, the header read them
 * once at start and again only after an action on a Moves row, while the
 * Moves section read its own: after a move the header said 1 (or
 * nothing) beside "2 move backups". Now every surface shows the same
 * answer, read again whenever one of them opens and after every action
 * that makes or lets go of a backup (a move, Undo, Finish, a repair,
 * Delete backup, Clear older: the IPC wrappers tell `BACKUPS_CHANGED`).
 */
import { useEffect } from "react";
import { sharedRead, useSharedRead, type SharedRead } from "./shared-read";
import { BACKUPS_CHANGED, switchMoveBackups, type MoveBackupsInfo } from "./ipc/switch-move";

const BACKUPS: SharedRead<MoveBackupsInfo> = sharedRead(() => switchMoveBackups(), "the move backups");

function readAgain(): void {
  void BACKUPS.refresh();
}

/** How many surfaces listen: one window listener while any does. */
let listeners = 0;

function listen(): () => void {
  if (listeners++ === 0) window.addEventListener(BACKUPS_CHANGED, readAgain);
  return () => {
    if (--listeners === 0) window.removeEventListener(BACKUPS_CHANGED, readAgain);
  };
}

/** The backups kept for Undo (`null` until read). */
export function useMoveBackups(): MoveBackupsInfo | null {
  useEffect(listen, []);
  return useSharedRead(BACKUPS) ?? null;
}
