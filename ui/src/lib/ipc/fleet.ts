/**
 * AJ-E702: Fleet identity and the paired-peer registry.
 */
import { call, liveBackend as hasBackend } from "./transport";
import type { FleetPeerInfo } from "../../types/generated/FleetPeerInfo";
import type { FleetSelfInfo } from "../../types/generated/FleetSelfInfo";


export type FleetSeams = {
  __AIJ_FLEET_SELF__?: Partial<FleetSelfInfo>;
  __AIJ_FLEET_PEERS__?: FleetPeerInfo[];
  __AIJ_FLEET_CALLS__?: string[];
};

const fleetSeams = () => window as unknown as FleetSeams;

export function mockFleetSelf(): FleetSelfInfo {
  return {
    machine_id: "machine-7f3a",
    label: "Studio Mac",
    fingerprint: "3F2A 9C01 B7E4 55D0 12AF 8C3E 0D91 66B2",
    trust: "trusted",
    trust_reason: null,
    peer_count: fleetSeams().__AIJ_FLEET_PEERS__?.length ?? 0,
    ...fleetSeams().__AIJ_FLEET_SELF__,
  };
}

export function recordFleetCall(called: string): void {
  const seams = fleetSeams();
  seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), called];
}

/** This Mac's Fleet identity (label, fingerprint, trust state). */
export async function fleetSelf(): Promise<FleetSelfInfo> {
  if (!hasBackend) return mockFleetSelf();
  return call("fleet_self");
}

/** The machines paired with this Mac. */
export async function fleetPeers(): Promise<FleetPeerInfo[]> {
  if (!hasBackend) return [...(fleetSeams().__AIJ_FLEET_PEERS__ ?? [])];
  return call("fleet_peers");
}

/** Unpairs one machine (deletes its pin). */
export async function fleetUnpair(machineId: string): Promise<void> {
  if (!hasBackend) {
    recordFleetCall(`unpair:${machineId}`);
    const seams = fleetSeams();
    seams.__AIJ_FLEET_PEERS__ = (seams.__AIJ_FLEET_PEERS__ ?? []).filter(
      (peer) => peer.machine_id !== machineId,
    );
    return;
  }
  return call("fleet_unpair", { machineId });
}

/** Renames this Mac locally. */
export async function fleetSetLabel(label: string): Promise<FleetSelfInfo> {
  if (!hasBackend) {
    recordFleetCall(`label:${label.trim()}`);
    fleetSeams().__AIJ_FLEET_SELF__ = { ...fleetSeams().__AIJ_FLEET_SELF__, label: label.trim() };
    return mockFleetSelf();
  }
  return call("fleet_set_label", { label });
}

/** Clears a broken registry so pairing can start again. */
export async function fleetResetTrust(): Promise<FleetSelfInfo> {
  if (!hasBackend) {
    recordFleetCall("reset-trust");
    fleetSeams().__AIJ_FLEET_PEERS__ = [];
    fleetSeams().__AIJ_FLEET_SELF__ = { ...fleetSeams().__AIJ_FLEET_SELF__, trust: "trusted", trust_reason: null };
    return mockFleetSelf();
  }
  return call("fleet_reset_trust");
}

/** Generates a new identity for this Mac; every peer must pair again. */
export async function fleetResetIdentity(): Promise<FleetSelfInfo> {
  if (!hasBackend) {
    recordFleetCall("reset-identity");
    fleetSeams().__AIJ_FLEET_PEERS__ = [];
    fleetSeams().__AIJ_FLEET_SELF__ = {
      ...fleetSeams().__AIJ_FLEET_SELF__,
      fingerprint: "A1C4 77E0 5B2D 9F13 C8E6 0447 DD21 9A5F",
    };
    return mockFleetSelf();
  }
  return call("fleet_reset_identity");
}
