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
import { call } from "./transport";
import { listenWhileShown } from "../shown-window";
import { hasLiveBackend } from "../bridge";
import { portInventory, type PortInventoryInfo } from "./kill";
import type { KillExecuteData, KillVerifyData } from "../../components/KillSheet";
import type { ProcessQueryInput } from "../../types/generated/ProcessQueryInput";
import type { ProcessStatusInfo } from "../../types/generated/ProcessStatusInfo";
import type { Page } from "../../types/generated/Page";
import type { PageRequest } from "../../types/generated/PageRequest";
import type { ProcessCountsInfo } from "../../types/generated/ProcessCountsInfo";
import type { ProcessGraphRowInfo } from "../../types/generated/ProcessGraphRowInfo";
import type { ProcRowInfo } from "../../types/generated/ProcRowInfo";
import type { RollupInfo } from "../../types/generated/RollupInfo";
import type { CleanupEventInfo } from "../../types/generated/CleanupEventInfo";
import type { RulePreviewInfo } from "../../types/generated/RulePreviewInfo";
import type { CleanupPolicyInfo } from "../../types/generated/CleanupPolicyInfo";
import type { ProcessSnapshotInfo } from "../../types/generated/ProcessSnapshotInfo";
import type { ProcessGraphChangedInfo } from "../../types/generated/ProcessGraphChangedInfo";

export type {
  CleanupEventInfo,
  CleanupPolicyInfo,
  Page,
  ProcessCountsInfo,
  ProcessGraphChangedInfo,
  ProcessGraphRowInfo,
  ProcessQueryInput,
  ProcessStatusInfo,
  ProcessSnapshotInfo,
  ProcRowInfo,
  RollupInfo,
  RulePreviewInfo,
};

/** The event the backend sends when the graph changed. */
export const PROCESSES_CHANGED = "processes://changed";
/** AJ-E805: the counts alone. AJ-E812: told only when the top bar's
 * count (`to_clean`) moves. */
export const PROCESS_COUNTS = "processes://counts";

/** A query with every filter off (the page's default). */
export function processQuery(over: Partial<ProcessQueryInput> = {}): ProcessQueryInput {
  return {
    view: "attention",
    session_id: null,
    session_ids: [],
    states: [],
    app: null,
    account_id: null,
    project_id: null,
    has_port: false,
    heavy: false,
    show_internal: false,
    others: false,
    search: "",
    expanded: [],
    collapsed: [],
    ...over,
  };
}

/** The preview's answers (loaded only without a backend). */
const preview = () => import("../../preview/processes");

/** Every port open on this Mac (the Ports view). */
export async function processesPorts(): Promise<PortInventoryInfo> {
  if (!hasLiveBackend()) return (await preview()).previewPorts();
  return portInventory();
}

/** One page of the Processes view (`page`: the cursor of the page before
 * and how many rows, at most 200). */
export async function processesPage(input: ProcessQueryInput, page: PageRequest): Promise<Page<ProcessGraphRowInfo>> {
  if (!hasLiveBackend()) return (await preview()).previewPage(input, page);
  return call("processes_page", { input, page });
}

/** The view's state: its version, counts, and whether the ledger records. */
export async function processesStatus(): Promise<ProcessStatusInfo> {
  if (!hasLiveBackend()) return (await preview()).previewStatus();
  return call("processes_status");
}

/** The counts now (the top bar's and This Mac's "to clean up"). */
export async function processesCounts(): Promise<ProcessCountsInfo> {
  if (!hasLiveBackend()) return (await preview()).previewCounts();
  return call("processes_counts");
}

/** Keep (not a leftover), Remind me later for `minutes`, or neither. */
export async function processesMark(
  pid: number,
  startTime: number,
  mark: "keep" | "snooze" | null,
  minutes?: number,
): Promise<void> {
  if (!hasLiveBackend()) return (await preview()).previewMark(pid, startTime, mark, minutes);
  return call("processes_mark", { pid, startTime, mark, minutes: minutes ?? null });
}

