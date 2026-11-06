/**
 * E14 machine health over IPC (AJ-E1401/E1403/E1407/E1408): how the Mac is
 * doing, This Mac, the launch advisor and the overload alerts. In a plain
 * browser (the design preview, Playwright) the mock serves a deterministic
 * fixture; `window.__AIJ_MACHINE__` picks the state ("plenty" | "tight" |
 * "pressure" | "unsampled", or a full payload).
 */
import { invoke, listen } from "./transport";
import { hasLiveBackend } from "../bridge";
import { instancesList } from "./instances";
import type { MachineFixture } from "../../preview/machine";
import type { MachineAdviceInfo } from "../../types/generated/MachineAdviceInfo";
import type { MachineHealthInfo } from "../../types/generated/MachineHealthInfo";
import type { OtherAppInfo } from "../../types/generated/OtherAppInfo";
import type { OverloadAlertInfo } from "../../types/generated/OverloadAlertInfo";
import type { ThisMacInfo } from "../../types/generated/ThisMacInfo";
import type { ToolTreeInfo } from "../../types/generated/ToolTreeInfo";
import { previewHooks } from "../preview-hooks";

interface MachineSeams {
  __AIJ_MACHINE__?: MachineFixture | MachineHealthInfo;
}
const machineSeams = () => window as unknown as MachineSeams;

/** Walking states by hand in the preview: `?aij_machine=tight`,
 * `?aij_load=heavy` (the first live session heavy, two leftovers). */
export function previewParam<T extends string>(name: string): T | undefined {
  if (typeof location === "undefined") return undefined;
  return (new URLSearchParams(location.search).get(name) as T | null) ?? undefined;
}

/** How the Mac is doing now. */
export async function machineHealth(): Promise<MachineHealthInfo> {
  if (!hasLiveBackend()) {
    const seeded = machineSeams().__AIJ_MACHINE__ ?? previewHooks().machine.asFixture(previewParam("aij_machine")) ?? "plenty";
    return typeof seeded === "string" ? previewHooks().machine.mockMachineHealth(seeded) : structuredClone(seeded);
  }
  return invoke<MachineHealthInfo>("machine_health");
}

interface ThisMacSeams {
  /** A full payload (tests), or nothing for the derived mock. */
  __AIJ_THIS_MAC__?: ThisMacInfo;
  /** The AI tools AI Juggler did not start, for the derived mock
   * (default: one Claude app window). */
  __AIJ_OBSERVED__?: ToolTreeInfo[];
  /** Leftover processes/ports for the derived mock (default 0/0). */
  __AIJ_LEFTOVERS__?: [number, number];
  /** Active overload alerts for the derived mock (AJ-E1408). */
  __AIJ_ALERTS__?: OverloadAlertInfo[];
  /** Alerts the next machineAlertsTake() hands out (AJ-E1408). */
  __AIJ_ALERTS_FRESH__?: OverloadAlertInfo[];
  /** Recommendations for the derived mock (AJ-E1409). */
  __AIJ_RECOMMENDATIONS__?: string[];
  /** The other apps for the derived mock (default: a few). */
  __AIJ_OTHER_APPS__?: OtherAppInfo[];
}
const thisMacSeams = () => window as unknown as ThisMacSeams;

/** This Mac now. */
export async function thisMac(): Promise<ThisMacInfo> {
  if (!hasLiveBackend()) {
    const seams = thisMacSeams();
    if (seams.__AIJ_THIS_MAC__) return structuredClone(seams.__AIJ_THIS_MAC__);
    let sessions = await instancesList();
    let leftovers = seams.__AIJ_LEFTOVERS__;
    if (previewParam("aij_load") === "heavy") {
      sessions = previewHooks().thisMac.withHeavyLoad(sessions);
      leftovers = leftovers ?? [2, 1];
    }
    const top = sessions.find((session) => session.status === "running");
    const others = previewHooks().machine.othersPreview(machineSeams().__AIJ_MACHINE__ ?? previewParam("aij_machine"));
    const alerts = seams.__AIJ_ALERTS__ ?? others?.alerts ?? previewHooks().thisMac.previewAlerts(top?.id ?? null);
    const advice = seams.__AIJ_RECOMMENDATIONS__ ?? (previewParam("aij_history") ? previewHooks().thisMac.PREVIEW_HISTORY : []);
    const observed = seams.__AIJ_OBSERVED__ ?? others?.observed ?? previewHooks().thisMac.previewWindows(Number(previewParam("aij_windows") ?? 0));
    return previewHooks().thisMac.mockThisMac(sessions, await machineHealth(), observed, leftovers, alerts, advice, seams.__AIJ_OTHER_APPS__ ?? others?.apps);
  }
  return invoke<ThisMacInfo>("this_mac");
}

/** "Can this Mac take another session?" for every launch setting. */
export async function machineAdvice(): Promise<MachineAdviceInfo[]> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_ADVICE__?: MachineAdviceInfo[] }).__AIJ_ADVICE__;
    if (seeded) return structuredClone(seeded);
    const { launchRecipesList } = await import("./launch");
    const [recipes, mac] = await Promise.all([launchRecipesList(), thisMac()]);
    const top = mac.top.find((tree) => tree.session_id !== null);
    return previewHooks().thisMac.mockMachineAdvice(
      recipes.map((recipe) => recipe.id),
      mac.machine,
      top ? { id: top.session_id!, bytes: top.rss_bytes } : null,
    );
  }
  return invoke<MachineAdviceInfo[]>("machine_advice");
}

// --- AJ-E1408: overload alerts ---------------------------------------------

/** The alerts to tell since the last call (the notification region's poll). */
export async function machineAlertsTake(): Promise<OverloadAlertInfo[]> {
  if (!hasLiveBackend()) {
    const seams = thisMacSeams();
    // `?aij_alert=…` tells its alert once per page load.
    const fresh = seams.__AIJ_ALERTS_FRESH__ ?? ((seams as { __AIJ_ALERT_TOLD__?: boolean }).__AIJ_ALERT_TOLD__ ? [] : previewHooks().thisMac.previewAlerts(null));
    seams.__AIJ_ALERTS_FRESH__ = [];
    (seams as { __AIJ_ALERT_TOLD__?: boolean }).__AIJ_ALERT_TOLD__ = true;
    return structuredClone(fresh);
  }
  return invoke<OverloadAlertInfo[]>("machine_alerts_take");
}

/** This Mac's 5 s `machine://this-mac` emission (live backends only). */
export function onThisMac(handler: (info: ThisMacInfo) => void): Promise<() => void> {
  return listen<ThisMacInfo>("machine://this-mac", (event) => handler(event.payload));
}
