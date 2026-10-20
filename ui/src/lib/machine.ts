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
import { useEffect, useState } from "react";
import { hasLiveBackend, instancesList, type InstanceInfo } from "./bridge";
import type { MachineHealthInfo } from "../types/generated/MachineHealthInfo";
import type { MachinePointInfo } from "../types/generated/MachinePointInfo";
import type { RoomReasonInfo } from "../types/generated/RoomReasonInfo";
import type { ThisMacInfo } from "../types/generated/ThisMacInfo";
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

/** How the Mac is doing now. */
export async function machineHealth(): Promise<MachineHealthInfo> {
  if (!hasLiveBackend()) {
    const seeded = machineSeams().__AIJ_MACHINE__ ?? "plenty";
    return typeof seeded === "string" ? mockMachineHealth(seeded) : structuredClone(seeded);
  }
  return invoke<MachineHealthInfo>("machine_health");
}

/** Bytes in plain words ("3.2 GB", "640 MB") — the same rounding as the backend. */
export function bytesWords(bytes: number): string {
  if (bytes >= 1e9) {
    const gb = bytes / 1e9;
    return gb >= 10 ? `${gb.toFixed(0)} GB` : `${gb.toFixed(1)} GB`;
  }
  return `${(bytes / 1e6).toFixed(0)} MB`;
}

// --- AJ-E1403: This Mac ------------------------------------------------------


export type { ThisMacInfo, ToolTreeInfo, ToolTotalsInfo };

interface ThisMacSeams {
  /** A full payload (tests), or nothing for the derived mock. */
  __AIJ_THIS_MAC__?: ThisMacInfo;
  /** The AI tools AI Juggler did not start, for the derived mock
   * (default: one Claude app window). */
  __AIJ_OBSERVED__?: ToolTreeInfo[];
  /** Leftover processes/ports for the derived mock (default 0/0). */
  __AIJ_LEFTOVERS__?: [number, number];
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
    cpu_pct: measured ? Math.min(100, trees.reduce((sum, tree) => sum + (tree.cpu_pct ?? 0), 0) / cores) : null,
    rss_bytes: trees.reduce((sum, tree) => sum + tree.rss_bytes, 0),
  };
}

const MOCK_OBSERVED: ToolTreeInfo[] = [
  { session_id: null, place: "desktop", account_label: "Personal", cpu_pct: 14, rss_bytes: 1_600_000_000, helpers: 21, heavy_note: null },
];

/** The preview's This Mac, derived from the sessions on screen. */
export function mockThisMac(sessions: InstanceInfo[], machine: MachineHealthInfo, observed: ToolTreeInfo[] = MOCK_OBSERVED, leftovers: [number, number] = [0, 0]): ThisMacInfo {
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
  };
}

/** This Mac now. */
export async function thisMac(): Promise<ThisMacInfo> {
  if (!hasLiveBackend()) {
    const seams = thisMacSeams();
    if (seams.__AIJ_THIS_MAC__) return structuredClone(seams.__AIJ_THIS_MAC__);
    return mockThisMac(await instancesList(), await machineHealth(), seams.__AIJ_OBSERVED__, seams.__AIJ_LEFTOVERS__);
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
  }, [refreshKey]);
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
