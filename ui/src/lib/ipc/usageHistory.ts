/**
 * 0.1.5 — the usage history's calls (the backend's
 * `features::usage_history`): summaries per range, a page of sessions,
 * a profile's limit windows and one session's turns. Without a backend
 * the preview's synthetic data answers, loaded only then.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { UsageHistoryInfo } from "../../types/generated/UsageHistoryInfo";
import type { UsageLimitsInfo } from "../../types/generated/UsageLimitsInfo";
import type { UsageQueryInput } from "../../types/generated/UsageQueryInput";
import type { UsageSessionDetailInfo } from "../../types/generated/UsageSessionDetailInfo";
import type { UsageSessionInfo } from "../../types/generated/UsageSessionInfo";
import type { Page, PageRequest } from "../paged";

export type { UsageHistoryInfo, UsageLimitsInfo, UsageQueryInput, UsageSessionDetailInfo, UsageSessionInfo };
export type { UsageShareInfo } from "../../types/generated/UsageShareInfo";
export type { UsageSeriesInfo } from "../../types/generated/UsageSeriesInfo";
export type { UsageTotalsInfo } from "../../types/generated/UsageTotalsInfo";

const preview = () => import("../../preview/usage-history");

/** A range's usage, summarised for the charts. */
export async function usageHistory(input: UsageQueryInput): Promise<UsageHistoryInfo> {
  if (!hasLiveBackend()) return (await preview()).previewHistory(input);
  return invoke<UsageHistoryInfo>("usage_history", { input });
}

/** A page of the range's sessions, the biggest first (the shared paging). */
export async function usageHistorySessions(input: UsageQueryInput, page: PageRequest): Promise<Page<UsageSessionInfo>> {
  if (!hasLiveBackend()) return (await preview()).previewSessionsPage(input, page);
  return invoke<Page<UsageSessionInfo>>("usage_history_sessions", { input, page });
}

/** The first usage, epoch ms (one profile's when named): where "All" starts. */
export async function usageHistoryFirst(profile: string | null): Promise<number | null> {
  if (!hasLiveBackend()) return (await preview()).previewFirst(profile);
  return invoke<number | null>("usage_history_first", { profile });
}

/** A profile's 5-hour and weekly windows over a range. */
export async function usageLimits(profile: string, startMs: number, endMs: number): Promise<UsageLimitsInfo> {
  if (!hasLiveBackend()) return (await preview()).previewLimits(profile, startMs, endMs);
  return invoke<UsageLimitsInfo>("usage_limits", { profile, startMs, endMs });
}

/** One session's usage turn by turn, and its share of its profile's windows. */
export async function usageSession(profile: string, conversation: string): Promise<UsageSessionDetailInfo> {
  if (!hasLiveBackend()) return (await preview()).previewSession(profile, conversation);
  return invoke<UsageSessionDetailInfo>("usage_session", { profile, conversation });
}
