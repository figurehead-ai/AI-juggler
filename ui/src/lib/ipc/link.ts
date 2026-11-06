/**
 * AJ-E704: Link on/off, discovery and pairing.
 */
import { invoke, liveBackend as hasBackend } from "./transport";
import { type FleetSeams, mockFleetSelf, recordFleetCall } from "./fleet";
import type { DiscoveredMachineInfo } from "../../types/generated/DiscoveredMachineInfo";
import type { FleetPeerInfo } from "../../types/generated/FleetPeerInfo";
import type { LinkStatusInfo } from "../../types/generated/LinkStatusInfo";
import type { PairingStateInfo } from "../../types/generated/PairingStateInfo";
import { previewHooks } from "../preview-hooks";

//
// The mock is a small scripted state machine so the wizard can be
// driven end-to-end in the browser (Playwright, gallery):
// `window.__AIJ_PAIR_SIMULATE__("joiner" | "mismatch" | "expire" |
// "cancel")` plays the other machine's side of an "Add a machine"
// window; a joiner code of 000000 fails as a mismatch, and
// `__AIJ_PAIR_JOIN_FAIL__ = "<reason>"` fails a join with that reason.

/** What the permissions step grants the other machine. */
export type PairPermission = "view" | "control";

type LinkSeams = FleetSeams & {
  __AIJ_LINK__?: LinkStatusInfo;
  __AIJ_DISCOVERED__?: DiscoveredMachineInfo[];
  __AIJ_PAIR_HOST__?: PairingStateInfo;
  __AIJ_PAIR_JOIN__?: PairingStateInfo;
  __AIJ_PAIR_LOCK__?: "locked_out" | "disabled";
  __AIJ_PAIR_JOIN_FAIL__?: string;
  __AIJ_PAIR_SIMULATE__?: (event: string) => void;
};

const linkSeams = () => window as unknown as LinkSeams;

const idlePairing = (): PairingStateInfo => ({
  phase: "idle",
  code: null,
  link: null,
  expires_at_ms: null,
  peer_label: null,
  peer_fingerprint: null,
  message: null,
  reason: null,
});

function mockLink(): LinkStatusInfo {
  return linkSeams().__AIJ_LINK__ ?? { enabled: false, port: null, error: null };
}

function mockWaiting(code: string, reason: string | null, message: string | null): PairingStateInfo {
  return {
    ...idlePairing(),
    phase: "waiting",
    code,
    link: `aijuggler-pair:v1:${code.replace(" ", "")}:${mockFleetSelf().fingerprint.replace(/ /g, "")}`,
    expires_at_ms: Date.now() + 120_000,
    reason,
    message,
  };
}

function mockPin(label: string, fingerprint: string, permission: PairPermission): void {
  const seams = linkSeams();
  const id = `machine-${label.toLowerCase().replace(/[^a-z]/g, "")}`;
  const peer: FleetPeerInfo = {
    machine_id: id,
    label,
    fingerprint,
    permission,
    permission_label: permission === "control" ? "View + control" : "View only",
    ask_each_time: false,
    paired_via: "code",
    paired_at_ms: Date.now(),
    last_seen_ms: Date.now(),
    connection: mockLink().enabled ? "connected" : "link_off",
    protocol: mockLink().enabled ? "v1.0" : null,
    connection_detail: null,
    queued_changes: null,
    pairing_unfinished: false,
  };
  seams.__AIJ_FLEET_PEERS__ = [
    ...(seams.__AIJ_FLEET_PEERS__ ?? []).filter((existing) => existing.machine_id !== id),
    peer,
  ];
}

