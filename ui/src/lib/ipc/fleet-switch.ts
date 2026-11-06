/**
 * AJ-E706: cross-machine switching.
 */
import { invoke, listen, liveBackend as hasBackend } from "./transport";
import { EVENTS } from "../bridge";
import type { DivergenceRecordInfo } from "../../types/generated/DivergenceRecordInfo";
import type { FleetSwitchBackPlanInfo } from "../../types/generated/FleetSwitchBackPlanInfo";
import type { FleetSwitchOutcomeInfo } from "../../types/generated/FleetSwitchOutcomeInfo";
import type { FleetSwitchProgressInfo } from "../../types/generated/FleetSwitchProgressInfo";
import type { FleetSwitchRequest } from "../../types/generated/FleetSwitchRequest";
import type { RemoteSwitchTargetInfo } from "../../types/generated/RemoteSwitchTargetInfo";


type CrossSeams = {
  __AIJ_FLEET_SWITCH_TARGETS__?: RemoteSwitchTargetInfo[];
  __AIJ_FLEET_SWITCH_OUTCOME__?: FleetSwitchOutcomeInfo;
  __AIJ_FLEET_SWITCH_BACK_PLAN__?: FleetSwitchBackPlanInfo | null;
  __AIJ_FLEET_SWITCH_BACK_OUTCOMES__?: FleetSwitchOutcomeInfo[];
  __AIJ_FLEET_DIVERGENCES__?: DivergenceRecordInfo[];
  __AIJ_FLEET_CALLS__?: string[];
  __AIJ_FLEET_SWITCH_PROGRESS__?: (update: FleetSwitchProgressInfo) => void;
  /** The push never ends (tests of Esc mid-push and the progress shot). */
  __AIJ_FLEET_SWITCH_HANG__?: boolean;
};

const crossSeams = () => window as unknown as CrossSeams;
const crossCall = (call: string) => {
  const seams = crossSeams();
  seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), call];
};

/** Paired machines as switch destinations for one source Instance —
 * computed from what they published; no transcript moves to list them. */
export async function fleetSwitchTargets(sessionId: string): Promise<RemoteSwitchTargetInfo[]> {
  if (!hasBackend) return [...(crossSeams().__AIJ_FLEET_SWITCH_TARGETS__ ?? [])];
  return invoke<RemoteSwitchTargetInfo[]>("fleet_switch_targets", { sessionId });
}

/** Runs one confirmed cross-machine switch (after the user's click). */
export async function fleetSwitchExecute(request: FleetSwitchRequest): Promise<FleetSwitchOutcomeInfo> {
  if (!hasBackend) {
    crossCall(`switch:${request.machine_id}:${request.recipe_id}:${request.mechanism}:${request.project_id ?? ""}`);
    const progress = crossSeams().__AIJ_FLEET_SWITCH_PROGRESS__;
    progress?.({ machine_id: request.machine_id, phase: "receiving", received_bytes: 6_000_000, total_bytes: 12_000_000 });
    if (crossSeams().__AIJ_FLEET_SWITCH_HANG__) return new Promise<FleetSwitchOutcomeInfo>(() => {});
    return (
      crossSeams().__AIJ_FLEET_SWITCH_OUTCOME__ ?? {
        accepted: true,
        reason_code: null,
        message: "Continuing under Desktop Max in storefront",
        instance_id: "sess-remote-new",
        edge_id: "edge-x",
        divergence: null,
      }
    );
  }
  return invoke<FleetSwitchOutcomeInfo>("fleet_switch_execute", { request });
}

/** Cancels the push to `machineId` (within one chunk). */
export async function fleetSwitchCancel(machineId: string): Promise<boolean> {
  if (!hasBackend) {
    crossCall(`switch-cancel:${machineId}`);
    return true;
  }
  return invoke<boolean>("fleet_switch_cancel", { machineId });
}

/** Push progress while a cross-machine switch runs. */
export function onFleetSwitchProgress(handler: (update: FleetSwitchProgressInfo) => void): () => void {
  if (!hasBackend) {
    crossSeams().__AIJ_FLEET_SWITCH_PROGRESS__ = handler;
    return () => {
      delete crossSeams().__AIJ_FLEET_SWITCH_PROGRESS__;
    };
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<FleetSwitchProgressInfo>(EVENTS.fleetSwitchProgress, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {
      /* backend gone */
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** The switch-back plan for an Instance that came from another machine
 * (`null` for any other Instance). */
export async function fleetSwitchBackPlan(sessionId: string): Promise<FleetSwitchBackPlanInfo | null> {
  if (!hasBackend) return crossSeams().__AIJ_FLEET_SWITCH_BACK_PLAN__ ?? null;
  return invoke<FleetSwitchBackPlanInfo | null>("fleet_switch_back_plan", { sessionId });
}

/** Switches an away Instance back to its original machine; `choice` is
 * empty, or `keep_mine` / `keep_both` after a diverged refusal. */
export async function fleetSwitchBackExecute(
  sessionId: string,
  choice: "" | "keep_mine" | "keep_both",
  keepAwayRunning: boolean,
): Promise<FleetSwitchOutcomeInfo> {
  if (!hasBackend) {
    crossCall(`switch-back:${sessionId}:${choice}`);
    const queue = crossSeams().__AIJ_FLEET_SWITCH_BACK_OUTCOMES__ ?? [];
    const next = queue.shift();
    return (
      next ?? {
        accepted: true,
        reason_code: null,
        message: "Resumed the original under Work Max.",
        instance_id: "sess-back",
        edge_id: "edge-back",
        divergence: null,
      }
    );
  }
  return invoke<FleetSwitchOutcomeInfo>("fleet_switch_back_execute", { sessionId, choice, keepAwayRunning });
}

/** Every unresolved divergence (the sync chip's "Diverged — N needs you"). */
export async function fleetDivergences(): Promise<DivergenceRecordInfo[]> {
  if (!hasBackend) return [...(crossSeams().__AIJ_FLEET_DIVERGENCES__ ?? [])];
  return invoke<DivergenceRecordInfo[]>("fleet_divergences");
}

/** "Keep theirs": leave both sides as they are (nothing is sent). */
export async function fleetDivergenceKeepTheirs(sessionId: string): Promise<void> {
  if (!hasBackend) {
    crossCall(`keep-theirs:${sessionId}`);
    const seams = crossSeams();
    seams.__AIJ_FLEET_DIVERGENCES__ = (seams.__AIJ_FLEET_DIVERGENCES__ ?? []).filter(
      (row) => row.session_id !== sessionId,
    );
    return;
  }
  return invoke<void>("fleet_divergence_keep_theirs", { sessionId });
}
