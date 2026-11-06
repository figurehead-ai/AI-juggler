/**
 * AJ-E1009: backup and restore.
 */
import { call, listen, liveBackend as hasBackend, pickPath } from "./transport";
import { EVENTS } from "../bridge";
import type { BackupExportInfo } from "../../types/generated/BackupExportInfo";
import type { RestorePlanInfo } from "../../types/generated/RestorePlanInfo";
import type { RestoreResultInfo } from "../../types/generated/RestoreResultInfo";


interface BackupSeams {
  __AIJ_BACKUP_CALLS__?: string[];
  __AIJ_BACKUP_DEST__?: string | null;
  __AIJ_BACKUP_FILE__?: string | null;
  __AIJ_BACKUP_EXPORT__?: BackupExportInfo;
  __AIJ_BACKUP_EXPORT_ERROR__?: string;
  __AIJ_RESTORE_PLAN__?: RestorePlanInfo;
  __AIJ_RESTORE_PLAN_ERROR__?: string;
  __AIJ_RESTORE_RESULT__?: RestoreResultInfo;
  __AIJ_BACKUP_PROGRESS__?: Array<[number, number]>;
  __AIJ_BACKUP_HOLD__?: boolean;
}
const backupSeams = () => window as unknown as BackupSeams;
const backupCall = (called: string) => {
  const seams = backupSeams();
  seams.__AIJ_BACKUP_CALLS__ = [...(seams.__AIJ_BACKUP_CALLS__ ?? []), called];
};
let mockProgress: ((done: number, total: number) => void) | null = null;

/** Where to save a backup (native save sheet); null when cancelled. */
export async function pickBackupDestination(): Promise<string | null> {
  if (!hasBackend) return backupSeams().__AIJ_BACKUP_DEST__ ?? null;
  const day = new Date().toISOString().slice(0, 10);
  return pickPath("backupDest", `AI Juggler backup ${day}.ajbackup`);
}

/** Which backup to restore (native open sheet); null when cancelled. */
export async function pickBackupFile(): Promise<string | null> {
  if (!hasBackend) return backupSeams().__AIJ_BACKUP_FILE__ ?? null;
  return pickPath("backupFile");
}

/** Export progress (bytes done, bytes total). */
export function onBackupProgress(handler: (done: number, total: number) => void): () => void {
  if (!hasBackend) {
    mockProgress = handler;
    return () => {
      if (mockProgress === handler) mockProgress = null;
    };
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<{ done: number; total: number }>(EVENTS.backupProgress, (event) => handler(event.payload.done, event.payload.total))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {});
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** Writes a backup of AI Juggler's own data to `dest`. */
export async function backupExport(dest: string): Promise<BackupExportInfo> {
  if (!hasBackend) {
    backupCall(`export:${dest}`);
    const seams = backupSeams();
    for (const [done, total] of seams.__AIJ_BACKUP_PROGRESS__ ?? [[1, 1]]) mockProgress?.(done, total);
    if (seams.__AIJ_BACKUP_HOLD__) return new Promise<BackupExportInfo>(() => {});
    if (seams.__AIJ_BACKUP_EXPORT_ERROR__) throw new Error(seams.__AIJ_BACKUP_EXPORT_ERROR__);
    if (seams.__AIJ_BACKUP_EXPORT__) return { ...seams.__AIJ_BACKUP_EXPORT__, path: dest };
    throw new Error("No backend in this preview");
  }
  return call("backup_export", { dest });
}

/** Verifies a backup in full; returns the restore plan (nothing changes yet). */
export async function backupVerify(path: string): Promise<RestorePlanInfo> {
  if (!hasBackend) {
    backupCall(`verify:${path}`);
    const seams = backupSeams();
    if (seams.__AIJ_RESTORE_PLAN_ERROR__) throw new Error(seams.__AIJ_RESTORE_PLAN_ERROR__);
    if (seams.__AIJ_RESTORE_PLAN__) return { ...seams.__AIJ_RESTORE_PLAN__, path };
    throw new Error("No backend in this preview");
  }
  return call("backup_verify", { path });
}

/** Applies a verified restore (with its plan's token). */
export async function backupRestore(path: string, token: string): Promise<RestoreResultInfo> {
  if (!hasBackend) {
    backupCall(`restore:${path}:${token}`);
    const result = backupSeams().__AIJ_RESTORE_RESULT__;
    if (result) return result;
    throw new Error("No backend in this preview");
  }
  return call("backup_restore", { path, token });
}
