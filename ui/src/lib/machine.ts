/**
 * E14 machine health (AJ-E1401): how the Mac is doing, for Home's This Mac
 * panel, the launch advisor and the overload alerts.
 *
 * In a plain browser (the design preview, Playwright) the mock serves a
 * deterministic fixture; `window.__AIJ_MACHINE__` picks the state
 * ("plenty" | "tight" | "pressure" | "unsampled", or a full payload).
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createContext, useContext, useEffect, useState } from "react";
import { hasLiveBackend, instancesList, type InstanceInfo } from "./bridge";
import type { MachineAdviceInfo } from "../types/generated/MachineAdviceInfo";
import type { MachineHealthInfo } from "../types/generated/MachineHealthInfo";
import type { MachinePointInfo } from "../types/generated/MachinePointInfo";
import type { RoomReasonInfo } from "../types/generated/RoomReasonInfo";
import type { ThisMacInfo } from "../types/generated/ThisMacInfo";
import type { OverloadAlertInfo } from "../types/generated/OverloadAlertInfo";
import type { ToolTreeInfo } from "../types/generated/ToolTreeInfo";
import type { ToolTotalsInfo } from "../types/generated/ToolTotalsInfo";

export type { MachineHealthInfo, MachinePointInfo, RoomReasonInfo };

export type MachineFixture = "plenty" | "tight" | "pressure" | "unsampled";

interface MachineSeams {
  __AIJ_MACHINE__?: MachineFixture | MachineHealthInfo;
}
const machineSeams = () => window as unknown as MachineSeams;

const GB = 1_000_000_000;

/** A deterministic 5-minute series (150 points, 2 s apart). */
function series(now: number, cpu: (i: number) => number, used: (i: number) => number): MachinePointInfo[] {
  return Array.from({ length: 150 }, (_, i) => ({
    at_ms: now - (149 - i) * 2_000,
    cpu_pct: i === 0 ? null : cpu(i),
    memory_used: used(i),
  }));
}

/** The mock payload for one fixture state. */
export function mockMachineHealth(fixture: MachineFixture, now = Date.now()): MachineHealthInfo {
  const base: MachineHealthInfo = {
    sampled: true,
    cores: 10,
    cpu_pct: 18,
    memory_total: 32 * GB,
    // macOS's app + wired + compressed (close to Activity Monitor's
    // "Memory Used"): high on a working Mac. Free is on the same basis, so
    // used + free = total.
    memory_used: 22 * GB,
    memory_basis: "app_wired_compressed",
    memory_available: 10 * GB,
    pressure: "normal",
    swap_total: 4 * GB,
    swap_used: 1.2 * GB,
    swap_growth_mb_per_min: 0,
    volumes: [{ name: "Macintosh HD", total_bytes: 994 * GB, available_bytes: 212 * GB }],
    power: { on_battery: false, minutes_left: null, battery_low: false, low_power_mode: false },
    thermal: "nominal",
    summary: { level: "plenty", words: "Plenty of room", reasons: [] },
    series: series(now, (i) => 14 + ((i * 7) % 9), (i) => 22 * GB + ((i * 13) % 5) * 40_000_000),
  };
  switch (fixture) {
    case "unsampled":
      return { ...base, sampled: false, cpu_pct: null, series: [], summary: { level: "plenty", words: "Plenty of room", reasons: [] } };
    case "plenty":
      return base;
    case "tight":
      return {
        ...base,
        cpu_pct: 64,
        memory_used: 29.4 * GB,
        memory_available: 2.6 * GB,
        pressure: "warning",
        swap_used: 3.1 * GB,
        swap_growth_mb_per_min: 140,
        summary: {
          level: "tight",
          words: "Getting tight",
          reasons: [
            { kind: "pressure", level: "tight", words: "macOS reports memory pressure" },
            { kind: "swap", level: "tight", words: "swap is growing — 140 MB a minute" },
          ],
        },
        series: series(now, (i) => 55 + ((i * 7) % 15), (i) => 27 * GB + i * 16_000_000),
      };
    case "pressure":
      return {
        ...base,
        cpu_pct: 96,
        memory_used: 31.2 * GB,
        memory_available: 0.8 * GB,
        pressure: "critical",
        swap_used: 3.9 * GB,
        swap_growth_mb_per_min: 620,
        thermal: "serious",
        volumes: [{ name: "Macintosh HD", total_bytes: 994 * GB, available_bytes: 14 * GB }],
        summary: {
          level: "pressure",
          words: "Under pressure",
          reasons: [
            { kind: "pressure", level: "pressure", words: "macOS reports critical memory pressure" },
            { kind: "swap", level: "pressure", words: "swap is growing — 620 MB a minute" },
            { kind: "memory", level: "pressure", words: "only 800 MB of memory free" },
            { kind: "cpu", level: "tight", words: "the processor has been 96% busy for over 60 s" },
            { kind: "thermal", level: "tight", words: "the Mac is warm and slowing down to cool off" },
            { kind: "disk", level: "tight", words: "Macintosh HD has only 14 GB free" },
          ],
        },
        series: series(now, (i) => 91 + ((i * 3) % 8), (i) => 30 * GB + i * 8_000_000),
      };
  }
}

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
      { key: "memory_pressure", kind: "memory_pressure", title: "Your Mac is under memory pressure", detail: "macOS reports memory pressure", session_id: sessionId, place: null, subject_bytes: 3_200_000_000, subject_estimate: false, subject_cpu: null, since_ms: Date.now() - 45_000 },
    ];
  if (kind === "cpu")
    return [
      { key: "cpu", kind: "cpu", title: "The processor has been flat out for minutes", detail: "97% busy for over two minutes", session_id: null, place: "desktop", subject_bytes: null, subject_estimate: false, subject_cpu: 140, since_ms: Date.now() - 150_000 },
    ];
  return [];
}

