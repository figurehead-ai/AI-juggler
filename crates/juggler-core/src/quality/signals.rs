//! The signal engine (D-07): tokens/min, error rate, context report,
//! stuck evidence — individually labelled, never a composite score.

use crate::model::TokenCounts;

/// Honesty tier adjacent to every value (pass2/04 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Observed directly from a vendor-provided feed.
    Measured,
    /// Deterministically computed from local data.
    Computed,
    /// Inferred; approximate by nature — always labelled.
    Estimated,
}

impl Tier {
    /// Wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Measured => "Measured",
            Tier::Computed => "Computed",
            Tier::Estimated => "Estimated",
        }
    }
}

/// One transcript usage event (input+output tokens at a timestamp).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    /// Epoch milliseconds of the turn.
    pub at_ms: i64,
    /// Tokens reported by the provider for this turn.
    pub tokens: TokenCounts,
    /// The model this turn actually ran (verbatim vendor string), when
    /// the transcript records one — the per-turn pricing key (AJ-E412
    /// Req.4); `None` turns are counted but never priced.
    pub model: Option<String>,
}

/// Tokens/min over the trailing 5 minutes: 1-minute bars (oldest →
/// newest) + the 5-minute average. Idle with events in-window renders a
/// REAL `0 tok/min`; no events at all is `no data` (different state).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokensPerMin {
    /// Five 1-minute bars, oldest first; `None` = no data at all.
    pub bars: Option<[u64; 5]>,
    /// Tokens in the full trailing 5 minutes.
    pub trailing_5_min: u64,
    /// Average tokens/min over the trailing 5 minutes (0 when idle).
    pub avg_per_min: f64,
}

impl TokensPerMin {
    /// `true` when at least one event exists in the trailing 5 min
    /// (idle-with-events is a real 0, distinct from no-data).
    #[must_use]
    pub fn has_window(&self) -> bool {
        self.bars.is_some()
    }
}

/// Computes tokens/min (input+output) from usage events in the trailing
/// 5 minutes. Single pass; callers keep the event list incremental.
// Token sums are u64; the /5 average crosses into f64 — precision loss
// past 2^53 tokens is irrelevant for a per-minute rate.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn tokens_per_min(events: &[UsageEvent], now_ms: i64) -> TokensPerMin {
    const WINDOW_MS: i64 = 5 * 60_000;
    let mut bars = [0_u64; 5];
    let mut any_in_window = false;
    for event in events {
        let age = now_ms - event.at_ms;
        if !(0..WINDOW_MS).contains(&age) {
            continue;
        }
        any_in_window = true;
        // Minute index 0 = oldest (4..0 minutes ago); age is
        // range-checked above so try_from never fails — unwrap_or(0)
        // only satisfies the compiler.
        let minute = usize::try_from(4 - age / 60_000).unwrap_or(0);
        bars[minute] += event.tokens.input + event.tokens.output;
    }
    if !any_in_window {
        // No events in-window at all: either the transcript never had
        // usage, or everything is older than 5 min. Callers distinguish
        // via events.is_empty() for `no data`.
        if events.is_empty() {
            return TokensPerMin {
                bars: None,
                trailing_5_min: 0,
                avg_per_min: 0.0,
            };
        }
        return TokensPerMin {
            bars: Some(bars),
            trailing_5_min: 0,
            avg_per_min: 0.0,
        };
    }
    let total = bars.iter().sum();
    TokensPerMin {
        bars: Some(bars),
        trailing_5_min: total,
        avg_per_min: total as f64 / 5.0,
    }
}

/// Error rate as numerator/denominator (`2 errors / 41 turns`) — never
/// a bare percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorRate {
    /// Failed turns + api_retry/tool-error events in the window.
    pub errors: u64,
    /// Total turns in the window.
    pub turns: u64,
    /// True when the provider has no structured error feed — the UI
    /// renders the provider-specific reason instead of numbers.
    pub not_measurable: bool,
}

impl ErrorRate {
    /// The exact rendered string (`2 errors / 41 turns`).
    #[must_use]
    pub fn label(&self) -> String {
        format!("{} errors / {} turns", self.errors, self.turns)
    }
}

/// Context-window report: only where the provider reports context
/// tokens. Estimating context from cumulative tokens is PROHIBITED.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextReport {
    /// Reported context tokens + window size.
    Reported {
        /// Context tokens in use.
        used: u64,
        /// The provider-reported window size.
        window: u64,
    },
    /// The provider does not report context — the exact reason renders.
    NotReported,
}

/// The full per-Instance signal set (no score field exists — by design).
#[derive(Debug, Clone, PartialEq)]
pub struct QualitySignals {
    /// tokens/min (Measured from transcript events).
    pub tokens_per_min: TokensPerMin,
    /// Error numerator/denominator or not-measurable.
    pub error_rate: ErrorRate,
    /// Context report (reported or the provider-specific reason).
    pub context: ContextReport,
    /// Cumulative session tokens — labelled `cumulative, not context`.
    pub cumulative: TokenCounts,
    /// The AJ-E405 stuck rule id + inputs when tripped (permanent `?`).
    pub stuck_evidence: Option<crate::model::AttentionReason>,
}

/// Quota-forecast label (Req.7): a RANGE at the current burn rate,
/// tier-tagged; idle renders `not burning` — never a point estimate.
/// `pct_burned_last_5_min` is the observed quota-% consumed in the
/// trailing 5 minutes (from quota-window deltas, not raw tokens).
// The ±33% range rounds to whole minutes — truncation past the round
// is irrelevant to a range this wide.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn quota_forecast_label(quota_used_pct: f64, pct_burned_last_5_min: f64) -> String {
    if pct_burned_last_5_min <= f64::EPSILON {
        return "not burning".to_string();
    }
    // Range = ±33% around the observed burn (uncertainty, not precision).
    let pct_per_min = (pct_burned_last_5_min / 5.0).max(0.001);
    let pct_remaining = (100.0 - quota_used_pct).max(0.0);
    let mid_minutes = pct_remaining / pct_per_min;
    let low = (mid_minutes * 0.67).round().max(0.0) as i64;
    let high = (mid_minutes * 1.33).round().max(0.0) as i64;
    if high == 0 {
        return "not burning".to_string();
    }
    format!("~{low}-{high}m at current rate · Est")
}
