/**
 * Wire-shaped builders: payloads exactly as serde sends them, for the
 * preview and for tests. Every Rust `Option` is `null` here, never a
 * missing key (the 0.0.12 report: fixtures written as partial objects hid
 * the nulls that crashed Usage). Start from these and override; the types
 * make a missing field an error.
 */
import type { ForecastDisplay, QuotaDisplayData, WindowDisplay } from "./bridge";

/** A window clock with nothing measured: every optional field null. */
export function windowDisplay(state: WindowDisplay["state"], over: Partial<WindowDisplay> = {}): WindowDisplay {
  return {
    state,
    percentage: null,
    badge: null,
    stale_note_ms: null,
    resets_at_ms: null,
    reset_basis: null,
    measured_at_ms: null,
    burned_tokens: 0,
    burned_credits: null,
    headroom_tokens: null,
    ...over,
  };
}

/** A forecast with no time to exhaustion. */
export function forecastDisplay(state: ForecastDisplay["state"], over: Partial<ForecastDisplay> = {}): ForecastDisplay {
  return { state, tte_min: null, range_min: null, range_max: null, badge: "estimated", ...over };
}

/** An account's two clocks with nothing measured. */
export function quotaDisplay(account: string, over: Partial<QuotaDisplayData> = {}): QuotaDisplayData {
  return {
    account,
    five_hour: null,
    five_hour_omitted_reason: null,
    weekly: null,
    weekly_omitted_reason: null,
    renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false },
    forecast: forecastDisplay("learning"),
    unavailable_reason: null,
    credits: null,
    ...over,
  };
}
