/**
 * 0.1.5 — the usage history's calls (the backend's
 * `features::usage_history`): summaries per range, a page of sessions,
 * a profile's limit windows and one session's turns. Without a backend
 * the preview's synthetic data answers, loaded only then.
 */
import { call } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { UsageHistoryInfo } from "../../types/generated/UsageHistoryInfo";
import type { UsageLimitsInfo } from "../../types/generated/UsageLimitsInfo";
import type { UsageQueryInput } from "../../types/generated/UsageQueryInput";
import type { UsageSessionDetailInfo } from "../../types/generated/UsageSessionDetailInfo";
import type { UsageSessionInfo } from "../../types/generated/UsageSessionInfo";
import type { UsageCostCutsInfo } from "../../types/generated/UsageCostCutsInfo";
import type { UsageWastedInfo } from "../../types/generated/UsageWastedInfo";
import type { Page, PageRequest } from "../paged";

export type { UsageCostCutsInfo, UsageHistoryInfo, UsageLimitsInfo, UsageQueryInput, UsageSessionDetailInfo, UsageSessionInfo, UsageWastedInfo };
export type { UsageAccountCostInfo } from "../../types/generated/UsageAccountCostInfo";
export type { UsageCutInfo } from "../../types/generated/UsageCutInfo";

export type { UsageShareInfo } from "../../types/generated/UsageShareInfo";
export type { UsageSeriesInfo } from "../../types/generated/UsageSeriesInfo";
export type { UsageTotalsInfo } from "../../types/generated/UsageTotalsInfo";

const preview = () => import("../../preview/usage-history");

/** A range's usage, summarised for the charts. */
export async function usageHistory(input: UsageQueryInput): Promise<UsageHistoryInfo> {
  if (!hasLiveBackend()) return (await preview()).previewHistory(input);
  return call("usage_history_summary", { input });
}

/** A page of the range's sessions, the biggest first (the shared paging). */
export async function usageHistorySessions(input: UsageQueryInput, page: PageRequest): Promise<Page<UsageSessionInfo>> {
  if (!hasLiveBackend()) return (await preview()).previewSessionsPage(input, page);
  return call("usage_history_sessions", { input, page });
}

/** AJ-E2902: the range's cost cuts (finished, wasted, by account, prices). */
export async function usageHistoryCuts(input: UsageQueryInput): Promise<UsageCostCutsInfo> {
  if (!hasLiveBackend()) return (await import("../../preview/usage-cuts")).previewCuts(input);
  return call("usage_history_cuts", { input });
}

/** AJ-E2902: a page of the range's sessions that crashed or were probably stuck. */
export async function usageHistoryWasted(input: UsageQueryInput, page: PageRequest): Promise<Page<UsageWastedInfo>> {
  if (!hasLiveBackend()) return (await import("../../preview/usage-cuts")).previewWasted(input, page);
  return call("usage_history_wasted", { input, page });
}

/** The first usage, epoch ms (one profile's when named): where "All" starts. */
export async function usageHistoryFirst(profile: string | null): Promise<number | null> {
  if (!hasLiveBackend()) return (await preview()).previewFirst(profile);
  return call("usage_history_first", { profile });
}

/** A profile's 5-hour and weekly windows over a range. */
export async function usageLimits(profile: string, startMs: number, endMs: number): Promise<UsageLimitsInfo> {
  if (!hasLiveBackend()) return (await preview()).previewLimits(profile, startMs, endMs);
  return call("usage_history_limits", { profile, startMs, endMs });
}

/** One session's usage turn by turn, and its share of its profile's windows. */
export async function usageSession(profile: string, conversation: string): Promise<UsageSessionDetailInfo> {
  if (!hasLiveBackend()) return (await preview()).previewSession(profile, conversation);
  return call("usage_history_session", { profile, conversation });
}
