/**
 * Stopping processes safely (AJ-E5xx): verify, execute, cancel,
 * liveness, and the port inventory.
 */
import { call, liveBackend as hasBackend } from "./transport";
import type { KillExecuteData, KillScope, KillVerifyData } from "../../components/KillSheet";

/** AJ-E505: one port-inventory row (classification is the key). */
export interface PortRowInfo {
  port: number;
  pid: number;
  class: string;
  session: string | null;
  provider: string | null;
  account: string | null;
  project: string | null;
  listener_age_ms: number | null;
  name: string | null;
  hint: string | null;
  killable: boolean;
}

/** AJ-E505: the inventory response (honest probe note + hog facts). */
export interface PortInventoryInfo {
  rows: PortRowInfo[];
  note: string | null;
  hogs: Array<{
    pid: number;
    session: string;
    provider: string;
    rss_bytes: number;
    cpu_pct: number | null;
    rule: string;
  }>;
}

/** AJ-E505: the pre-launch collision warning (copy-exact message). */
export interface PortWarningInfo {
  port: number;
  pid: number;
  instance: string;
  age_ms: number;
  message: string;
}

/** AJ-E507: the kill sheet's step-1 verify (zero signalling). */
export async function killVerify(sessionId: string, pid: number): Promise<KillVerifyData> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_KILL_VERIFY__?: (session: string, pid: number) => KillVerifyData })
      .__AIJ_KILL_VERIFY__?.(sessionId, pid) ?? {
      pid,
      session: sessionId,
      verified: false,
      refusal: "This PID now belongs to a different program — Juggler will not touch it.",
      exe_path: "?",
      cmdline: "",
      cwd: "",
      ports: [],
      pgid: null,
      escaped_group: false,
      members: [],
      tree_pids: [],
      platform: "TERM → grace → KILL",
      grace_secs: 5,
      provider: "unknown",
      account_id: null,
      project_id: null,
    };
  }
  return call("kill_verify", { sessionId, pid });
}

/** AJ-E507: the audited TERM→grace→KILL ladder (step 3). */
export async function killExecute(
  sessionId: string,
  pid: number,
  scope: KillScope,
  attempt: string,
  goalPort: number | null,
): Promise<KillExecuteData> {
  if (!hasBackend) {
    const fixture = (
      window as unknown as {
        __AIJ_KILL_EXECUTE__?: (scope: KillScope, goalPort: number | null) => KillExecuteData;
      }
    ).__AIJ_KILL_EXECUTE__;
    if (fixture !== undefined) {
      return fixture(scope, goalPort);
    }
    // The countdown state's seam: a ladder that never resolves.
    if ((window as unknown as { __AIJ_KILL_EXECUTE_PENDING__?: boolean }).__AIJ_KILL_EXECUTE_PENDING__) {
      return new Promise<KillExecuteData>(() => {});
    }
    return {
      refusal: null,
      members: [{ pid, outcome: "already gone" }],
      cancelled: false,
      all_succeeded: true,
      still_holding: [],
      reoffer_pid: null,
    };
  }
  return call("kill_execute", { sessionId, pid, scope, attempt, goalPort });
}

/** AJ-E507: cancel a running ladder's grace window. */
export async function killCancel(attempt: string): Promise<void> {
  if (!hasBackend) {
    return Promise.resolve();
  }
  return call("kill_cancel", { attempt });
}

/** AJ-E507: the grace window's per-process exit ticks. */
export async function killLiveness(pids: number[]): Promise<boolean[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_KILL_LIVENESS__?: (pids: number[]) => boolean[] })
      .__AIJ_KILL_LIVENESS__?.(pids) ?? pids.map(() => false);
  }
  return call("kill_liveness", { pids });
}

/** AJ-E505: the port occupancy map. */
export async function portInventory(): Promise<PortInventoryInfo> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_PORTS__?: PortInventoryInfo }).__AIJ_PORTS__
      ?? { rows: [], note: null, hogs: [] };
  }
  return call("port_inventory");
}

/** AJ-E505: the launch pre-check — a warning NEVER blocks launch. */
export async function launchPortWarning(projectId: string | null): Promise<PortWarningInfo | null> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_PORT_WARNING__?: PortWarningInfo }).__AIJ_PORT_WARNING__
      ?? null;
  }
  return call("launch_port_warning", { projectId });
}
