/**
 * D-38 (needs the maintainer's ruling; off until then): asking the tool
 * itself for a profile's limits (Claude Code's own usage check, headless).
 * The preview and the tests read the seam `__AIJ_PROBE__` (per account);
 * without an entry it isn't offered, as in the app until it is ruled.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { ProbeStatusInfo } from "../../types/generated/ProbeStatusInfo";

export type { ProbeStatusInfo };

type Seam = Record<string, ProbeStatusInfo>;

const seam = (): Seam => (window as unknown as { __AIJ_PROBE__?: Seam }).__AIJ_PROBE__ ?? {};
const OFF: ProbeStatusInfo = { offered: false, state: "off", last_run_ms: null, last_outcome: null };

function mock(accountId: string, change?: Partial<ProbeStatusInfo>): ProbeStatusInfo {
  const all = seam();
  const entry = { ...(all[accountId] ?? OFF), ...change };
  if (all[accountId]) all[accountId] = entry;
  return entry;
}

/** Where the profile's probe stands. */
export async function probeStatus(accountId: string): Promise<ProbeStatusInfo> {
  if (!hasLiveBackend()) return mock(accountId);
  return invoke<ProbeStatusInfo>("quota_probe_status", { accountId });
}

/** Turns it on for the profile (and asks once). */
export async function probeTurnOn(accountId: string): Promise<ProbeStatusInfo> {
  if (!hasLiveBackend()) return mock(accountId, { state: "on", last_run_ms: Date.now(), last_outcome: "ran" });
  return invoke<ProbeStatusInfo>("quota_probe_turn_on", { accountId });
}

/** Turns it off for the profile. */
export async function probeTurnOff(accountId: string): Promise<ProbeStatusInfo> {
  if (!hasLiveBackend()) return mock(accountId, { state: "off" });
  return invoke<ProbeStatusInfo>("quota_probe_turn_off", { accountId });
}

/** Asks now ("Measure now"). */
export async function probeRun(accountId: string): Promise<ProbeStatusInfo> {
  if (!hasLiveBackend()) return mock(accountId, { last_run_ms: Date.now(), last_outcome: "ran" });
  return invoke<ProbeStatusInfo>("quota_probe_run", { accountId });
}
