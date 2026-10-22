/**
 * The preview's This Mac (no backend): derived from the sessions on
 * screen, the AI tools AI Juggler didn't start and the seams tests set,
 * grouped by tool as the backend groups them (#120). Moved from
 * lib/machine.ts (the size budget).
 */
import { bytesWords } from "../lib/machine";
import { providerDisplayName } from "../lib/providers";
import { PREVIEW_OBSERVED } from "./machine";
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

/** The preview's This Mac, derived from the sessions on screen. */
export function mockThisMac(
  sessions: InstanceInfo[],
  machine: MachineHealthInfo,
  observed: ToolTreeInfo[] = PREVIEW_OBSERVED,
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
      tool: session.provider,
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
    const provider = tree.tool ?? "";
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
    heavy: all.filter((tree) => tree.heavy_note !== null),
    leftover_processes: leftovers[0],
    leftover_ports: leftovers[1],
    alerts,
    recommendations,
  };
}
