/**
 * AJ-E1205/E1206: Cloud backup and the Macs on the account.
 */
import { call, liveBackend as hasBackend } from "./transport";
import type { AccountMacsInfo } from "../../types/generated/AccountMacsInfo";
import type { CloudBackupInfo } from "../../types/generated/CloudBackupInfo";
import type { CloudBackupStatusInfo } from "../../types/generated/CloudBackupStatusInfo";
import type { LastBackupInfo } from "../../types/generated/LastBackupInfo";
import type { RestorePlanInfo } from "../../types/generated/RestorePlanInfo";
import { previewHooks } from "../preview-hooks";


interface CloudBackupSeams {
  __AIJ_CLOUD_BACKUP__?: CloudBackupStatusInfo;
  __AIJ_CLOUD_BACKUPS__?: CloudBackupInfo[];
  __AIJ_CLOUD_BACKUP_CALLS__?: string[];
  __AIJ_CLOUD_BACKUP_ERROR__?: string;
  __AIJ_CLOUD_RESTORE_PLAN__?: RestorePlanInfo;
  __AIJ_CLOUD_RESTORE_ERROR__?: string;
  __AIJ_CLOUD_BACKUP_HOLD__?: boolean;
}
export const cloudSeams = () => window as unknown as CloudBackupSeams;
const cloudCall = (called: string) => {
  const seams = cloudSeams();
  seams.__AIJ_CLOUD_BACKUP_CALLS__ = [...(seams.__AIJ_CLOUD_BACKUP_CALLS__ ?? []), called];
};
/** Where Cloud backup stands. */
export async function cloudBackupStatus(): Promise<CloudBackupStatusInfo> {
  if (!hasBackend) return { ...previewHooks().appState.mockCloud() };
  return call("cloud_backup_status");
}

/** Sets the schedule (off | daily | weekly). */
export async function cloudBackupSetSchedule(schedule: string): Promise<CloudBackupStatusInfo> {
  if (!hasBackend) {
    cloudCall(`schedule:${schedule}`);
    const next = { ...previewHooks().appState.mockCloud(), schedule };
    cloudSeams().__AIJ_CLOUD_BACKUP__ = next;
    return next;
  }
  return call("cloud_backup_set_schedule", { schedule });
}

/** Backs up now (builds, seals and uploads). */
export async function cloudBackupNow(): Promise<LastBackupInfo> {
  if (!hasBackend) {
    cloudCall("now");
    const seams = cloudSeams();
    if (seams.__AIJ_CLOUD_BACKUP_HOLD__) return new Promise<LastBackupInfo>(() => {});
    if (seams.__AIJ_CLOUD_BACKUP_ERROR__) throw new Error(seams.__AIJ_CLOUD_BACKUP_ERROR__);
    const last = { at_ms: Date.now(), ok: true, size_bytes: 412_000, error: null };
    seams.__AIJ_CLOUD_BACKUP__ = { ...previewHooks().appState.mockCloud(), last };
    return last;
  }
  return call("cloud_backup_now");
}

/** The account's backups, newest first. */
export async function cloudBackupList(): Promise<CloudBackupInfo[]> {
  if (!hasBackend) {
    cloudCall("list");
    return [...(cloudSeams().__AIJ_CLOUD_BACKUPS__ ?? [])];
  }
  return call("cloud_backup_list");
}

/** Downloads and opens one backup; returns the verified restore plan. */
export async function cloudBackupPrepareRestore(id: string, recoveryCode: string | null): Promise<RestorePlanInfo> {
  if (!hasBackend) {
    cloudCall(`prepare:${id}:${recoveryCode ? "code" : "key"}`);
    const seams = cloudSeams();
    if (seams.__AIJ_CLOUD_RESTORE_ERROR__) throw new Error(seams.__AIJ_CLOUD_RESTORE_ERROR__);
    if (seams.__AIJ_CLOUD_RESTORE_PLAN__) return seams.__AIJ_CLOUD_RESTORE_PLAN__;
    throw new Error("No backend in this preview");
  }
  return call("cloud_backup_prepare_restore", { id, recoveryCode });
}


interface AccountMacSeams {
  __AIJ_ACCOUNT_MACS__?: AccountMacsInfo;
  __AIJ_ACCOUNT_MACS_CALLS__?: string[];
}
const accountMacSeams = () => window as unknown as AccountMacSeams;

/** Your other linked Macs (Fleet discovery on; at most one read a minute). */
export async function fleetAccountMacs(): Promise<AccountMacsInfo> {
  if (!hasBackend) {
    const seams = accountMacSeams();
    seams.__AIJ_ACCOUNT_MACS_CALLS__ = [...(seams.__AIJ_ACCOUNT_MACS_CALLS__ ?? []), "list"];
    const seeded = seams.__AIJ_ACCOUNT_MACS__;
    if (seeded) return { ...seeded, macs: seeded.macs.map((mac) => ({ ...mac })) };
    throw new Error("Fleet discovery is off");
  }
  return call("fleet_account_macs");
}

/** Pins a listed Mac after its fingerprint was confirmed here. */
export async function fleetAccountPin(deviceId: string, fingerprint: string): Promise<void> {
  if (!hasBackend) {
    const seams = accountMacSeams();
    seams.__AIJ_ACCOUNT_MACS_CALLS__ = [...(seams.__AIJ_ACCOUNT_MACS_CALLS__ ?? []), `pin:${deviceId}:${fingerprint}`];
    const mac = seams.__AIJ_ACCOUNT_MACS__?.macs.find((row) => row.device_id === deviceId);
    if (mac) mac.state = "waiting";
    return;
  }
  return call("fleet_account_pin", { deviceId, fingerprint });
}
