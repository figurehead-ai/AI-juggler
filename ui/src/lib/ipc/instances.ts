/**
 * Sessions (instances): the list, quality, stop, the dashboard stats and
 * the live update events.
 */
import { call, liveBackend as hasBackend } from "./transport";
import { EVENTS } from "../bridge";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { InstanceQuotaInfo } from "../../types/generated/InstanceQuotaInfo";
import type { InstanceResourcesInfo } from "../../types/generated/InstanceResourcesInfo";
import type { QualitySignalsInfo } from "../../types/generated/QualitySignalsInfo";
import type { StatsInfo } from "../../types/generated/StatsInfo";
import { previewHooks } from "../preview-hooks";
import { listenWhileShown } from "../shown-window";

export async function instancesList(): Promise<InstanceInfo[]> {
  if (!hasBackend) {
    const hang = previewHooks().support.loadHang();
    if (hang) return hang;
    return [...previewHooks().fixtures.MOCK_SESSIONS];
  }
  return call("instances_list");
}

/** @deprecated legacy alias (AJ-E107) — use instancesList. */
export const sessionsList = instancesList;

/** AJ-E409: D-07 quality signals for the selected Instance (no score). */
export async function instanceQuality(sessionId: string): Promise<QualitySignalsInfo | null> {
  if (!hasBackend) {
    return previewHooks().fixtures.MOCK_QUALITY[sessionId] ?? null;
  }
  return call("instance_quality", { sessionId });
}

export async function stopSession(sessionId: string): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await call("stop_session", { sessionId });
}

export async function dashboardStats(): Promise<StatsInfo> {
  if (!hasBackend) {
    return previewHooks().fixtures.MOCK_STATS;
  }
  return call("dashboard_stats");
}

/** The preview's "the sessions changed" (no backend sends one there). */
export const PREVIEW_SESSIONS_UPDATED = "aij:preview-sessions-updated";

/** The sessions changed (no payload: read the lists again); not heard
 * while the window is hidden, read again once when it shows (AJ-E805). */
export function onSessionsUpdated(handler: () => void): () => void {
  if (!hasBackend) {
    // The preview and the specs' seam (round 7 #6): a session started or
    // ended in a fixture says so.
    window.addEventListener(PREVIEW_SESSIONS_UPDATED, handler);
    return () => window.removeEventListener(PREVIEW_SESSIONS_UPDATED, handler);
  }
  return listenWhileShown<null>(EVENTS.sessionsUpdated, () => handler(), handler);
}

/**
 * 5 s resource tick (AJ-E401): coalesced I-07/I-08 deltas for live
 * Instances — merge by `id` into the session rows; absent ids keep
 * their honest `null`s.
 */
export function onSessionsResources(
  handler: (deltas: InstanceResourcesInfo[]) => void,
): () => void {
  if (!hasBackend) {
    return () => {};
  }
  // Not heard while hidden (AJ-E805): the rows read when it shows carry
  // the resources.
  return listenWhileShown<InstanceResourcesInfo[]>(EVENTS.sessionsResources, handler, () => undefined);
}

/**
 * 30 s quota tick (AJ-E401): coalesced I-05 deltas — merge by `id`;
 * `quota: null` is the honest "no feed exists for this Provider".
 */
export function onSessionsQuota(
  handler: (deltas: InstanceQuotaInfo[]) => void,
): () => void {
  if (!hasBackend) {
    return () => {};
  }
  // Not heard while hidden (AJ-E805): the rows read when it shows carry
  // the quota.
  return listenWhileShown<InstanceQuotaInfo[]>(EVENTS.sessionsQuota, handler, () => undefined);
}
