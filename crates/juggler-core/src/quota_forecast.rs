//! Burn-rate forecast (AJ-E608, deep/09 §Q4): EWMA burn-rates over
//! 5/15/60-minute windows, time-to-exhaust gated by the window's
//! remaining life, the launch/handover admission verdict
//! (OK/Tight/No), and the silent calibration-note maths — all PURE
//! over `usage_events` with an injected clock, so fixture timelines
//! need no harness (the E606 discipline).
//!
//! Honesty rules that are FEATURES here, not missing polish: with
//! <10 min of data the forecast is `Learning` and carries NO number
//! (never a fake zero-rate); a TTE beyond the window's reset is
//! suppressed as `WontExhaust`; Estimated forecasts are RANGES (the
//! 5↔60-minute cone), never a bare point; confidence is the LOWEST of
//! the inputs (deep/09 §Q2 Q-2). `ZCode` forecasts run on credits,
//! Claude/Codex on tokens — the cone maths is shared, the units are
//! not interchangeable.

use crate::model::Confidence;
use crate::quota::{QuotaCap, UsageEvent, WindowKind, window_bounds};

/// The three EWMA widths (minutes) — headline is 15, the cone is the
/// 5↔60 spread.
pub const RATE_WINDOWS_MIN: [u64; 3] = [5, 15, 60];
/// Cold start: less data than this ⇒ `Learning`, no number (deep/09
/// line 407).
pub const COLD_START_MIN_MS: i64 = 10 * 60 * 1000;
/// The launch gate's Tight threshold: projected burn above this share
/// of headroom is `Tight`; above headroom itself is `No`.
pub const TIGHT_HEADROOM_SHARE: f64 = 0.5;

/// Trust ranking for confidence inheritance (§Q2 Q-2: the forecast
/// inherits the LOWEST of its inputs). Higher = more trustworthy.
#[must_use]
pub fn confidence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Estimated => 0,
        Confidence::Computed => 1,
        Confidence::Measured => 2,
    }
}

/// Which unit a forecast runs in (per provider: `ZCode` = credits —
/// deep/09 §Q4 handover note).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RateUnits {
    /// Tokens per minute (Claude/Codex).
    Tokens,
    /// Vendor credits per minute (`ZCode`).
    Credits,
}

/// The headline burn-rate cone: the 15-minute EWMA plus the 5↔60
/// spread. `None` rate ⇒ Learning.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BurnRate {
    /// The 15-minute EWMA, units/minute.
    pub rate15: Option<f64>,
    /// The 5-minute EWMA (cone's fast edge).
    pub rate5: Option<f64>,
    /// The 60-minute EWMA (cone's slow edge).
    pub rate60: Option<f64>,
    /// The units the rates are in.
    pub units: RateUnits,
    /// Lowest confidence among the events folded.
    pub confidence: Confidence,
}

/// EWMA α for an N-period window (the standard `2/(N+1)`).
#[must_use]
pub fn ewma_alpha(window_min: u64) -> f64 {
    2.0 / (f64::from(u32::try_from(window_min).unwrap_or(60)) + 1.0)
}

/// One unit of burn per event, per the forecast's unit choice.
fn event_units(event: &UsageEvent, units: RateUnits) -> f64 {
    match units {
        RateUnits::Tokens => {
            #[allow(clippy::cast_precision_loss)]
            {
                (event.tokens.input
                    + event.tokens.output
                    + event.tokens.cache_read
                    + event.tokens.cache_write
                    + event.tokens.reasoning) as f64
            }
        }
        RateUnits::Credits => event.credits.unwrap_or(0.0),
    }
}

