/**
 * AI Juggler itself (ENGINEERING-GUIDE U3: one IPC module per feature):
 * "Prepare to uninstall", and opening at login (AJ-E1509). In the preview, the seam `__AIJ_UNINSTALL__`
 * answers instead (or a sample).
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { LoginItemInfo } from "../../types/generated/LoginItemInfo";
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

export type { LoginItemInfo };

/** The preview's login item (the seam `__AIJ_LOGIN_ITEM__` sets it). */
const preview = () => window as unknown as { __AIJ_LOGIN_ITEM__?: LoginItemInfo["state"] };

/** AJ-E1509: whether AI Juggler opens at login (`on` | `off` |
 * `needs_approval` | `unavailable`). */
export async function appLoginItemGet(): Promise<LoginItemInfo> {
  if (!hasLiveBackend()) return { state: preview().__AIJ_LOGIN_ITEM__ ?? "off" };
  return invoke<LoginItemInfo>("app_login_item_get");
}

/** AJ-E1509: turns opening at login on or off; how it stands after. */
export async function appLoginItemSet(on: boolean): Promise<LoginItemInfo> {
  if (!hasLiveBackend()) {
    preview().__AIJ_LOGIN_ITEM__ = on ? "on" : "off";
    return { state: on ? "on" : "off" };
  }
  return invoke<LoginItemInfo>("app_login_item_set", { on });
}
