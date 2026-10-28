/**
 * AI Juggler itself (ENGINEERING-GUIDE U3: one IPC module per feature):
 * "Prepare to uninstall", opening at login (AJ-E1509), and diagnostics and
 * the bug form (AJ-E1517). In the preview, the seam `__AIJ_UNINSTALL__`
 * answers instead (or a sample).
 */
import { invoke, saveDialog } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { LoginItemInfo } from "../../types/generated/LoginItemInfo";
import type { UninstallPrepInfo } from "../../types/generated/UninstallPrepInfo";
import type { DiagnosticsPreviewInfo } from "../../types/generated/DiagnosticsPreviewInfo";
import type { DiagnosticsSavedInfo } from "../../types/generated/DiagnosticsSavedInfo";
import type { AppUserInfo } from "../../types/generated/AppUserInfo";

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

// --- AJ-E1517: diagnostics and the bug form ----------------------------

export type { DiagnosticsPreviewInfo, DiagnosticsSavedInfo };

/** The preview's diagnostics (the seam `__AIJ_DIAGNOSTICS__` sets them;
 * `__AIJ_DIAGNOSTICS_DEST__` is where the save sheet "saves"). */
const diagnosticsSeams = () =>
  window as unknown as {
    __AIJ_DIAGNOSTICS__?: DiagnosticsPreviewInfo;
    __AIJ_DIAGNOSTICS_DEST__?: string | null;
    __AIJ_DIAGNOSTICS_SAVED__?: { id: string; dest: string };
    __AIJ_BUG_FORM_OPENED__?: number;
  };

const SAMPLE_DIAGNOSTICS: DiagnosticsPreviewInfo = {
  id: "preview",
  file_name: "AI Juggler diagnostics 2026-10-23.zip",
  left_out: ["restore_token", "figurehead_device_id", "fleet_peers"],
  files: [
    {
      name: "about.json",
      about: "Versions, the database's schema and counts",
      text: '{\n  "app": { "name": "AI Juggler", "version": "0.0.1", "edition": "beta", "channel": "beta" },\n  "database": { "schema_this_build": 31, "schema_on_disk": 31 }\n}',
    },
    {
      name: "app-settings.json",
      about: "Your settings, without what's left out",
      text: '{\n  "settings": { "theme": "dark", "projects.default_folder": "~/Dev" },\n  "left_out": ["restore_token", "figurehead_device_id", "fleet_peers"]\n}',
    },
    {
      name: "logs/supervision.jsonl",
      about: "Sessions started and ended",
      text: '{"event":"spawn","program":"~/.local/bin/claude","cwd":"~/Dev/payments-api"}\n{"event":"exit","code":0}\n',
    },
  ],
};

/** AJ-E1517: what a diagnostics zip will hold, scrubbed, to look over. */
export async function appDiagnosticsPreview(): Promise<DiagnosticsPreviewInfo> {
  if (!hasLiveBackend()) return diagnosticsSeams().__AIJ_DIAGNOSTICS__ ?? SAMPLE_DIAGNOSTICS;
  return invoke<DiagnosticsPreviewInfo>("app_diagnostics_preview");
}

/** Where to save it (the native save sheet); null when cancelled. */
export async function pickDiagnosticsDestination(fileName: string): Promise<string | null> {
  if (!hasLiveBackend()) return diagnosticsSeams().__AIJ_DIAGNOSTICS_DEST__ ?? null;
  const chosen = await saveDialog({ defaultPath: fileName, filters: [{ name: "Zip archive", extensions: ["zip"] }] });
  return typeof chosen === "string" ? chosen : null;
}

/** Saves the zip shown in preview `id`, exactly as shown. */
export async function appDiagnosticsExport(id: string, dest: string): Promise<DiagnosticsSavedInfo> {
  if (!hasLiveBackend()) {
    diagnosticsSeams().__AIJ_DIAGNOSTICS_SAVED__ = { id, dest };
    return { path: dest.replace(/^\/Users\/[^/]+/, "~"), bytes: 4_812 };
  }
  return invoke<DiagnosticsSavedInfo>("app_diagnostics_export", { id, dest });
}

/** Opens the bug form on GitHub in the browser. */
/** AJ-E2507: who the user menu says you are (the system's name for you
 * and for this computer). */
export async function appUserGet(): Promise<AppUserInfo> {
  if (!hasLiveBackend()) return (window as unknown as { __AIJ_USER__?: AppUserInfo }).__AIJ_USER__ ?? { name: "Sam", initial: "S", computer: "Mac", computers: "Macs" };
  return invoke<AppUserInfo>("app_user_get");
}

/** AJ-E2507: Help and guides, in the browser. */
export async function appHelpOpen(): Promise<void> {
  if (!hasLiveBackend()) return;
  return invoke<void>("app_help_open");
}

export async function appReportBugOpen(): Promise<void> {
  if (!hasLiveBackend()) {
    const seams = diagnosticsSeams();
    seams.__AIJ_BUG_FORM_OPENED__ = (seams.__AIJ_BUG_FORM_OPENED__ ?? 0) + 1;
    return;
  }
  return invoke<void>("app_report_bug_open");
}
