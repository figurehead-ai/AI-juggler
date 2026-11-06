/**
 * AJ-E1204: the backup vault.
 */
import { invoke, liveBackend as hasBackend, pickPath } from "./transport";
import type { RecoveryCodeInfo } from "../../types/generated/RecoveryCodeInfo";
import type { VaultStatusInfo } from "../../types/generated/VaultStatusInfo";


interface VaultSeams {
  __AIJ_VAULT_STATUS__?: VaultStatusInfo;
  __AIJ_VAULT_CODE__?: string;
  __AIJ_VAULT_CALLS__?: string[];
  __AIJ_VAULT_SAVE_DEST__?: string | null;
}
const vaultSeams = () => window as unknown as VaultSeams;
const vaultCall = (call: string) => {
  const seams = vaultSeams();
  seams.__AIJ_VAULT_CALLS__ = [...(seams.__AIJ_VAULT_CALLS__ ?? []), call];
};

/** Where the vault stands on this Mac. */
export async function vaultStatus(): Promise<VaultStatusInfo> {
  // The preview's default is a Mac whose code is already saved; specs seed
  // the first-time state explicitly.
  if (!hasBackend) return { ...(vaultSeams().__AIJ_VAULT_STATUS__ ?? { has_key: true, code_saved: true }) };
  return invoke<VaultStatusInfo>("vault_status");
}

/** Makes the vault key (first time) and returns its recovery code. */
export async function vaultSetup(): Promise<RecoveryCodeInfo> {
  if (!hasBackend) {
    vaultCall("setup");
    const seams = vaultSeams();
    seams.__AIJ_VAULT_STATUS__ = { has_key: true, code_saved: false };
    return { code: seams.__AIJ_VAULT_CODE__ ?? "ABCD-EFGH-IJKL-MNOP-QRST-UVWX-YZ23-4567-ABCD-EFGH-IJKL-MNOP-QRST-UVWX" };
  }
  return invoke<RecoveryCodeInfo>("vault_setup");
}

/** "I've saved my recovery code". */
export async function vaultConfirmSaved(): Promise<VaultStatusInfo> {
  if (!hasBackend) {
    vaultCall("confirm");
    const next = { has_key: true, code_saved: true };
    vaultSeams().__AIJ_VAULT_STATUS__ = next;
    return next;
  }
  return invoke<VaultStatusInfo>("vault_confirm_saved");
}

/** Saves the recovery code to a text file the user picks; false if cancelled. */
export async function vaultSaveCode(): Promise<boolean> {
  let dest: string | null;
  if (!hasBackend) {
    dest = vaultSeams().__AIJ_VAULT_SAVE_DEST__ ?? null;
  } else {
    dest = await pickPath("recoveryCodeDest");
  }
  if (!dest) return false;
  if (!hasBackend) {
    vaultCall(`save:${dest}`);
    return true;
  }
  await invoke<void>("vault_save_code", { dest });
  return true;
}

/** A new Mac enters its recovery code. */
export async function vaultEnterCode(code: string): Promise<VaultStatusInfo> {
  if (!hasBackend) {
    vaultCall(`enter:${code}`);
    const next = { has_key: true, code_saved: true };
    vaultSeams().__AIJ_VAULT_STATUS__ = next;
    return next;
  }
  return invoke<VaultStatusInfo>("vault_enter_code", { code });
}
