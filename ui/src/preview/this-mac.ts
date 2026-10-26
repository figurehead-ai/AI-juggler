/**
 * The preview's This Mac (no backend): derived from the sessions on
 * screen, the AI tools AI Juggler didn't start and the seams tests set,
 * grouped by tool as the backend groups them (#120). Moved from
 * lib/machine.ts (the size budget).
 */
import { bytesWords } from "../lib/machine";
import { providerDisplayName } from "../lib/providers";
import { PREVIEW_OBSERVED, PREVIEW_OTHER_APPS } from "./machine";
import type { OtherAppInfo } from "../types/generated/OtherAppInfo";
import type { InstanceInfo } from "../lib/bridge";
import type { MachineHealthInfo } from "../types/generated/MachineHealthInfo";
import type { ThisMacInfo } from "../types/generated/ThisMacInfo";
import type { OverloadAlertInfo } from "../types/generated/OverloadAlertInfo";
import type { ToolTreeInfo } from "../types/generated/ToolTreeInfo";
import type { ToolTotalsInfo } from "../types/generated/ToolTotalsInfo";
import type { ToolGroupInfo } from "../types/generated/ToolGroupInfo";

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

/** CPU summed as Home's rows count it (100% is one core); null while any
 * tree is unmeasured. */
function oneCore(trees: ToolTreeInfo[]): number | null {
  return trees.every((tree) => tree.cpu_pct !== null) ? trees.reduce((sum, tree) => sum + (tree.cpu_pct ?? 0), 0) : null;
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

/** Another app's group in words, as the backend: the app for a desktop
 * app ("ChatGPT"), else the tool in it ("Codex in Terminal"). The group
 * key is `app:<app>:<tool>`. */
function appGroupLabel(tree: ToolTreeInfo): string {
  const app = tree.group.split(":")[1] ?? "";
  return tree.place === "terminal" || tree.place === "editor" ? `${providerDisplayName(tree.tool ?? "")} in ${app}` : app;
}

/** The preview's This Mac, derived from the sessions on screen. */
export function mockThisMac(
  sessions: InstanceInfo[],
  machine: MachineHealthInfo,
  observed: ToolTreeInfo[] = PREVIEW_OBSERVED,
  leftovers: [number, number] = [0, 0],
  alerts: OverloadAlertInfo[] = [],
  recommendations: string[] = [],
  otherApps: OtherAppInfo[] = PREVIEW_OTHER_APPS,
): ThisMacInfo {
  const live = sessions.filter((session) => !["finished", "stopped", "handed_over", "failed"].includes(session.status));
  const ours: ToolTreeInfo[] = live
    .filter((session) => session.rss_bytes !== null)
    .map((session, at) => ({
      root_pid: 1000 + at,
      session_id: session.id,
      place: null,
      tool: session.provider,
      group: session.provider,
      account_label: session.account_label,
      cpu_pct: session.cpu_pct,
      rss_bytes: session.rss_bytes ?? 0,
      helpers: session.child_count ?? 0,
      heavy_note: mockHeavyNote(session.cpu_pct, session.rss_bytes ?? 0),
      memory_estimate: false,
    }));
  const all = [...ours, ...observed].sort((a, b) => b.rss_bytes - a.rss_bytes);
  // As the backend: our sessions by Provider (heaviest first), then the
  // Claude app, then claude in a terminal.
  const providers = new Map<string, ToolTreeInfo[]>();
  for (const tree of ours) {
    const provider = tree.group;
    providers.set(provider, [...(providers.get(provider) ?? []), tree]);
  }
  const byTool: ToolGroupInfo[] = [...providers.entries()]
    .map(([provider, trees]) => ({
      tool: provider,
      label: providerDisplayName(provider),
      ours: true,
      totals: totals(trees, machine.cores),
      cpu_one_core_pct: oneCore(trees),
    }))
    .sort((a, b) => b.totals.rss_bytes - a.totals.rss_bytes);
  for (const [place, label] of [
    ["desktop", "Claude app"],
    ["terminal", "claude in a terminal"],
  ] as const) {
    const trees = observed.filter((tree) => tree.place === place);
    if (trees.length > 0) byTool.push({ tool: place, label, ours: false, totals: totals(trees, machine.cores), cpu_one_core_pct: oneCore(trees) });
  }
  // As the backend (AJ-E2407): every other app's sessions, by app and tool,
  // heaviest first.
  const apps = new Map<string, ToolTreeInfo[]>();
  for (const tree of observed.filter((tree) => tree.group.startsWith("app:"))) apps.set(tree.group, [...(apps.get(tree.group) ?? []), tree]);
  byTool.push(
    ...[...apps.entries()]
      .map(([group, trees]) => ({ tool: group, label: appGroupLabel(trees[0]!), ours: false, totals: totals(trees, machine.cores), cpu_one_core_pct: oneCore(trees) }))
      .sort((a, b) => b.totals.rss_bytes - a.totals.rss_bytes),
  );
  // As the backend's align_with_alerts: while the memory-pressure alert is
  // on, the Mac's state says the same words.
  const aligned: MachineHealthInfo = alerts.some((alert) => alert.kind === "memory_pressure")
    ? { ...machine, summary: { ...machine.summary, level: "pressure", words: "Under memory pressure" } }
    : machine;
  return {
    machine: aligned,
    tools: totals(all, machine.cores),
    ours: totals(ours, machine.cores),
    theirs: totals(observed, machine.cores),
    by_tool: byTool,
    top: all.slice(0, 3),
    others: all.slice(3),
    heavy: all.filter((tree) => tree.heavy_note !== null),
    other_apps: otherApps,
    leftover_processes: leftovers[0],
    leftover_ports: leftovers[1],
    // As the backend's awake keeper, with Settings' defaults: kept awake
    // while sessions work, on power.
    awake: (() => {
      const working = live.filter((session) => session.status === "running" || session.status === "starting").length;
      const onBattery = machine.power?.on_battery ?? false;
      return { working, kept_awake: working > 0 && !onBattery, on_battery: onBattery };
    })(),
    // As a release build: its crash watchdog runs; it opens at login when
    // the preview's switch says so.
    own: {
      rss_bytes: 214_000_000,
      watchdog: true,
      opens_at_login: (window as unknown as { __AIJ_LOGIN_ITEM__?: string }).__AIJ_LOGIN_ITEM__ === "on",
    },
    alerts,
    recommendations,
  };
}

/** `?aij_load=heavy`: the first running session works the Mac hard. */
export function withHeavyLoad(sessions: InstanceInfo[]): InstanceInfo[] {
  const first = sessions.findIndex((session) => session.status === "running");
  return sessions.map((session, at) => (at === first ? { ...session, cpu_pct: 96, rss_bytes: 3_200_000_000, child_count: 5 } : session));
}

/** `?aij_windows=N`: N Claude app windows, heaviest first (the "and N
 * more" state); none asked for, the default. */
export function previewWindows(count: number): ToolTreeInfo[] | undefined {
  if (count <= 0) return undefined;
  return Array.from({ length: count }, (_, at) => ({
    ...PREVIEW_OBSERVED[0]!,
    root_pid: 600 + at,
    account_label: `Window ${at + 1}`,
    rss_bytes: Math.round(7_400_000_000 / (at + 1)),
  }));
}
