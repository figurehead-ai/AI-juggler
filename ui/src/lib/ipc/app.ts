/**
 * AI Juggler itself (ENGINEERING-GUIDE U3: one IPC module per feature):
 * "Prepare to uninstall". In the preview, the seam `__AIJ_UNINSTALL__`
 * answers instead (or a sample).
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { UninstallPrepInfo } from "../../types/generated/UninstallPrepInfo";

export type { UninstallPrepInfo };

const SAMPLE: UninstallPrepInfo = {
  status_lines: [{ account: "Work Max", outcome: "restored" }],
  data_folder: "~/Library/Application Support/AI Juggler",
  keychain_services: ["ai.figurehead.juggler", "ai.figurehead.juggler.vault"],
  worktrees: [{ path: "~/Dev/payments-api-juggler-fix", branch: "juggler/fix", exists: true }],
};

/** Puts back what AI Juggler changed outside its data; lists the rest. */
export async function appPrepareUninstall(): Promise<UninstallPrepInfo> {
  if (!hasLiveBackend()) {
    return (window as unknown as { __AIJ_UNINSTALL__?: UninstallPrepInfo }).__AIJ_UNINSTALL__ ?? SAMPLE;
  }
  return invoke<UninstallPrepInfo>("app_prepare_uninstall");
}