/** D-41b: the verify sheet for another app's leftover (zero signals). */
export async function processesObservedVerify(pid: number, startTime: number | null): Promise<KillVerifyData> {
  if (!hasLiveBackend()) return (await preview()).previewObservedVerify(pid, startTime);
  return call("processes_observed_verify", { pid, startTime });
}

/** D-41b: the user's stop of another app's leftover (checked again, audited). */
export async function processesObservedStop(
  pid: number,
  startTime: number | null,
  attempt: string,
): Promise<KillExecuteData> {
  if (!hasLiveBackend()) return (await preview()).previewObservedStop(pid);
  return call("processes_observed_stop", { pid, startTime, attempt });
}

/** Cleanup history (7 days), one page, optionally one session's or project's. */
export async function processesHistory(
  sessionId: string | null,
  projectId: string | null,
  page: PageRequest,
): Promise<Page<CleanupEventInfo>> {
  if (!hasLiveBackend()) return (await preview()).previewHistory(sessionId, projectId, page.cursor);
  return call("processes_history", { sessionId, projectId, page });
}

/** What an account's automatic rule would tidy now (nothing is stopped). */
export async function processesDryRun(accountId: string, tier: 1 | 2): Promise<RulePreviewInfo> {
  if (!hasLiveBackend()) return (await preview()).previewDryRun(accountId, tier);
  return call("processes_dry_run", { accountId, tier });
}

/** Each account's automatic cleanup, with pause and the crash hold. */
export async function processesPolicy(): Promise<CleanupPolicyInfo> {
  if (!hasLiveBackend()) return (await preview()).previewPolicy();
  return call("processes_policy");
}

/** A snapshot to save: names and hashed command lines only. */
export async function processesSnapshot(): Promise<ProcessSnapshotInfo> {
  if (!hasLiveBackend()) return (await preview()).previewSnapshot();
  return call("processes_snapshot");
}

/** Calls `on` with the counts each time the graph changes (the Processes
 * page, which shows it), while the window can be seen (read once when it
 * shows: AJ-E805); returns the disposer. In the preview, a
 * `aij:processes-changed` window event stands in. */
export function onProcessesChanged(on: (counts: ProcessCountsInfo) => void): () => void {
  return (
    previewCounts(on) ??
    listenWhileShown<ProcessGraphChangedInfo>(PROCESSES_CHANGED, (change) => on(change.counts), countsAgain(on))
  );
}

/** Calls `on` with the counts when the top bar's count moves: the graph
 * changes on nearly every 2 s tick while sessions work, and a window that
 * listens to it is sent each (AJ-E805). AJ-E812: the event fires only
 * when `to_clean` moves, so its other counts may be stale: a reader of
 * any other count listens to the graph's event (`PROCESSES_CHANGED`,
 * `onProcessesChanged`) instead. */
export function onProcessCounts(on: (counts: ProcessCountsInfo) => void): () => void {
  return previewCounts(on) ?? listenWhileShown<ProcessCountsInfo>(PROCESS_COUNTS, on, countsAgain(on));
}

/** The counts read again (a window that shows again catches up). */
function countsAgain(on: (counts: ProcessCountsInfo) => void): () => void {
  return () => void processesCounts().then(on, () => undefined);
}

/** In the preview, the `aij:processes-changed` window event; live, `null`. */
function previewCounts(on: (counts: ProcessCountsInfo) => void): (() => void) | null {
  if (hasLiveBackend()) return null;
  const handler = (message: Event) => on((message as CustomEvent<ProcessGraphChangedInfo>).detail.counts);
  window.addEventListener("aij:processes-changed", handler);
  return () => window.removeEventListener("aij:processes-changed", handler);
}

/** Reveal folder: the session's folder in Finder (the one the backend
 * knows for it). False when it has none. */
export async function processesReveal(sessionKey: string): Promise<boolean> {
  if (!hasLiveBackend()) return false;
  return call("processes_reveal", { sessionKey });
}

/** Show in Activity Monitor (macOS's own). */
export async function processesActivityMonitor(): Promise<boolean> {
  if (!hasLiveBackend()) return false;
  return call("processes_activity_monitor");
}
