/**
 * Sessions (instances): the list, quality, stop, the dashboard stats and
 * the live update events.
 */
import { call, listen, liveBackend as hasBackend } from "./transport";
import { EVENTS } from "../bridge";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { InstanceQuotaInfo } from "../../types/generated/InstanceQuotaInfo";
import type { InstanceResourcesInfo } from "../../types/generated/InstanceResourcesInfo";
import type { QualitySignalsInfo } from "../../types/generated/QualitySignalsInfo";
import type { StatsInfo } from "../../types/generated/StatsInfo";
import { previewHooks } from "../preview-hooks";

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

/** The sessions changed (no payload: read the lists again). */
export function onSessionsUpdated(handler: () => void): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<null>(EVENTS.sessionsUpdated, () => handler())
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {
      // backend gone; callers poll via sessionsList
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
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
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<InstanceResourcesInfo[]>(EVENTS.sessionsResources, (event) =>
    handler(event.payload),
  )
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {
      // backend gone; the row payload still carries resource fields
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
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
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<InstanceQuotaInfo[]>(EVENTS.sessionsQuota, (event) =>
    handler(event.payload),
  )
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {
      // backend gone; the row payload still carries the quota field
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}