/// Computes the EWMA burn-rate cone over per-minute unit buckets.
/// Minutes with no events contribute zero burn (an honest gap, not a
/// skipped sample). With <[`COLD_START_MIN_MS`] of data span every
/// rate is `None` (Learning — never a fake number at any confidence).
#[must_use]
pub fn burn_rate(events: &[UsageEvent], account: &str, units: RateUnits, now_ms: i64) -> BurnRate {
    let confidence = events
        .iter()
        .filter(|event| event.account == account)
        .map(|event| event.confidence)
        .min_by_key(|confidence| confidence_rank(*confidence))
        .unwrap_or_default();
    let span_start = events
        .iter()
        .filter(|event| event.account == account)
        .map(|event| event.ts)
        .min();
    let cold = match span_start {
        Some(first) => now_ms.saturating_sub(first) < COLD_START_MIN_MS,
        None => true,
    };
    let mut rates = [None::<f64>, None::<f64>, None::<f64>];
    if !cold {
        for (index, window) in RATE_WINDOWS_MIN.iter().enumerate() {
            let width_ms = i64::try_from(window * 60_000).unwrap_or(i64::MAX);
            let minutes = (*window).max(1);
            let per_minute: Vec<f64> = (0..minutes)
                .map(|minute_back| {
                    let hi = now_ms - i64::try_from(minute_back * 60_000).unwrap_or(0);
                    let lo = hi - 60_000;
                    let sum: f64 = events
                        .iter()
                        .filter(|event| {
                            event.account == account
                                && event.ts > lo
                                && event.ts <= hi
                                && (width_ms == 0 || event.ts > (now_ms.saturating_sub(width_ms)))
                        })
                        .map(|event| event_units(event, units))
                        .sum::<f64>()
                        // IEEE signed-zero: an EMPTY reduction yields
                        // −0.0 on this toolchain (contraction), and
                        // −0.0 would later divide into −inf. Adding
                        // +0.0 normalises the sign ((−0)+(+0)=+0).
                        + 0.0;
                    sum
                })
                .collect();
            // Fold oldest→newest so recent minutes dominate.
            let alpha = ewma_alpha(*window);
            let mut ewma = per_minute[0];
            for sample in per_minute.iter().skip(1) {
                ewma = alpha * sample + (1.0 - alpha) * ewma;
            }
            // Rate per minute: the EWMA already operates on per-minute
            // burn, so the rate IS the EWMA value.
            rates[index] = Some(ewma);
        }
    }
    BurnRate {
        rate15: rates[1],
        rate5: rates[0],
        rate60: rates[2],
        units,
        confidence,
    }
}

/// The forecast's pill state (deep/10 §1 copy is AJ-E610's; these are
/// the typed states it renders).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ForecastState {
    /// <10 min of data — "learning — no forecast yet", NO number.
    Learning,
    /// The computed TTE meets the reset boundary first — "won't
    /// exhaust this window at current pace". No TTE is emitted beyond
    /// the reset, ever.
    WontExhaust,
    /// TTE inside the window. Point = rate15; the range is the cone
    /// (rate5/rate60 inverted — the faster rate exhausts sooner).
    Exhausts {
        /// Point estimate, minutes (rate15; rendered for Measured).
        tte_min: f64,
        /// Cone lower bound (minutes) — the SLOW edge (rate60).
        range_low_min: f64,
        /// Cone upper bound (minutes) — the FAST edge (rate5).
        range_high_min: f64,
    },
    /// Headroom is not computable (no user-confirmed cap): no TTE, no
    /// percentage — the honest absence (AJ-E606 §Q3's law).
    NoHeadroom,
}

/// The full forecast for one account's window.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Forecast {
    /// The pill state.
    pub state: ForecastState,
    /// Inherited confidence: the LOWEST of the inputs (events, cap,
    /// window reading — deep/09 §Q2 Q-2).
    pub confidence: Confidence,
}

/// Headroom in the forecast's units: `cap − window burn`. `None` when
/// no user-confirmed cap exists (never an invented denominator).
#[must_use]
pub fn headroom(
    events: &[UsageEvent],
    account: &str,
    kind: WindowKind,
    cap: QuotaCap,
    units: RateUnits,
    now_ms: i64,
) -> Option<f64> {
    let (lo, hi) = window_bounds(kind, now_ms);
    let burned: f64 = events
        .iter()
        .filter(|event| event.account == account && event.ts > lo && event.ts <= hi)
        .map(|event| event_units(event, units))
        .sum::<f64>()
        + 0.0; // signed-zero normalisation (see burn_rate)
    let cap_value = match units {
        RateUnits::Tokens => {
            #[allow(clippy::cast_precision_loss)]
            {
                cap.tokens.map(|tokens| tokens as f64)
            }
        }
        RateUnits::Credits => cap.credits,
    }?;
    Some((cap_value - burned).max(0.0))
}

/// The time-to-exhaust forecast. Pure; `resets_at` gates the emission
/// (a TTE beyond the reset is `WontExhaust`; an unknown reset leaves
/// the TTE unbounded ⇒ `NoHeadroom`-adjacent honesty: the state is
/// `Exhausts` only when the reset proves it fits).
#[must_use]
pub fn forecast(
    events: &[UsageEvent],
    account: &str,
    kind: WindowKind,
    cap: QuotaCap,
    units: RateUnits,
    resets_at: Option<i64>,
    now_ms: i64,
) -> Forecast {
    let rate = burn_rate(events, account, units, now_ms);
    let confidence = rate.confidence;
    let Some(rate15) = rate.rate15 else {
        return Forecast {
            state: ForecastState::Learning,
            confidence,
        };
    };
    // A decayed-to-zero rate (stalled burn) yields an effectively
    // infinite TTE — which the reset bound below turns into
    // `WontExhaust`: the honest "won't exhaust at this pace", never a
    // fake number (the four deep/10 pill states have no "stalled").
    let Some(room) = headroom(events, account, kind, cap, units, now_ms) else {
        return Forecast {
            state: ForecastState::NoHeadroom,
            confidence,
        };
    };
    let tte = room / rate15;
    // Non-finite TTE: +∞ is the honest "won't exhaust at this pace";
    // NaN promises nothing. Either way no number is emitted.
    if !tte.is_finite() {
        return Forecast {
            state: ForecastState::WontExhaust,
            confidence,
        };
    }
    // The reset bound: an unknown reset cannot prove the TTE fits —
    // never emit an unbounded promise (deep/09 lines 408-410).
    let Some(reset) = resets_at else {
        return Forecast {
            state: ForecastState::WontExhaust,
            confidence,
        };
    };
    let remaining_min = f64::from(u32::try_from((reset - now_ms).max(0) / 60_000).unwrap_or(0));
    if tte >= remaining_min {
        return Forecast {
            state: ForecastState::WontExhaust,
            confidence,
        };
    }
    let cone = |rate: Option<f64>| -> f64 {
        rate.filter(|value| *value > f64::EPSILON)
            .map_or(tte, |value| room / value)
    };
    Forecast {
        state: ForecastState::Exhausts {
            tte_min: tte,
            // The FAST edge (rate5) exhausts sooner — the range's low
            // bound; the SLOW edge (rate60) is the high bound.
            range_low_min: cone(rate.rate5).min(tte),
            range_high_min: cone(rate.rate60).max(tte),
        },
        confidence,
    }
}

