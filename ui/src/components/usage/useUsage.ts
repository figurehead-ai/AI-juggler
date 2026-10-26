/**
 * 0.1.5 — what the usage pages read: a range's summary for a scope (all,
 * one profile, one project), cut in the user's time zone, shown at once
 * from memory and refreshed; and a range choice remembered per page.
 */
import { useEffect, useMemo, useState } from "react";
import { useCached } from "../../lib/usage/cached";
import { cutRange, RANGES, type Cut, type RangeId } from "../../lib/usage/range";
import { usageHistory, usageHistoryFirst, type UsageHistoryInfo, type UsageShareInfo } from "../../lib/ipc/usageHistory";
import { toolSpec } from "../../lib/tools";
import { modelName, seriesColor, type GroupBy } from "../charts/colors";
import type { Series } from "../charts/scale";

export type Scope = { profile?: string | null; project?: string | null };

/** The range a page shows, remembered for that page. */
export function useRange(page: string, initial: RangeId = "30d"): [RangeId, (next: RangeId) => void] {
  const key = `aij.usage.range.${page}`;
  const [range, setRange] = useState<RangeId>(() => {
    try {
      const saved = localStorage.getItem(key);
      return RANGES.some((entry) => entry.id === saved) ? (saved as RangeId) : initial;
    } catch {
      return initial;
    }
  });
  const choose = (next: RangeId) => {
    setRange(next);
    try {
      localStorage.setItem(key, next);
    } catch {
      // This visit only.
    }
  };
  return [range, choose];
}

/** The clock, a minute at a time (the ranges move with it). */
export function useMinute(): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(timer);
  }, []);
  return now;
}

export type UsageView = {
  cut: Cut;
  data: UsageHistoryInfo | null;
  /** `data` is for this range and scope (else the last one's, shown dimmed). */
  current: boolean;
  loading: boolean;
  error: string | null;
};

/** A range's summary for a scope, split by `groupBy`. */
export function useUsageHistory(range: RangeId, groupBy: GroupBy, scope: Scope, top = 10): UsageView {
  const now = useMinute();
  const profile = scope.profile ?? null;
  const project = scope.project ?? null;
  const first = useCached<number | null>(range === "all" ? `first:${profile ?? "*"}` : null, () => usageHistoryFirst(profile), 300_000);
  const firstMs = first.data ?? null;
  // The hour pins the cut: a minute's tick doesn't read the range again.
  const hour = Math.floor(now / 3_600_000);
  const cut = useMemo(() => cutRange(range, now, firstMs), [range, hour, firstMs]);
  const ready = range !== "all" || !first.loading;
  const key = ready ? JSON.stringify({ e: [cut.edges[0], cut.edges[cut.edges.length - 1], cut.edges.length], groupBy, profile, project, top }) : null;
  const result = useCached<UsageHistoryInfo>(key, () => usageHistory({ edges_ms: cut.edges, group_by: groupBy, profile, project, top }));
  return { cut, data: result.data, current: result.current, loading: result.loading || !ready, error: result.error };
}

/** A group's name on screen: a tool's and a model's as people say them. */
export function groupLabel(groupBy: GroupBy, key: string, label: string): string {
  if (key === "__other") return "Other";
  if (groupBy === "tool" && key) return toolSpec(key).display_name;
  if (groupBy === "model") return modelName(key);
  return label;
}

/** The summary's series as the chart draws them: tokens or dollars. */
export function chartSeries(data: UsageHistoryInfo, groupBy: GroupBy, metric: "tokens" | "usd"): Series[] {
  return data.series.map((entry) => ({
    key: entry.key,
    label: groupLabel(groupBy, entry.key, entry.label),
    values: metric === "usd" ? entry.usd : entry.io,
    color: seriesColor(groupBy, entry.key),
  }));
}

/** Shares as a donut's parts. */
export function shareItems(shares: UsageShareInfo[], groupBy: GroupBy, metric: "tokens" | "usd" = "tokens", max = 7) {
  const items = shares.map((share) => ({
    key: share.key,
    label: groupLabel(groupBy, share.key, share.label),
    value: metric === "usd" ? (share.usd ?? 0) : share.io,
    color: seriesColor(groupBy, share.key),
  }));
  if (items.length <= max) return items;
  const rest = items.slice(max - 1);
  return [...items.slice(0, max - 1), { key: "__other", label: `${rest.length} more`, value: rest.reduce((sum, item) => sum + item.value, 0), color: seriesColor(groupBy, "__other") }];
}
