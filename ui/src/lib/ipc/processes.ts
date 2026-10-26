/**
 * AJ-E2401..E2406 — the Processes view over IPC (ENGINEERING-GUIDE U3: one
 * thin file per feature). The backend builds one graph of apps, sessions
 * and their processes on its 2 s tick and says when it changed
 * (`processes://changed`); a page reads the rows it shows, by cursor.
 *
 * In the preview (no backend) the same calls answer from the made-up Mac
 * in `preview/processes`, so the page, its filters and its sheets work in
 * the dev preview and the gallery.
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { KillExecuteData, KillVerifyData } from "../../components/KillSheet";
import type { ProcessQueryInput } from "../../types/generated/ProcessQueryInput";
import type { ProcessPageInfo } from "../../types/generated/ProcessPageInfo";
import type { ProcessCountsInfo } from "../../types/generated/ProcessCountsInfo";
import type { ProcessGraphRowInfo } from "../../types/generated/ProcessGraphRowInfo";
import type { ProcRowInfo } from "../../types/generated/ProcRowInfo";
import type { RollupInfo } from "../../types/generated/RollupInfo";
import type { CleanupHistoryPageInfo } from "../../types/generated/CleanupHistoryPageInfo";
import type { CleanupEventInfo } from "../../types/generated/CleanupEventInfo";
import type { RulePreviewInfo } from "../../types/generated/RulePreviewInfo";
import type { CleanupPolicyInfo } from "../../types/generated/CleanupPolicyInfo";
import type { ProcessSnapshotInfo } from "../../types/generated/ProcessSnapshotInfo";
import type { ProcessGraphChangedInfo } from "../../types/generated/ProcessGraphChangedInfo";

export type {
  CleanupEventInfo,
  CleanupHistoryPageInfo,
  CleanupPolicyInfo,
  ProcessCountsInfo,
  ProcessGraphChangedInfo,
  ProcessGraphRowInfo,
  ProcessPageInfo,
  ProcessQueryInput,
  ProcessSnapshotInfo,
  ProcRowInfo,
  RollupInfo,
  RulePreviewInfo,
};

/** The event the backend sends when the graph changed. */
export const PROCESSES_CHANGED = "processes://changed";

/** A query with every filter off (the page's default). */
export function processQuery(over: Partial<ProcessQueryInput> = {}): ProcessQueryInput {
  return {
    view: "attention",
    session_id: null,
    states: [],
    app: null,
    account_id: null,
    project_id: null,
    has_port: false,
    heavy: false,
    show_internal: false,
    search: "",
    expanded: [],
    collapsed: [],
    cursor: null,
    limit: null,
    ...over,
  };
}

/** The preview's answers (loaded only without a backend). */
const preview = () => import("../../preview/processes");

/** One page of the Processes view. */
export async function processesPage(input: ProcessQueryInput): Promise<ProcessPageInfo> {
  if (!hasLiveBackend()) return (await preview()).previewPage(input);
  return invoke<ProcessPageInfo>("processes_page", { input });
}

/** The counts now (the top bar's and This Mac's "to clean up"). */
export async function processesCounts(): Promise<ProcessCountsInfo> {
  if (!hasLiveBackend()) return (await preview()).previewCounts();
  return invoke<ProcessCountsInfo>("processes_counts");
}

/** Keep (not a leftover), Remind me later for `minutes`, or neither. */
export async function processesMark(
  pid: number,
  startTime: number,
  mark: "keep" | "snooze" | null,
  minutes?: number,
): Promise<void> {
  if (!hasLiveBackend()) return (await preview()).previewMark(pid, startTime, mark, minutes);
  return invoke<void>("processes_mark", { pid, startTime, mark, minutes: minutes ?? null });
}

/** D-41b: the verify sheet for another app's leftover (zero signals). */
export async function processesObservedVerify(pid: number, startTime: number | null): Promise<KillVerifyData> {
  if (!hasLiveBackend()) return (await preview()).previewObservedVerify(pid, startTime);
  return invoke<KillVerifyData>("processes_observed_verify", { pid, startTime });
}

/** D-41b: the user's stop of another app's leftover (checked again, audited). */
export async function processesObservedStop(
  pid: number,
  startTime: number | null,
  attempt: string,
): Promise<KillExecuteData> {
  if (!hasLiveBackend()) return (await preview()).previewObservedStop(pid);
  return invoke<KillExecuteData>("processes_observed_stop", { pid, startTime, attempt });
}

/** Cleanup history (7 days), one page, optionally one session's or project's. */
export async function processesHistory(
  sessionId: string | null,
  projectId: string | null,
  cursor: string | null,
  limit?: number,
): Promise<CleanupHistoryPageInfo> {
  if (!hasLiveBackend()) return (await preview()).previewHistory(sessionId, projectId, cursor);
  return invoke<CleanupHistoryPageInfo>("processes_history", {
    sessionId,
    projectId,
    cursor,
    limit: limit ?? null,
  });
}

/** What an account's automatic rule would tidy now (nothing is stopped). */
export async function processesDryRun(accountId: string, tier: 1 | 2): Promise<RulePreviewInfo> {
  if (!hasLiveBackend()) return (await preview()).previewDryRun(accountId, tier);
  return invoke<RulePreviewInfo>("processes_dry_run", { accountId, tier });
}

/** Each account's automatic cleanup, with pause and the crash hold. */
export async function processesPolicy(): Promise<CleanupPolicyInfo> {
  if (!hasLiveBackend()) return (await preview()).previewPolicy();
  return invoke<CleanupPolicyInfo>("processes_policy");
}

/** A snapshot to save: names and hashed command lines only. */
export async function processesSnapshot(): Promise<ProcessSnapshotInfo> {
  if (!hasLiveBackend()) return (await preview()).previewSnapshot();
  return invoke<ProcessSnapshotInfo>("processes_snapshot");
}

/** Calls `on` each time the graph changes; returns the disposer. In the
 * preview, a `aij:processes-changed` window event stands in. */
export function onProcessesChanged(on: (change: ProcessGraphChangedInfo) => void): () => void {
  if (!hasLiveBackend()) {
    const handler = (event: Event) => on((event as CustomEvent<ProcessGraphChangedInfo>).detail);
    window.addEventListener("aij:processes-changed", handler);
    return () => window.removeEventListener("aij:processes-changed", handler);
  }
  let disposed = false;
  let stop: (() => void) | null = null;
  void listen<ProcessGraphChangedInfo>(PROCESSES_CHANGED, (event) => on(event.payload)).then((unlisten) => {
    if (disposed) unlisten();
    else stop = unlisten;
  });
  return () => {
    disposed = true;
    stop?.();
  };
}

/** Reveal folder: the session's folder in Finder (the one the backend
 * knows for it). False when it has none. */
export async function processesReveal(sessionKey: string): Promise<boolean> {
  if (!hasLiveBackend()) return false;
  return invoke<boolean>("processes_reveal", { sessionKey });
}

/** Show in Activity Monitor (macOS's own). */
export async function processesActivityMonitor(): Promise<boolean> {
  if (!hasLiveBackend()) return false;
  return invoke<boolean>("processes_activity_monitor");
}
