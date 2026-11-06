/**
 * Usage statistics and the funnel (AJ-E309): local or shared, the
 * report, UI events and gauges.
 */
import { call, liveBackend as hasBackend } from "./transport";
import type { UsageStatsInfo as UsageStatsInfoData } from "../../types/generated/UsageStatsInfo";
import type { UsageStatsOverview as UsageStatsOverviewData } from "../../types/generated/UsageStatsOverview";
import type { UsageStatsSetResult as UsageStatsSetResultData } from "../../types/generated/UsageStatsSetResult";

/** AJ-E309 Req.6: appends one funnel event to the LOCAL store (the
 * backend enforces the allow-list schema — counts, vocabulary words,
 * lowercase codes; never paths or labels). Preview mode records it on
 * `window.__AIJ_FUNNEL_EVENTS__` for specs. */
export async function funnelEventEmit(
  kind: string,
  fields: Record<string, string>,
): Promise<void> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_FUNNEL_EVENTS__?: { kind: string; fields: Record<string, string> }[];
    };
    seams.__AIJ_FUNNEL_EVENTS__ = [...(seams.__AIJ_FUNNEL_EVENTS__ ?? []), { kind, fields }];
    return;
  }
  return call("funnel_event", { kind, fields });
}

/** The usage-stats modes. */
export type UsageStatsMode = "off" | "local" | "share";

/** Preview mode's usage-stats state (tests and gallery captures set it). */
type StatsSeam = {
  __AIJ_USAGE_STATS__?: UsageStatsOverviewData;
  __AIJ_USAGE_STATS_REPORTS__?: Record<string, string>;
  __AIJ_USAGE_STATS_SETS__?: string[];
};

function previewStats(): UsageStatsOverviewData {
  const seams = window as unknown as StatsSeam;
  seams.__AIJ_USAGE_STATS__ ??= {
    info: {
      included: true,
      chosen: "off",
      effective: "off",
      can_upload: false,
      reason: null,
      decided_at_ms: null,
      consent_version: 0,
      current_consent_version: 1,
    },
    week: "2026-W42",
    rows: [],
    reports: [],
    next_upload_ms: null,
    last_error: null,
    docs_url: "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/usage-stats.md",
    sync: { offered: false, on: false, email: null, notice: null },
  };
  return seams.__AIJ_USAGE_STATS__;
}

/** AJ-E1102: the usage-stats choice and what applies. */
export async function usageStatsInfo(): Promise<UsageStatsInfoData> {
  if (!hasBackend) return previewStats().info;
  return call("usage_stats_info");
}

/** AJ-E1102: sets the mode (`off` deletes everything counted). */
export async function usageStatsSet(mode: UsageStatsMode): Promise<UsageStatsSetResultData> {
  if (!hasBackend) {
    const seams = window as unknown as StatsSeam;
    seams.__AIJ_USAGE_STATS_SETS__ = [...(seams.__AIJ_USAGE_STATS_SETS__ ?? []), mode];
    const current = previewStats();
    const info: UsageStatsInfoData = {
      ...current.info,
      chosen: mode,
      effective: current.info.reason ? current.info.effective : mode,
      can_upload: mode === "share" && !current.info.reason,
      decided_at_ms: Date.now(),
      consent_version: current.info.current_consent_version,
    };
    seams.__AIJ_USAGE_STATS__ = mode === "off" ? { ...current, info, rows: [], reports: [] } : { ...current, info };
    return {
      info,
      message:
        mode === "off"
          ? "Usage stats are off. Local stats deleted."
          : mode === "local"
            ? "Usage stats are counted on this computer only."
            : "Usage stats are on. One small report a week will be sent.",
    };
  }
  return call("usage_stats_set", { mode });
}

/** AJ-E1105: everything Settings ▸ Privacy shows. */
export async function usageStatsOverview(): Promise<UsageStatsOverviewData> {
  if (!hasBackend) return previewStats();
  return call("usage_stats_overview");
}

/** AJ-E1105: one report's exact JSON (read-only). */
export async function usageStatsReport(status: string, week: string): Promise<string> {
  if (!hasBackend) {
    const reports = (window as unknown as StatsSeam).__AIJ_USAGE_STATS_REPORTS__ ?? {};
    return reports[`${status}/${week}`] ?? "{}";
  }
  return call("usage_stats_report", { status, week });
}

/** AJ-E1105: "Delete local stats" (the mode is kept). */
export async function usageStatsDeleteLocal(): Promise<UsageStatsSetResultData> {
  if (!hasBackend) {
    const current = previewStats();
    (window as unknown as StatsSeam).__AIJ_USAGE_STATS__ = { ...current, rows: [], reports: [], last_error: null };
    return { info: current.info, message: "Local stats deleted." };
  }
  return call("usage_stats_delete_local");
}

/** AJ-E1109: turns the personal stats sync on or off. */
export async function usageStatsSetSync(on: boolean): Promise<UsageStatsOverviewData> {
  if (!hasBackend) {
    const current = previewStats();
    (window as unknown as StatsSeam).__AIJ_USAGE_STATS__ = {
      ...current,
      sync: { ...current.sync, on, notice: on ? null : current.sync.notice },
    };
    return previewStats();
  }
  return call("usage_stats_set_sync", { on });
}

/** AJ-E1105: opens the public list of every metric. */
export async function usageStatsOpenDocs(): Promise<void> {
  if (!hasBackend) return;
  return call("usage_stats_open_docs");
}

/** AJ-E1103: the usage-statistics metrics the interface counts. The
 * backend accepts only these, and only with the schema's values (a
 * `provider` may be any Provider id; it is mapped there). */
export type UiStatMetric =
  | "ui.screen_opened"
  | "ui.inspector_tab"
  | "ui.notification"
  | "ui.shortcut_used"
  | "cleanup_centre.opened"
  | "usage.suggestion"
  | "resource.alert"
  | "onboarding.login"
  | "onboarding.flow"
  | "update.flow"
  | "perf.board_interactive_ms";

/** AJ-E1103: counts one interface event — fire and forget, never awaited
 * on a user path, and a no-op unless the user opted in to usage stats. */
export function statsEvent(metric: UiStatMetric, dims: Record<string, string> = {}, value?: number): void {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_STATS_EVENTS__?: { metric: string; dims: Record<string, string>; value?: number }[];
    };
    seams.__AIJ_STATS_EVENTS__ = [...(seams.__AIJ_STATS_EVENTS__ ?? []), { metric, dims, value }];
    return;
  }
  void call("usage_stats_ui_event", { metric, dims, value: value ?? null }).catch(() => undefined);
}

/** AJ-E1103: a display preference as this week's gauge (fire and forget). */
export function statsGauge(metric: "ui.density" | "ui.theme", value: string): void {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_STATS_GAUGES__?: Record<string, string> };
    seams.__AIJ_STATS_GAUGES__ = { ...(seams.__AIJ_STATS_GAUGES__ ?? {}), [metric]: value };
    return;
  }
  void call("usage_stats_ui_gauge", { metric, value }).catch(() => undefined);
}

/** AJ-E309 Req.7: the LOCAL funnel report (markdown; never a network
 * call). Preview mode returns a short honest placeholder. */
export async function funnelExport(): Promise<string> {
  if (!hasBackend) {
    return "# Onboarding funnel — local export\n\n(preview mode — no events are stored)\n";
  }
  return call("funnel_export");
}