/** How the Mac is doing now. */
export async function machineHealth(): Promise<MachineHealthInfo> {
  if (!hasLiveBackend()) {
    const seeded = machineSeams().__AIJ_MACHINE__ ?? previewParam("aij_machine") ?? "plenty";
    return typeof seeded === "string" ? mockMachineHealth(seeded) : structuredClone(seeded);
  }
  return invoke<MachineHealthInfo>("machine_health");
}

/** Bytes in plain words ("3.2 GB", "640 MB", "under 1 MB") — the same rounding as the backend. */
export function bytesWords(bytes: number): string {
  if (bytes < 1e6) return "under 1 MB";
  if (bytes >= 1e9) {
    const gb = bytes / 1e9;
    return gb >= 10 ? `${gb.toFixed(0)} GB` : `${gb.toFixed(1)} GB`;
  }
  return `${(bytes / 1e6).toFixed(0)} MB`;
}

// --- AJ-E1403: This Mac ------------------------------------------------------


export type { ThisMacInfo, ToolTreeInfo, ToolTotalsInfo, OverloadAlertInfo };

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
}
const thisMacSeams = () => window as unknown as ThisMacSeams;

const HEAVY_CPU = 90;
const HEAVY_RSS = 2e9;

/** The rail's note for a tree, only when it's heavy (the backend rule,
 * on the current sample — the mock has no minute of history). */
export function mockHeavyNote(cpu: number | null, rss: number): string | null {
  const parts: string[] = [];
  if (cpu !== null && cpu > HEAVY_CPU) parts.push(`CPU ${Math.round(cpu)}%`);
  if (rss > HEAVY_RSS) parts.push(bytesWords(rss));
  return parts.length ? parts.join(" · ") : null;
}

function totals(trees: ToolTreeInfo[], cores: number): ToolTotalsInfo {
  const measured = trees.every((tree) => tree.cpu_pct !== null);
  return {
    count: trees.length,
    memory_estimate: trees.some((tree) => tree.memory_estimate),
    cpu_pct: measured ? Math.min(100, trees.reduce((sum, tree) => sum + (tree.cpu_pct ?? 0), 0) / cores) : null,
    rss_bytes: trees.reduce((sum, tree) => sum + tree.rss_bytes, 0),
  };
}

