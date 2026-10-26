/**
 * The preview's This Mac: a tool AI Juggler didn't start, observed (the
 * catalog's primary tool's app, in its adapter's words); the other apps;
 * and the 0.1.4 review's state, where other apps hold most of the memory.
 */
import type { MachineHealthInfo } from "../types/generated/MachineHealthInfo";
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
