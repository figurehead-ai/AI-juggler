/**
 * App settings: generic keys, notification prefs, what the OS must allow
 * (D-25), the cleanup default and the Go-to hotkey.
 */
import { invoke, listen, liveBackend as hasBackend } from "./transport";
import { EVENTS } from "../bridge";
import type { NotificationPrefsInfo } from "../../types/generated/NotificationPrefsInfo";
import type { OsAccessInfo } from "../../types/generated/OsAccessInfo";
import type { ReopenBlockInfo } from "../../types/generated/ReopenBlockInfo";

/** Mock app settings (browser preview and tests): kept for the page. */
const appSettingSeams = () =>
  window as unknown as { __AIJ_APP_SETTINGS__?: Record<string, string> };

/** Reads one app-level setting (schema v10 kv); null = never set. */
export async function appSettingGet(key: string): Promise<string | null> {
  if (!hasBackend) {
    const stored = appSettingSeams().__AIJ_APP_SETTINGS__?.[key];
    return stored === undefined || stored === "" ? null : stored;
  }
  return invoke<string | null>("app_setting_get", { key });
}

/** Writes one app-level setting (empty value = reset to default). */
export async function appSettingSet(key: string, value: string): Promise<void> {
  if (!hasBackend) {
    const seams = appSettingSeams();
    seams.__AIJ_APP_SETTINGS__ = { ...(seams.__AIJ_APP_SETTINGS__ ?? {}), [key]: value };
    return;
  }
  await invoke("app_setting_set", { key, value });
}

const notificationSeams = () =>
  window as unknown as { __AIJ_NOTIFICATION_PREFS__?: NotificationPrefsInfo; __AIJ_CLEANUP_DEFAULT__?: boolean };

/** Reads Settings › Notifications. */
export async function notificationPrefsGet(): Promise<NotificationPrefsInfo> {
  if (!hasBackend) {
    return { quiet_start: 22, quiet_end: 7, digest_muted_accounts: [], wait_after_min: 5, quota_first_pct: 80, quota_last_pct: 95, ...notificationSeams().__AIJ_NOTIFICATION_PREFS__ };
  }
  return invoke<NotificationPrefsInfo>("notification_prefs_get");
}

/** Saves Settings › Notifications (hours 0–23; equal hours = none). */
export async function notificationPrefsSet(prefs: NotificationPrefsInfo): Promise<NotificationPrefsInfo> {
  if (!hasBackend) {
    const saved = {
      ...prefs,
      digest_muted_accounts: [...new Set(prefs.digest_muted_accounts)].sort(),
    };
    notificationSeams().__AIJ_NOTIFICATION_PREFS__ = saved;
    return saved;
  }
  return invoke<NotificationPrefsInfo>("notification_prefs_set", { prefs });
}

export const osAccessSeams = () =>
  window as unknown as {
    __AIJ_OS_ACCESS__?: OsAccessInfo[];
    __AIJ_OS_ACCESS_CALLS__?: string[];
    /** Test seam: what a Reopen click is told (null = it relaunches). */
    __AIJ_REOPEN_BLOCK__?: ReopenBlockInfo | null;
  };
const MOCK_OS_ACCESS: OsAccessInfo[] = [
  {
    id: "focus",
    title: "Jump to a session's own window",
    why: "Brings the right terminal window to the front when you open a session.",
    system_name: "Accessibility and Automation",
    status: "not_asked",
    action: "prompt",
    note: null,
  },
];
// No "preview" (Screen Recording): as the backend, it's offered only once
// a tool can show a live preview, and none can yet. Tests of its reopen
// flow seed it through `__AIJ_OS_ACCESS__`.

/** What this OS needs, as it stands (never prompts — safe to poll). */
export async function osAccessList(): Promise<OsAccessInfo[]> {
  if (!hasBackend) return (osAccessSeams().__AIJ_OS_ACCESS__ ?? MOCK_OS_ACCESS).map((entry) => ({ ...entry }));
  return invoke<OsAccessInfo[]>("os_access_list");
}

/** Allow: raises the system's prompts for one entry. */
export async function osAccessRequest(id: string): Promise<OsAccessInfo[]> {
  if (!hasBackend) {
    const seams = osAccessSeams();
    seams.__AIJ_OS_ACCESS_CALLS__ = [...(seams.__AIJ_OS_ACCESS_CALLS__ ?? []), `request:${id}`];
    // The preview's user says yes in the system dialog.
    seams.__AIJ_OS_ACCESS__ = (seams.__AIJ_OS_ACCESS__ ?? MOCK_OS_ACCESS).map((entry) =>
      entry.id === id ? { ...entry, status: "allowed", action: "none", note: null } : entry,
    );
    return osAccessList();
  }
  return invoke<OsAccessInfo[]>("os_access_request", { id });
}

/** Opens the settings pane for one entry (a fixed link per id). */
export async function osAccessOpenSettings(id: string): Promise<void> {
  if (!hasBackend) {
    const seams = osAccessSeams();
    seams.__AIJ_OS_ACCESS_CALLS__ = [...(seams.__AIJ_OS_ACCESS_CALLS__ ?? []), `settings:${id}`];
    return;
  }
  await invoke("os_access_open_settings", { id });
}

/** Whether new Accounts start with left-behind dev servers tidied (D-08:
 * off unless the user chooses it). */
export async function cleanupDefaultGet(): Promise<boolean> {
  if (!hasBackend) return notificationSeams().__AIJ_CLEANUP_DEFAULT__ ?? false;
  return invoke<boolean>("cleanup_default_get");
}

/** Sets the cleanup default for new Accounts (existing ones keep theirs). */
export async function cleanupDefaultSet(on: boolean): Promise<boolean> {
  if (!hasBackend) {
    notificationSeams().__AIJ_CLEANUP_DEFAULT__ = on;
    return on;
  }
  return invoke<boolean>("cleanup_default_set", { on });
}

/** Rebinds the global "Go to instance…" hotkey (registers with the OS
 * first; a failure restores the previous binding). Empty turns it off:
 * there is no default (AJ-E2110), the user picks one. */
export async function goToHotkeySet(hotkey: string): Promise<void> {
  if (!hasBackend) {
    const seams = appSettingSeams();
    seams.__AIJ_APP_SETTINGS__ = { ...(seams.__AIJ_APP_SETTINGS__ ?? {}), go_to_hotkey: hotkey.trim() };
    return;
  }
  await invoke("app_go_to_hotkey_set", { hotkey });
}

/** Opens the system Accessibility privacy pane (the explainer's deep
 * link; the target is fixed in Rust — never caller-supplied). */
export async function openAccessibilitySettings(): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await invoke("open_accessibility_settings");
}

/** Fires when the global "Go to instance…" hotkey is pressed anywhere
 * in the system (AJ-E414 Req.4). No-backend: the palette stays
 * reachable in-app via ⌘K. */
export function onGoToHotkey(handler: () => void): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen(EVENTS.goTo, () => handler())
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {
      // backend gone; ⌘K remains
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}
