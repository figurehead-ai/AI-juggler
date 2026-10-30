/**
 * The preview's This Mac: a tool AI Juggler didn't start, observed (the
 * catalog's primary tool's app, in its adapter's words); the other apps;
 * and the 0.1.4 review's state, where other apps hold most of the memory.
 */
import type { MachineHealthInfo } from "../types/generated/MachineHealthInfo";
import type { MachinePointInfo } from "../types/generated/MachinePointInfo";
import type { OtherAppInfo } from "../types/generated/OtherAppInfo";
import type { OverloadAlertInfo } from "../types/generated/OverloadAlertInfo";
import type { ToolTreeInfo } from "../types/generated/ToolTreeInfo";
import { PREVIEW_TOOLS } from "./tools";

export const PREVIEW_OBSERVED: ToolTreeInfo[] = [
  { root_pid: 501, session_id: null, place: "desktop", label: "Claude app", tool: PREVIEW_TOOLS[0]!.id, group: "claude-app", account_label: "Personal", cpu_pct: 14, rss_bytes: 1_600_000_000, helpers: 21, heavy_note: null, memory_estimate: true },
];

// Memory, as macOS counts it (`lib/compact`'s memorySize): a 32 GB Mac
// has 32 × 1024³ bytes.
const GB = 1024 ** 3;

/** The other apps on a working Mac (This Mac's folded list). */
export const PREVIEW_OTHER_APPS: OtherAppInfo[] = [
  { name: "Safari", rss_bytes: 1_400_000_000, processes: 6 },
  { name: "Mail", rss_bytes: 420_000_000, processes: 1 },
  { name: "Music", rss_bytes: 180_000_000, processes: 1 },
];

/** The 0.1.4 review's Mac (`?aij_machine=others`): other apps hold most
 * of the memory, macOS reports pressure, and the Claude app's six windows
 * are about 16 GB of it. */
const OTHERS_APPS: OtherAppInfo[] = [
  { name: "next-server", rss_bytes: 6_400_000_000, processes: 1 },
  { name: "Docker", rss_bytes: 4_000_000_000, processes: 3 },
  { name: "fseventsd", rss_bytes: 1_500_000_000, processes: 1 },
  { name: "ffmpeg", rss_bytes: 1_200_000_000, processes: 2 },
  { name: "Google Chrome", rss_bytes: 900_000_000, processes: 8 },
  { name: "clang", rss_bytes: 700_000_000, processes: 4 },
];

/** The machine in that state, from the preview's calm one. */
export function othersFillIt(base: MachineHealthInfo): MachineHealthInfo {
  return {
    ...base,
    cores: 30,
    cpu_pct: 93,
    memory_total: 96 * GB,
    // As on that Mac: 47 GB compressed and swap nearly full, yet about
    // 12 GB still reads as available — the state the explanation is for.
    memory_used: 84 * GB,
    memory_compressed: 47 * GB,
    memory_available: 12 * GB,
    pressure: "warning",
    swap_total: 21.5 * GB,
    swap_used: 21.1 * GB,
    swap_growth_mb_per_min: 3004,
    summary: {
      level: "pressure",
      words: "Under pressure",
      reasons: [
        { kind: "swap", level: "pressure", words: "swap is growing — 3004 MB a minute" },
        { kind: "pressure", level: "tight", words: "macOS reports memory pressure" },
      ],
    },
    series: base.series.map((point) => ({ ...point, cpu_pct: point.cpu_pct === null ? null : 90 + (point.at_ms % 7), memory_used: 91 * GB })),
  };
}

/** What the preview shows in that state: the other apps, the alerts (as
 * the backend words them: other apps named, the Claude app not), and six
 * Claude app windows. `null` in any other state. */
export function othersPreview(fixture: unknown): { apps: OtherAppInfo[]; alerts: OverloadAlertInfo[]; observed: ToolTreeInfo[] } | null {
  if (fixture !== "others") return null;
  const named = OTHERS_APPS.slice(0, 3);
  const alert = (key: string, title: string, detail: string): OverloadAlertInfo => ({
    key,
    kind: key,
    title,
    detail,
    session_id: null,
    place: null,
    subject_name: null,
    subject_bytes: null,
    subject_estimate: false,
    subject_cpu: null,
    subject_windows: null,
    other_apps: named,
    since_ms: Date.now() - 90_000,
  });
  return {
    apps: OTHERS_APPS,
    alerts: [
      alert("memory_pressure", "Your Mac is under memory pressure", "macOS reports memory pressure"),
      alert("swap", "Your Mac is swapping hard", "swap is growing — 3004 MB a minute"),
    ],
    observed: [4_100_000_000, 3_600_000_000, 2_900_000_000, 2_300_000_000, 1_800_000_000, 1_300_000_000].map((rss, at) => ({
      ...PREVIEW_OBSERVED[0]!,
      root_pid: 700 + at,
      account_label: `Profile ${at + 1}`,
      cpu_pct: 3,
      rss_bytes: rss,
    })),
  };
}

/** `others`: the 0.1.4 review's Mac, other apps holding most of the memory. */
export type MachineFixture = "plenty" | "tight" | "pressure" | "others" | "disk" | "unsampled";

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
    memory_compressed: 2.4 * GB,
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
    case "others":
      return othersFillIt(base);
    case "disk":
      // Only the disk is short (3.1 GB free): what the Local cache's
      // nearly-full warning sits under, so the two agree.
      return {
        ...base,
        volumes: [{ name: "Macintosh HD", total_bytes: 994 * GB, available_bytes: 3.1 * GB }],
        summary: { level: "pressure", words: "Under pressure", reasons: [{ kind: "disk", level: "pressure", words: "Macintosh HD has only 3.1 GB free" }] },
      };
    case "tight":
      return {
        ...base,
        cpu_pct: 64,
        memory_used: 29.4 * GB,
        memory_compressed: 5.1 * GB,
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
        memory_compressed: 8.6 * GB,
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

const FIXTURES: readonly MachineFixture[] = ["plenty", "tight", "pressure", "others", "disk", "unsampled"];

/** A preview URL's fixture name, only when it is one we know. */
export function asFixture(value: string | undefined): MachineFixture | undefined {
  return FIXTURES.find((fixture) => fixture === value);
}