const MOCK_OBSERVED: ToolTreeInfo[] = [
  { session_id: null, place: "desktop", account_label: "Personal", cpu_pct: 14, rss_bytes: 1_600_000_000, helpers: 21, heavy_note: null, memory_estimate: true },
];

/** The preview's This Mac, derived from the sessions on screen. */
export function mockThisMac(
  sessions: InstanceInfo[],
  machine: MachineHealthInfo,
  observed: ToolTreeInfo[] = MOCK_OBSERVED,
  leftovers: [number, number] = [0, 0],
  alerts: OverloadAlertInfo[] = [],
  recommendations: string[] = [],
): ThisMacInfo {
  const live = sessions.filter((session) => !["finished", "stopped", "handed_over", "failed"].includes(session.status));
  const ours: ToolTreeInfo[] = live
    .filter((session) => session.rss_bytes !== null)
    .map((session) => ({
      session_id: session.id,
      place: null,
      account_label: session.account_label,
      cpu_pct: session.cpu_pct,
      rss_bytes: session.rss_bytes ?? 0,
      helpers: session.child_count ?? 0,
      heavy_note: mockHeavyNote(session.cpu_pct, session.rss_bytes ?? 0),
      memory_estimate: false,
    }));
  const all = [...ours, ...observed].sort((a, b) => b.rss_bytes - a.rss_bytes);
  return {
    machine,
    tools: totals(all, machine.cores),
    ours: totals(ours, machine.cores),
    theirs: totals(observed, machine.cores),
    top: all.slice(0, 3),
    heavy: all.filter((tree) => tree.heavy_note !== null),
    leftover_processes: leftovers[0],
    leftover_ports: leftovers[1],
    alerts,
    recommendations,
  };
}

/** This Mac now. */
export async function thisMac(): Promise<ThisMacInfo> {
  if (!hasLiveBackend()) {
    const seams = thisMacSeams();
    if (seams.__AIJ_THIS_MAC__) return structuredClone(seams.__AIJ_THIS_MAC__);
    let sessions = await instancesList();
    let leftovers = seams.__AIJ_LEFTOVERS__;
    if (previewParam("aij_load") === "heavy") {
      const first = sessions.findIndex((session) => session.status === "running");
      sessions = sessions.map((session, at) => (at === first ? { ...session, cpu_pct: 96, rss_bytes: 3_200_000_000, child_count: 5 } : session));
      leftovers = leftovers ?? [2, 1];
    }
    const top = sessions.find((session) => session.status === "running");
    const alerts = seams.__AIJ_ALERTS__ ?? previewAlerts(top?.id ?? null);
    const advice = seams.__AIJ_RECOMMENDATIONS__ ?? (previewParam("aij_history") ? PREVIEW_HISTORY : []);
    return mockThisMac(sessions, await machineHealth(), seams.__AIJ_OBSERVED__, leftovers, alerts, advice);
  }
  return invoke<ThisMacInfo>("this_mac");
}

/**
 * This Mac, kept current by the backend's 5 s `machine://this-mac`
 * emission (no UI timer). `refreshKey` re-reads it on demand — the
 * preview has no events, so callers pass something that changes with
 * the sessions on screen.
 */
