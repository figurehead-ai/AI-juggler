/**
 * AJ-E705/E709: what paired machines report, and permissions changed on
 * this Mac ("Ask each time" prompts).
 */
import { invoke, liveBackend as hasBackend } from "./transport";
import type { FleetPeerInfo } from "../../types/generated/FleetPeerInfo";
import type { FleetPromptInfo } from "../../types/generated/FleetPromptInfo";
import type { RemoteCommandOutcomeInfo } from "../../types/generated/RemoteCommandOutcomeInfo";
import type { RemoteMachineInfo } from "../../types/generated/RemoteMachineInfo";


type RemoteSeams = {
  __AIJ_FLEET_REMOTE__?: RemoteMachineInfo[];
  __AIJ_REMOTE_STOP__?: RemoteCommandOutcomeInfo;
  __AIJ_FLEET_CALLS__?: string[];
};

const remoteSeams = () => window as unknown as RemoteSeams;

/** Every paired machine with what it last reported (source + freshness). */
export async function fleetRemote(): Promise<RemoteMachineInfo[]> {
  if (!hasBackend) return [...(remoteSeams().__AIJ_FLEET_REMOTE__ ?? [])];
  return invoke<RemoteMachineInfo[]>("fleet_remote");
}

/** AJ-E707: asks a paired machine to clean one process it owns (it
 * re-verifies and runs its own ladder). */
export async function fleetRemoteClean(
  machineId: string,
  instanceId: string,
  pid: number,
): Promise<RemoteCommandOutcomeInfo> {
  if (!hasBackend) {
    const seams = remoteSeams() as RemoteSeams & { __AIJ_REMOTE_CLEAN__?: RemoteCommandOutcomeInfo };
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `remote-clean:${machineId}:${instanceId}:${pid}`];
    return seams.__AIJ_REMOTE_CLEAN__ ?? { accepted: true, reason_code: null, message: `Exited (pid ${pid})` };
  }
  return invoke<RemoteCommandOutcomeInfo>("fleet_remote_clean", { machineId, instanceId, pid });
}

/** AJ-E711: asks a paired machine to bring one of its desktop windows
 * forward — executed there, with its permissions. */
export async function fleetRemoteFocus(machineId: string, instanceId: string): Promise<RemoteCommandOutcomeInfo> {
  if (!hasBackend) {
    const seams = remoteSeams() as RemoteSeams & { __AIJ_REMOTE_FOCUS__?: RemoteCommandOutcomeInfo };
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `remote-focus:${machineId}:${instanceId}`];
    return seams.__AIJ_REMOTE_FOCUS__ ?? { accepted: true, reason_code: null, message: "Focused window" };
  }
  return invoke<RemoteCommandOutcomeInfo>("fleet_remote_focus", { machineId, instanceId });
}

/** Asks a paired machine to stop one of its Instances; the node decides. */
export async function fleetRemoteStop(machineId: string, instanceId: string): Promise<RemoteCommandOutcomeInfo> {
  if (!hasBackend) {
    const seams = remoteSeams();
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `remote-stop:${machineId}:${instanceId}`];
    return (
      seams.__AIJ_REMOTE_STOP__ ?? { accepted: true, reason_code: null, message: "Stopping" }
    );
  }
  return invoke<RemoteCommandOutcomeInfo>("fleet_remote_stop", { machineId, instanceId });
}


type PromptSeams = {
  __AIJ_FLEET_PROMPTS__?: FleetPromptInfo[];
  __AIJ_FLEET_CALLS__?: string[];
  __AIJ_FLEET_PEERS__?: FleetPeerInfo[];
};
const promptSeams = () => window as unknown as PromptSeams;

/** What a paired machine may do on this Mac (the backend's wire words). */
export type FleetPermission = "no_access" | "view" | "control";

/** The words each permission shows as (the backend's own labels). */
const FLEET_PERMISSION_LABELS: Record<FleetPermission, string> = {
  no_access: "No access",
  view: "View only",
  control: "View + control",
};

/** Changes what a paired machine may do on THIS Mac (only ever here). */
export async function fleetSetPermission(
  machineId: string,
  permission: FleetPermission,
  askEachTime: boolean,
): Promise<FleetPeerInfo[]> {
  if (!hasBackend) {
    const seams = promptSeams();
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `permission:${machineId}:${permission}:${askEachTime}`];
    seams.__AIJ_FLEET_PEERS__ = (seams.__AIJ_FLEET_PEERS__ ?? []).map((peer) =>
      peer.machine_id === machineId
        ? {
            ...peer,
            permission,
            permission_label: FLEET_PERMISSION_LABELS[permission],
            ask_each_time: permission === "control" && askEachTime,
          }
        : peer,
    );
    return [...(seams.__AIJ_FLEET_PEERS__ ?? [])];
  }
  return invoke<FleetPeerInfo[]>("fleet_set_permission", { machineId, permission, askEachTime });
}

/** Control commands waiting for this Mac's user ("Ask each time"). */
export async function fleetPrompts(): Promise<FleetPromptInfo[]> {
  if (!hasBackend) return [...(promptSeams().__AIJ_FLEET_PROMPTS__ ?? [])];
  return invoke<FleetPromptInfo[]>("fleet_prompts");
}

/** Allows or denies a waiting command. */
export async function fleetPromptDecide(id: number, allow: boolean): Promise<void> {
  if (!hasBackend) {
    const seams = promptSeams();
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `prompt:${id}:${allow ? "allow" : "deny"}`];
    seams.__AIJ_FLEET_PROMPTS__ = (seams.__AIJ_FLEET_PROMPTS__ ?? []).filter((p) => p.id !== id);
    return;
  }
  return invoke<void>("fleet_prompt_decide", { id, allow });
}