function installPairSimulator(): void {
  const seams = linkSeams();
  if (seams.__AIJ_PAIR_SIMULATE__) return;
  seams.__AIJ_PAIR_SIMULATE__ = (event: string) => {
    const host = seams.__AIJ_PAIR_HOST__ ?? idlePairing();
    if (event === "joiner" && host.phase === "waiting") {
      seams.__AIJ_PAIR_HOST__ = {
        ...idlePairing(),
        phase: "confirm",
        peer_label: previewHooks().fleet.MOCK_JOINER.label,
        peer_fingerprint: previewHooks().fleet.MOCK_JOINER.fingerprint,
      };
    } else if (event === "mismatch") {
      seams.__AIJ_PAIR_HOST__ = mockWaiting("731 058", "mismatch", "That code didn't match — here's a new one.");
    } else if (event === "expire") {
      seams.__AIJ_PAIR_HOST__ = {
        ...idlePairing(),
        phase: "failed",
        reason: "expired",
        message: "The pairing window closed after 2 minutes.",
      };
    } else if (event === "cancel") {
      seams.__AIJ_PAIR_HOST__ = mockWaiting(
        host.code ?? "482 913",
        "cancelled",
        "Pairing was cancelled on one of the machines — the code still works.",
      );
    }
  };
}

/** Link status (off by default). */
export async function linkStatus(): Promise<LinkStatusInfo> {
  if (!hasBackend) return mockLink();
  return invoke<LinkStatusInfo>("link_status");
}

/** Turns Link on or off (a user click; remembered). */
export async function linkSetEnabled(enabled: boolean): Promise<LinkStatusInfo> {
  if (!hasBackend) {
    recordFleetCall(`link:${enabled ? "on" : "off"}`);
    linkSeams().__AIJ_LINK__ = { enabled, port: enabled ? 47821 : null, error: null };
    const seams = linkSeams();
    seams.__AIJ_FLEET_PEERS__ = (seams.__AIJ_FLEET_PEERS__ ?? []).map((peer) => ({
      ...peer,
      connection: enabled ? (peer.connection === "link_off" ? "connected" : peer.connection) : "link_off",
      protocol: enabled ? (peer.protocol ?? "v1.0") : null,
    }));
    return mockLink();
  }
  return invoke<LinkStatusInfo>("link_set_enabled", { enabled });
}

/** Machines on this network that aren't paired. */
export async function fleetDiscovered(): Promise<DiscoveredMachineInfo[]> {
  if (!hasBackend) return mockLink().enabled ? [...(linkSeams().__AIJ_DISCOVERED__ ?? [])] : [];
  return invoke<DiscoveredMachineInfo[]>("fleet_discovered");
}

/** Opens "Add a machine" (shows a code for 2 minutes). */
export async function pairingHostOpen(): Promise<PairingStateInfo> {
  if (!hasBackend) {
    recordFleetCall("pair-host-open");
    installPairSimulator();
    const lock = linkSeams().__AIJ_PAIR_LOCK__;
    linkSeams().__AIJ_PAIR_HOST__ = lock
      ? {
          ...idlePairing(),
          phase: "failed",
          reason: lock,
          message:
            lock === "disabled"
              ? "Pairing is turned off after too many wrong codes today — turn it back on in Fleet."
              : "Too many wrong codes — pairing is paused for a minute.",
        }
      : mockWaiting("482 913", null, null);
    return { ...linkSeams().__AIJ_PAIR_HOST__! };
  }
  return invoke<PairingStateInfo>("pairing_host_open");
}

/** The host's pairing state (polled while the wizard is open). */
export async function pairingHostState(): Promise<PairingStateInfo> {
  if (!hasBackend) return { ...(linkSeams().__AIJ_PAIR_HOST__ ?? idlePairing()) };
  return invoke<PairingStateInfo>("pairing_host_state");
}

/** The host user's decision: pair (granting `permission`) or cancel. */
export async function pairingHostDecide(commit: boolean, permission: PairPermission): Promise<PairingStateInfo> {
  if (!hasBackend) {
    recordFleetCall(`pair-host-decide:${commit ? permission : "cancel"}`);
    const host = linkSeams().__AIJ_PAIR_HOST__ ?? idlePairing();
    if (commit && host.phase === "confirm") {
      mockPin(host.peer_label ?? "Laptop", host.peer_fingerprint ?? "", permission);
      linkSeams().__AIJ_PAIR_HOST__ = { ...idlePairing(), phase: "paired", peer_label: host.peer_label };
    } else {
      linkSeams().__AIJ_PAIR_HOST__ = mockWaiting(
        "482 913",
        "cancelled",
        "Pairing was cancelled on one of the machines — the code still works.",
      );
    }
    return { ...linkSeams().__AIJ_PAIR_HOST__! };
  }
  return invoke<PairingStateInfo>("pairing_host_decide", { commit, permission });
}

