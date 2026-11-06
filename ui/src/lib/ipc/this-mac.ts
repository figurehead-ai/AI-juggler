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
import { mockThisMac, previewWindows, withHeavyLoad } from "../../preview/this-mac";
import { asFixture, mockMachineHealth, othersPreview, type MachineFixture } from "../../preview/machine";
import type { MachineAdviceInfo } from "../../types/generated/MachineAdviceInfo";
import type { MachineHealthInfo } from "../../types/generated/MachineHealthInfo";
import type { OtherAppInfo } from "../../types/generated/OtherAppInfo";
import type { OverloadAlertInfo } from "../../types/generated/OverloadAlertInfo";
import type { ThisMacInfo } from "../../types/generated/ThisMacInfo";
import type { ToolTreeInfo } from "../../types/generated/ToolTreeInfo";

interface MachineSeams {
  __AIJ_MACHINE__?: MachineFixture | MachineHealthInfo;
}
const machineSeams = () => window as unknown as MachineSeams;

/** Walking states by hand in the preview: `?aij_machine=tight`,
 * `?aij_load=heavy` (the first live session heavy, two leftovers). */
function previewParam<T extends string>(name: string): T | undefined {
  if (typeof location === "undefined") return undefined;
  return (new URLSearchParams(location.search).get(name) as T | null) ?? undefined;
}

/** `?aij_history=1`: advice as from a dozen measured sessions. */
const PREVIEW_HISTORY = [
  "This Mac comfortably runs about 5 sessions at once (about 2.1 GB each, from your last 12)",
  "\u201cDeep work\u201d (Work Max) is the heaviest launch setting \u2014 about 3.1 GB a session",
];

/** `?aij_alert=memory|cpu`: one sustained overload alert. */
function previewAlerts(sessionId: string | null): OverloadAlertInfo[] {
  const kind = previewParam("aij_alert");
  if (kind === "memory")
    return [
      { key: "memory_pressure", kind: "memory_pressure", title: "Your Mac is under memory pressure", detail: "macOS reports memory pressure", session_id: sessionId, place: null, subject_name: null, subject_bytes: 3_200_000_000, subject_estimate: false, subject_cpu: null, subject_windows: null, other_apps: [], since_ms: Date.now() - 45_000 },
    ];
  if (kind === "cpu")
    return [
      { key: "cpu", kind: "cpu", title: "The processor has been flat out for minutes", detail: "97% busy for over two minutes", session_id: null, place: "claude-app", subject_name: "The Claude app", subject_bytes: null, subject_estimate: false, subject_cpu: 140, subject_windows: null, other_apps: [], since_ms: Date.now() - 150_000 },
    ];
  return [];
}


/** How the Mac is doing now. */
export async function machineHealth(): Promise<MachineHealthInfo> {
  if (!hasLiveBackend()) {
    const seeded = machineSeams().__AIJ_MACHINE__ ?? asFixture(previewParam("aij_machine")) ?? "plenty";
    return typeof seeded === "string" ? mockMachineHealth(seeded) : structuredClone(seeded);
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
      sessions = withHeavyLoad(sessions);
      leftovers = leftovers ?? [2, 1];
    }
    const top = sessions.find((session) => session.status === "running");
    const others = othersPreview(machineSeams().__AIJ_MACHINE__ ?? previewParam("aij_machine"));
    const alerts = seams.__AIJ_ALERTS__ ?? others?.alerts ?? previewAlerts(top?.id ?? null);
    const advice = seams.__AIJ_RECOMMENDATIONS__ ?? (previewParam("aij_history") ? PREVIEW_HISTORY : []);
    const observed = seams.__AIJ_OBSERVED__ ?? others?.observed ?? previewWindows(Number(previewParam("aij_windows") ?? 0));
    return mockThisMac(sessions, await machineHealth(), observed, leftovers, alerts, advice, seams.__AIJ_OTHER_APPS__ ?? others?.apps);
  }
  return invoke<ThisMacInfo>("this_mac");
}

/** The preview's advice: the machine fixture decides the verdict; "Deep
 * work" (p-1) has a learned footprint, the rest are not measured yet. */
function mockMachineAdvice(recipeIds: string[], machine: MachineHealthInfo, heaviest: { id: string; bytes: number } | null): MachineAdviceInfo[] {
  const level = machine.summary.level;
  const verdict = level === "pressure" ? "no" : level === "tight" ? "tight" : "ok";
  const words =
    verdict === "no" ? "Not now — this Mac is short of room" : verdict === "tight" ? "Tight — this Mac can take it, just" : "This Mac can take another session";
  const reasons = machine.summary.reasons.slice(0, 2).map((reason) => reason.words);
  return recipeIds.map((id, index) => ({
    recipe_id: id,
    verdict,
    words,
    reasons: verdict === "ok" ? [] : reasons,
    projected_bytes: index === 0 ? 2_100_000_000 : 1_000_000_000,
    based_on: index === 0 ? 6 : 0,
    // As the backend: closing a session is offered only when it frees 500 MB.
    relief_session_id: verdict !== "ok" && heaviest && heaviest.bytes >= 500_000_000 ? heaviest.id : null,
    relief_bytes: verdict !== "ok" && heaviest && heaviest.bytes >= 500_000_000 ? heaviest.bytes : null,
  }));
}

/** "Can this Mac take another session?" for every launch setting. */
export async function machineAdvice(): Promise<MachineAdviceInfo[]> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_ADVICE__?: MachineAdviceInfo[] }).__AIJ_ADVICE__;
    if (seeded) return structuredClone(seeded);
    const { launchRecipesList } = await import("./launch");
    const [recipes, mac] = await Promise.all([launchRecipesList(), thisMac()]);
    const top = mac.top.find((tree) => tree.session_id !== null);
    return mockMachineAdvice(
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
    const fresh = seams.__AIJ_ALERTS_FRESH__ ?? ((seams as { __AIJ_ALERT_TOLD__?: boolean }).__AIJ_ALERT_TOLD__ ? [] : previewAlerts(null));
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
