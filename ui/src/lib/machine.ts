/**
 * E14 machine health (AJ-E1401): how the Mac is doing, for Home's This Mac
 * panel, the launch advisor and the overload alerts.
 *
 * In a plain browser (the design preview, Playwright) the mock serves a
 * deterministic fixture; `window.__AIJ_MACHINE__` picks the state
 * ("plenty" | "tight" | "pressure" | "unsampled", or a full payload).
 */
import { invoke } from "@tauri-apps/api/core";
import { hasLiveBackend } from "./bridge";
import type { MachineHealthInfo } from "../types/generated/MachineHealthInfo";
import type { MachinePointInfo } from "../types/generated/MachinePointInfo";
import type { RoomReasonInfo } from "../types/generated/RoomReasonInfo";

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
    // Activity Monitor's "Memory Used" (app + wired + compressed): high on
    // a working Mac, and not total − available (inactive pages count as
    // available).
    memory_used: 22 * GB,
    memory_available: 13 * GB,
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