/// The admission-advisor verdict (SPEC §237 language) for a projected
/// session: `No` WARNS and never blocks — the verdict is data for the
/// UI, not a gate the engine enforces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AdmissionVerdict {
    /// Projected burn comfortably inside headroom.
    Ok,
    /// Projected burn above [`TIGHT_HEADROOM_SHARE`] of headroom.
    Tight,
    /// Projected burn exceeds headroom — warn, never block.
    No,
}

/// Launch/handover gate: the INTENDED session's projected burn (the
/// median historical session burn for this project) vs the account's
/// window headroom. `None` verdict when either input is unknown —
/// never a guessed median or an invented headroom.
#[must_use]
pub fn admission_verdict(
    median_session_burn: Option<u64>,
    headroom_tokens: Option<f64>,
) -> Option<AdmissionVerdict> {
    let median = median_session_burn?;
    let room = headroom_tokens?;
    #[allow(clippy::cast_precision_loss)]
    let projected = median as f64;
    Some(if projected > room {
        AdmissionVerdict::No
    } else if projected > room * TIGHT_HEADROOM_SHARE {
        AdmissionVerdict::Tight
    } else {
        AdmissionVerdict::Ok
    })
}

/// The median historical session burn (tokens) for one project's
/// sessions — the launch gate's projection input. Sessions without
/// events are ignored (a never-used session mediates nothing).
#[must_use]
pub fn median_session_burn(events: &[UsageEvent], project_sessions: &[&str]) -> Option<u64> {
    let mut totals: Vec<u64> = Vec::new();
    for session in project_sessions {
        let total: u64 = events
            .iter()
            .filter(|event| event.session.as_deref() == Some(*session))
            .map(|event| {
                event.tokens.input
                    + event.tokens.output
                    + event.tokens.cache_read
                    + event.tokens.cache_write
                    + event.tokens.reasoning
            })
            .sum();
        if total > 0 {
            totals.push(total);
        }
    }
    if totals.is_empty() {
        return None;
    }
    totals.sort_unstable();
    let middle = totals.len() / 2;
    Some(if totals.len() % 2 == 1 {
        totals[middle]
    } else {
        u64::midpoint(totals[middle - 1], totals[middle])
    })
}

/// One silent calibration note (deep/09 §Q5 line 435): recorded when
/// realised exhaustion deviates >2× from the forecast — a trust input
/// for AJ-E611, NEVER an interruption. The loop that pairs forecasts
/// with realised exhaust events ships with the alert FSM (AJ-E611);
/// this is the pure trigger + shape.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct CalibrationNote {
    /// The account the forecast ran for.
    pub account: String,
    /// Forecast TTE, minutes.
    pub forecast_tte_min: f64,
    /// Realised TTE, minutes.
    pub actual_tte_min: f64,
    /// `actual / forecast` (>2 or <0.5 trips the note).
    pub ratio: f64,
    /// When the note was recorded, epoch ms.
    pub at_ms: i64,
}

/// Builds the calibration note when the deviation exceeds 2× either
/// way; `None` when the forecast held (no note, no noise).
#[must_use]
pub fn calibration_note(
    account: &str,
    forecast_tte_min: f64,
    actual_tte_min: f64,
    at_ms: i64,
) -> Option<CalibrationNote> {
    if forecast_tte_min <= f64::EPSILON {
        return None;
    }
    let ratio = actual_tte_min / forecast_tte_min;
    if ratio > 2.0 || ratio < 0.5 {
        Some(CalibrationNote {
            account: account.to_owned(),
            forecast_tte_min,
            actual_tte_min,
            ratio,
            at_ms,
        })
    } else {
        None
    }
}
