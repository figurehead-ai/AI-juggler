/** The design preview's app state: what the OS must allow, the live-preview frame, updates and Cloud backup (moved from lib/ipc, AJ-E2018). */
import type { OsAccessInfo } from "../types/generated/OsAccessInfo";
import { updateSeams } from "../lib/ipc/updates";
import type { UpdateStatusInfo } from "../types/generated/UpdateStatusInfo";
import { cloudSeams } from "../lib/ipc/cloud-backup";
import type { CloudBackupStatusInfo } from "../types/generated/CloudBackupStatusInfo";

export const MOCK_OS_ACCESS: OsAccessInfo[] = [
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

/** AJ-E415: the deterministic mock preview frame (a rendered fake-app
 * window; harness captures pin phases via __AIJ_PREVIEW_PHASE__). */
export const MOCK_PREVIEW_PNG_B64 = "iVBORw0KGgoAAAANSUhEUgAAAUAAAADICAIAAAAWZq/8AAACA0lEQVR42u3cMQqDQBCGUQ8hGEhY0SZ4hRSpQ4ocQSxzBk/vBazE5ncfvBMsfNXOTPN6f4FQjScAAQMCBgQMAgYEDAgYEDAIGBAwIGAQMCBgQMCAgEHAgIABAQMCBgEDAgYOB1yGCQglYEgOuO0KEErAIGBAwICAQcCAgAEBAwIGAQMCBgQMAgYEDAgYEDBcP+DlvwKhBAwCBgQMCBgEDAgYEDAgYBAwIGBAwFBzwMbBwTYSIGBAwCBgQMCAgAEBg4ABAQMCBgEDiQF/fjMQSsAgYEDAgIBBwICAAQEDewH7CgeTWICAAQGDgAEBAwIGBAwCBgQMCBgEDCQG/OifQCgBg4ABAQMCBgEDAgYEDAgYBAwIGBAw1Bzw7T4CoQQMAgYEDAgYBAwIGBAwIGAQMCBgQMBQc8DGwcE2EiBgQMAgYEDAgIABAYOAAQEDAgYBA+5CA7aRAAGDgAEBAwIGBAwCBgQMCBgEDAgYcBcasI0EAgYEDAgYEDAIGBAwIGAQMCBgQMCAu9BgGwkQMCBgELBXAAEDAgYEDAIGBAwIGBAwuAsN2EYCBAwCBvICbrsChBIwCBgQMCBgEDAgYEDAgIBBwICAAQGDgAEBAwIGBAwCBgQMCBgQMAgYEDAgYBAwIGBAwICAQcCAgAEBAwIGAQMCBgQMAgYEDAgYEDAIGBAwcIoN1C7gkSubCwYAAAAASUVORK5CYII=";

export const mockUpdateStatus = (): UpdateStatusInfo =>
  updateSeams().__AIJ_UPDATE_STATUS__ ?? {
    current_version: "0.0.1",
    channel: "beta",
    edition: "main",
    app_name: "AI Juggler",
    channels: ["stable", "beta"],
    configured: false,
    check_on_launch: true,
    state: "idle",
    available: null,
    progress: null,
    error: null,
    error_kind: null,
    error_detail: null,
    restart_wait: null,
    backup_path: null,
    checked_at_ms: null,
  };

export const mockCloud = (): CloudBackupStatusInfo =>
  cloudSeams().__AIJ_CLOUD_BACKUP__ ?? {
    enabled: true,
    code_saved: true,
    schedule: "weekly",
    last: null,
    uploading: false,
    limit_bytes: 25 * 1024 * 1024,
  };