/** Closes "Add a machine". */
export async function pairingHostClose(): Promise<void> {
  if (!hasBackend) {
    recordFleetCall("pair-host-close");
    linkSeams().__AIJ_PAIR_HOST__ = idlePairing();
    return;
  }
  return invoke<void>("pairing_host_close");
}

/** Turns pairing back on after the daily disable (a user click). */
export async function pairingReEnable(): Promise<void> {
  if (!hasBackend) {
    recordFleetCall("pair-re-enable");
    linkSeams().__AIJ_PAIR_LOCK__ = undefined;
    return;
  }
  return invoke<void>("pairing_re_enable");
}

/** Starts joining a machine found on the network, or at an address. */
export async function pairingJoinStart(
  target: { instanceId: string } | { address: string },
  code: string,
): Promise<PairingStateInfo> {
  if (!hasBackend) {
    recordFleetCall(`pair-join:${"address" in target ? target.address : target.instanceId}`);
    const digits = code.startsWith("aijuggler-pair:") ? code.split(":")[2] ?? "" : code.replace(/\D/g, "");
    const fail = linkSeams().__AIJ_PAIR_JOIN_FAIL__ ?? (digits === "000000" ? "mismatch" : undefined);
    linkSeams().__AIJ_PAIR_JOIN__ = fail
      ? { ...idlePairing(), phase: "failed", reason: fail, message: null }
      : {
          ...idlePairing(),
          phase: "confirm",
          peer_label: previewHooks().fleet.MOCK_HOST.label,
          peer_fingerprint: previewHooks().fleet.MOCK_HOST.fingerprint,
        };
    return { ...linkSeams().__AIJ_PAIR_JOIN__! };
  }
  return invoke<PairingStateInfo>("pairing_join_start", {
    instanceId: "instanceId" in target ? target.instanceId : null,
    address: "address" in target ? target.address : null,
    code,
  });
}

/** The joiner's pairing state (polled while the wizard is open). */
export async function pairingJoinState(): Promise<PairingStateInfo> {
  if (!hasBackend) return { ...(linkSeams().__AIJ_PAIR_JOIN__ ?? idlePairing()) };
  return invoke<PairingStateInfo>("pairing_join_state");
}

/** The joiner user's decision: pair (granting `permission`) or cancel. */
export async function pairingJoinDecide(commit: boolean, permission: PairPermission): Promise<PairingStateInfo> {
  if (!hasBackend) {
    recordFleetCall(`pair-join-decide:${commit ? permission : "cancel"}`);
    const join = linkSeams().__AIJ_PAIR_JOIN__ ?? idlePairing();
    if (commit && join.phase === "confirm") {
      mockPin(join.peer_label ?? "Mac mini", join.peer_fingerprint ?? "", permission);
      linkSeams().__AIJ_PAIR_JOIN__ = { ...idlePairing(), phase: "paired", peer_label: join.peer_label };
    } else {
      linkSeams().__AIJ_PAIR_JOIN__ = { ...idlePairing(), phase: "failed", reason: "cancelled", message: null };
    }
    return { ...linkSeams().__AIJ_PAIR_JOIN__! };
  }
  return invoke<PairingStateInfo>("pairing_join_decide", { commit, permission });
}

/** Resets the joiner (Done / Start over / Close). */
export async function pairingJoinReset(): Promise<void> {
  if (!hasBackend) {
    linkSeams().__AIJ_PAIR_JOIN__ = idlePairing();
    return;
  }
  return invoke<void>("pairing_join_reset");
}