export function useThisMac(refreshKey?: unknown): ThisMacInfo | null {
  const [info, setInfo] = useState<ThisMacInfo | null>(null);
  useEffect(() => {
    let alive = true;
    void thisMac().then((data) => alive && setInfo(data), () => null);
    return () => {
      alive = false;
    };
    // Live, the event keeps it current; the preview re-reads with the
    // sessions on screen.
  }, [hasLiveBackend() ? 0 : refreshKey]);
  useEffect(() => {
    if (!hasLiveBackend()) return;
    let unlisten: (() => void) | null = null;
    let alive = true;
    void listen<ThisMacInfo>("machine://this-mac", (event) => alive && setInfo(event.payload)).then((stop) => {
      if (alive) unlisten = stop;
      else stop();
    });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);
  return info;
}

/** "CPU 12% · 1.1 GB · 3 helpers" for a live session ("—" where unmeasured). */
export function resourceLine(cpu: number | null, rss: number | null, helpers: number | null): string {
  const parts = [
    `CPU ${cpu !== null ? `${Math.round(cpu)}%` : "—"}`,
    rss !== null ? bytesWords(rss) : "memory —",
    helpers === null ? "helpers —" : helpers === 1 ? "1 helper" : `${helpers} helpers`,
  ];
  return parts.join(" · ");
}

/** This Mac for components deep in the tree (the rail rows): the app
 * provides its one subscription here. */
export const ThisMacContext = createContext<ThisMacInfo | null>(null);

/** A session's heavy note ("CPU 95%", "3.2 GB"), only while it's heavy. */
export function useHeavyNote(sessionId: string): string | null {
  const info = useContext(ThisMacContext);
  return info?.heavy.find((tree) => tree.session_id === sessionId)?.heavy_note ?? null;
}

/** Reasons joined as one sentence ("macOS …" keeps its lower-case m). */
export function reasonSentence(words: string[]): string {
  const joined = words.join("; ");
  return joined.startsWith("macOS") ? joined : joined.replace(/^./, (first) => first.toUpperCase());
}

/** The rail's short form of a heavy note: one figure (memory first). */
export function shortHeavy(note: string): string {
  const parts = note.split(" · ");
  return parts[parts.length - 1] ?? note;
}

// --- AJ-E1407: the machine launch advisor ---------------------------------

export type { MachineAdviceInfo };

/** The preview's advice: the machine fixture decides the verdict; "Deep
 * work" (p-1) has a learned footprint, the rest are not measured yet. */
export function mockMachineAdvice(recipeIds: string[], machine: MachineHealthInfo, heaviest: { id: string; bytes: number } | null): MachineAdviceInfo[] {
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
    const { launchRecipesList } = await import("./bridge");
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

/** AJ-E1408: a told overload alert, until dismissed or its condition
 * ends. */
export interface MachineNotice {
  key: string;
  /** The engine's key (`memory_pressure`, `heavy:<session>`, …). */
  alertKey: string;
  kind: string;
  text: string;
  /** When it was told here, epoch ms. */
  toldAt: number;
}
/** A told notice stays at least this long before This Mac's (5 s) read can
 * end it — that read may predate the alert. */
const NOTICE_SETTLE_MS = 10_000;

/** Pure: the notices to keep — the newest one per kind (alerts queued while
 * the window was hidden land together), none whose condition has ended. */
export function keptNotices(notices: MachineNotice[], activeKeys: Set<string> | null, now: number): MachineNotice[] {
  const seen = new Set<string>();
  return notices.filter((notice) => {
    if (seen.has(notice.kind)) return false;
    seen.add(notice.kind);
    return activeKeys === null || activeKeys.has(notice.alertKey) || now - notice.toldAt < NOTICE_SETTLE_MS;
  });
}
/** Who an alert is about, and what of theirs is high ("feat/read-model is
 * using 3.2 GB", "The Claude app is using CPU 140%"): the engine sends only
 * the figures that are high, so a CPU-heavy session isn't told by its
 * memory. */
export function alertSubject(alert: OverloadAlertInfo, nameOf: (sessionId: string) => string | null): string | null {
  const who = alert.session_id
    ? (nameOf(alert.session_id) ?? "a session")
    : alert.place
      ? alert.place === "desktop"
        ? "The Claude app"
        : "claude in a terminal"
      : null;
  if (!who) return null;
  const figures = [
    alert.subject_cpu !== null ? `CPU ${Math.round(alert.subject_cpu)}%` : null,
    alert.subject_bytes !== null
      ? // A tool AI Juggler didn't start: its helpers share memory the
        // sum counts more than once (AJ-E1402).
        `${alert.subject_estimate ? "about " : ""}${bytesWords(alert.subject_bytes)}`
      : null,
  ].filter((figure): figure is string => figure !== null);
  if (figures.length === 0) return null;
  const what = figures.join(" and ");
  // A session's name keeps its own case ("feat/read-model").
  return `${who} is using ${what}`;
}
