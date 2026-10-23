/**
 * AJ-E1515 — the paired Macs as one place to work (ENGINEERING-GUIDE U3:
 * one thin file per feature): what New session can start on each paired
 * Mac, starting there, and each machine's usage in the last day. In the
 * preview, `__AIJ_LAUNCH_MACHINES__`, `__AIJ_MACHINE_USAGE__` and
 * `__AIJ_REMOTE_STARTED__` stand in.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import { NOTIFICATION_CLICK_EVENT } from "./notifications";
import type { LaunchMachineInfo } from "../../types/generated/LaunchMachineInfo";
import type { MachineUsageInfo } from "../../types/generated/MachineUsageInfo";
import type { RemoteCommandOutcomeInfo } from "../../types/generated/RemoteCommandOutcomeInfo";

export type { LaunchMachineInfo, MachineUsageInfo, RemoteCommandOutcomeInfo };

const seams = () =>
  window as unknown as {
    __AIJ_LAUNCH_MACHINES__?: LaunchMachineInfo[];
    __AIJ_MACHINE_USAGE__?: MachineUsageInfo[];
    __AIJ_REMOTE_STARTED__?: { machineId: string; recipeId: string; projectId: string | null };
    __AIJ_REMOTE_OUTCOME__?: RemoteCommandOutcomeInfo;
  };

/** What New session can start on each paired Mac (none unpaired). */
export async function fleetLaunchOptions(): Promise<LaunchMachineInfo[]> {
  if (!hasLiveBackend()) return seams().__AIJ_LAUNCH_MACHINES__ ?? [];
  return invoke<LaunchMachineInfo[]>("fleet_launch_options");
}

/** Starts a session on `machineId`; that Mac answers in its own words. A
 * started session opens in the app (the backend's `app://open`). */
export async function fleetLaunchStart(machineId: string, recipeId: string, projectId: string | null): Promise<RemoteCommandOutcomeInfo> {
  if (!hasLiveBackend()) {
    const s = seams();
    s.__AIJ_REMOTE_STARTED__ = { machineId, recipeId, projectId };
    const outcome = s.__AIJ_REMOTE_OUTCOME__ ?? { accepted: true, reason_code: null, message: "Started a session", started_instance_id: "remote-1" };
    if (outcome.started_instance_id) {
      window.dispatchEvent(
        new CustomEvent(NOTIFICATION_CLICK_EVENT, { detail: { session_id: outcome.started_instance_id, page: null, machine_id: machineId } }),
      );
    }
    return outcome;
  }
  return invoke<RemoteCommandOutcomeInfo>("fleet_launch_start", { machineId, recipeId, projectId });
}

/** Each machine's usage in the last day: this Mac first. */
export async function fleetUsageByMachine(): Promise<MachineUsageInfo[]> {
  if (!hasLiveBackend()) return seams().__AIJ_MACHINE_USAGE__ ?? [];
  return invoke<MachineUsageInfo[]>("fleet_usage_by_machine");
}
