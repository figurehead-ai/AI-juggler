/**
 * AJ-E610, #120 — the quota display: each account's window clocks,
 * renewal, forecast and credits, as every quota surface reads them
 * (ENGINEERING-GUIDE U3: the feature's calls; the backend's
 * `features::quota`). Moved from bridge.ts (the size budget).
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";

/** AJ-E610: one window clock's typed rendered state. */
export interface WindowDisplay {
  /** `unmeasured`: use seen, no percentage (no sample, no confirmed cap). */
  /** `reset`: the window the last reading described has reset since;
   * nothing is known about the new one. */
  state: "unknown" | "learning" | "unmeasured" | "tracking" | "stale" | "exhausted" | "reset";
  percentage: number | null;
  badge: string | null;
  stale_note_ms: number | null;
  resets_at_ms: number | null;
  /** Where `resets_at_ms` came from; null with it (a reset time is
   * known or absent, never estimated). */
  reset_basis: "measured" | "calendar" | null;
  /** When the vendor's reading was taken: the "as of" beside a measured
   * percentage. */
  measured_at_ms: number | null;
  burned_tokens: number;
  burned_credits: number | null;
  headroom_tokens: number | null;
  /** AJ-E1512: sessions left and the burn-down (absent without a measured
   * reading of a window that hasn't reset). */
  runway?: RunwayDisplay | null;
  /** Where the reading behind it came from (`statusline`, `rollout`,
   * `desktop_app`, `usage_cache`, `api_retry`), when known. */
  source?: string | null;
  /** That source in words, from the tool's spec ("the Claude app"). */
  source_words?: string | null;
  /** In `reset` only: what the last reading said, as history. */
  last_used_pct?: number | null;
}

/** AJ-E1512: a window's runway — worked out from the vendor's readings of
 * it and the account's typical session (`juggler_core::quota_runway`). */
export interface RunwayDisplay {
  /** The window's readings so far, oldest first: `[epoch ms, percent used]`. */
  series: [number, number][];
  starts_ms: number;
  resets_at_ms: number;
  used_pct: number;
  /** The recent pace, percent of the window per hour. */
  pct_per_hour: number | null;
  /** When it runs out at that pace — only when before the reset. */
  exhausts_at_ms: number | null;
  /** A typical session's cost, percent of the window. */
  session_pct: number | null;
  sessions_left: number | null;
  sessions_counted: number;
  /** `measured|computed|estimated`: the lowest of what went in. */
  badge: string;
}

/** AJ-E610: the subscription clock's typed state (calendar line, D-12). */
export interface RenewalDisplay {
  state: "none" | "set" | "approaching" | "overdue";
  label: string;
  entered_by_user: boolean;
}

/** AJ-E610: the forecast pill data. */
export interface ForecastDisplay {
  state: "learning" | "wont-exhaust" | "exhausts" | "no-headroom";
  tte_min: number | null;
  range_min: number | null;
  range_max: number | null;
  badge: string;
}

/** AJ-E610: the two-clock display for one account. */
export interface QuotaDisplayData {
  account: string;
  five_hour: WindowDisplay | null;
  five_hour_omitted_reason: string | null;
  weekly: WindowDisplay | null;
  weekly_omitted_reason: string | null;
  renewal: RenewalDisplay;
  forecast: ForecastDisplay;
  unavailable_reason: string | null;
  /** Codex's credits, from its own rollouts. */
  credits: CreditsDisplay | null;
}

/** An account's credits, as its tool reports them. */
export interface CreditsDisplay {
  has_credits: boolean;
  unlimited: boolean;
  balance: string | null;
  as_of_ms: number;
  /** Who reported it: the tool's name ("Codex"). */
  source: string;
}

/** AJ-E306: one window's token headroom (cap − burned) — present
 * ONLY when the cap is known; the admission verdict's input. */
export interface WindowHeadroom {
  headroom_tokens?: number | null;
}

/** AJ-E610: one account's two clocks (engine-evaluated). */
export async function quotaDisplay(account: string): Promise<QuotaDisplayData> {
  if (!hasLiveBackend()) {
    const all = await quotaDisplayAll();
    return (
      all.find((entry) => entry.account === account) ?? {
        // What serde sends for an account with nothing measured: nulls.
        account,
        five_hour: null,
        five_hour_omitted_reason: null,
        weekly: null,
        weekly_omitted_reason: null,
        renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false },
        forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "estimated" },
        unavailable_reason: null,
        credits: null,
      }
    );
  }
  return invoke<QuotaDisplayData>("quota_display", { account });
}

/** AJ-E610: every account's two clocks (rail/row source; one query). */
export async function quotaDisplayAll(): Promise<QuotaDisplayData[]> {
  if (!hasLiveBackend()) {
    return (window as unknown as { __AIJ_QUOTA_DISPLAY__?: QuotaDisplayData[] })
      .__AIJ_QUOTA_DISPLAY__ ?? [];
  }
  return invoke<QuotaDisplayData[]>("quota_display_all");
}
