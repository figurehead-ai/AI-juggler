/**
 * AJ-E2903: how an Account's sessions went in the last 7 days (the
 * backend's `features::quality`). Without a backend the preview answers.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import { primaryToolId } from "../tools";
import type { AccountQualityInfo } from "../../types/generated/AccountQualityInfo";

export type { AccountQualityInfo };

export async function accountQuality(folders: string[]): Promise<AccountQualityInfo> {
  if (!hasLiveBackend()) {
    const fixture = (window as unknown as { __AIJ_ACCOUNT_QUALITY__?: AccountQualityInfo }).__AIJ_ACCOUNT_QUALITY__;
    if (fixture) return fixture;
    return { tool: primaryToolId() ?? "", sessions: 14, pending: 0, errors: 9, turns: 300, errors_measured: true, stuck_episodes: 2, context_avg_pct: 71, context_sessions: 11, finished: 9, tokens_per_finished: 412_000 };
  }
  return invoke<AccountQualityInfo>("quality_rollup", { folders });
}
