/**
 * D-62 (AJ-E3204) — the Apps as stored (the backend's `features::apps`):
 * each one's id, the user's name for it, its folder and the Account its
 * folder was last read as. What runs and what can start come from the
 * places and the folders (lib/apps.ts joins them). The preview has none
 * stored: every App there is made from its places and folders.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { StoredAppInfo } from "../../types/generated/StoredAppInfo";

export type { StoredAppInfo };

/** The event this window sends when an App was renamed. */
export const APPS_CHANGED = "aij:apps-changed";

/** What one page asks for: every App (one per folder and per desktop
 * profile seen). */
const ALL = { cursor: null, limit: 200 };

/** The names the preview gives (no backend to keep them). */
const previewNames = new Map<string, string>();

export async function appsList(): Promise<StoredAppInfo[]> {
  if (!hasLiveBackend()) {
    return [...previewNames].map(([id, label]) => ({
      id,
      kind: id.startsWith("cli:") ? "cli" : "desktop",
      tool: "",
      host_app_id: null,
      label,
      folder_id: null,
      account_id: null,
      last_seen_ms: 0,
    }));
  }
  const page = await invoke<{ items: StoredAppInfo[] }>("apps_list", { page: ALL });
  return page.items;
}

/** Names an App (`null` or blank: its default name). */
export async function appsRename(id: string, label: string | null): Promise<void> {
  if (!hasLiveBackend()) {
    const name = label?.trim() ?? "";
    if (name) previewNames.set(id, name);
    else previewNames.delete(id);
  } else {
    await invoke<void>("apps_rename", { id, label });
  }
  window.dispatchEvent(new Event(APPS_CHANGED));
}
