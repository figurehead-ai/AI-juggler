//! Two-clock quota domain core (AJ-E606, deep/09 §Q3 + §Q8): the
//! WINDOW clock (rolling 5 h / weekly pools over an append-only event
//! store) and the SUBSCRIPTION clock (renewal/expiry per D-12) live in
//! deliberately separate types — no window computation reads or writes
//! subscription state, and nothing ever infers a renewal date from
//! window resets, first-seen dates, or plan tier.
//!
//! All maths here is PURE over `usage_events` slices with an injected
//! `now_ms` (deep/09 line 517-518): property tests need no harness,
//! DB, or wall time. Windows are EVENT SETS `(t−W, t]`, never buckets
//! — bucketing breaks rolling windows at boundaries. `Forecast` lives
//! in AJ-E608, ingest in AJ-E607; this module has neither.

use crate::model::{Confidence, TokenCounts};

/// Five-hour window length, milliseconds.
pub const FIVE_HOURS_MS: i64 = 5 * 60 * 60 * 1000;
/// Weekly window length, milliseconds.
pub const WEEK_MS: i64 = 7 * 24 * 60 * 60 * 1000;
/// A Measured reading older than this demotes `Tracking` to `Stale`
/// (§Q8: Stale applies to Measured freshness only — Computed/Estimated
/// burn never claims freshness in the first place).
pub const STALE_AFTER_MS: i64 = 10 * 60 * 1000;

/// Where a usage event came from (§Q8 `source` vocabulary).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum UsageSource {
    /// A vendor statusline feed sample.
    Statusline,
    /// A harness transcript's usage block.
    Transcript,
    /// A Codex-style rollout record.
    Rollout,
    /// A vendor SQLite store (`ZCode` `model_usage`).
    Sqlite,
    /// A retry-after signal observed on an API error (429 class).
    ApiRetry,
}

impl UsageSource {
    /// Stable storage name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            UsageSource::Statusline => "statusline",
            UsageSource::Transcript => "transcript",
            UsageSource::Rollout => "rollout",
            UsageSource::Sqlite => "sqlite",
            UsageSource::ApiRetry => "api_retry",
        }
    }

    /// Parses a stored source name; unknown values degrade to `None`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "statusline" => Some(UsageSource::Statusline),
            "transcript" => Some(UsageSource::Transcript),
            "rollout" => Some(UsageSource::Rollout),
            "sqlite" => Some(UsageSource::Sqlite),
            "api_retry" => Some(UsageSource::ApiRetry),
            _ => None,
        }
    }
}

/// One usage fact, append-only once stored (§Q3 event shape). Token
/// fields mirror [`TokenCounts`]; `credits` carries vendor credit
/// burns (`ZCode`), `usd_est` a local USD estimate (AJ-E609 owns real
/// pricing — here it is pass-through only).
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct UsageEvent {
    /// Owning account id (storage string form).
    pub account: String,
    /// Session the event is attributed to, when known.
    pub session: Option<String>,
    /// Event time, UTC epoch milliseconds.
    pub ts: i64,
    /// Model name, when the source names one.
    pub model: Option<String>,
    /// Token counters for the event.
    pub tokens: TokenCounts,
    /// Vendor credits burned, when the source reports them.
    pub credits: Option<f64>,
    /// Local USD estimate (pass-through; AJ-E609 computes real ones).
    pub usd_est: Option<f64>,
    /// Where the event came from.
    pub source: UsageSource,
    /// Trust level of the event's numbers.
    pub confidence: Confidence,
    /// Dedupe key (the vendor message uuid). Events without a uuid
    /// never dedupe — `UNIQUE(account, msg_uuid)` treats NULLs as
    /// distinct, which is the honest behaviour for resumed files.
    pub msg_uuid: Option<String>,
}

/// A vendor-observed quota sample for one window (§Q8 `quota_samples`).
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct QuotaSample {
    /// Sample time, UTC epoch milliseconds.
    pub ts: i64,
    /// Vendor-reported percent used, 0–100 (trusted verbatim when
    /// `confidence == Measured`).
    pub used_pct: f64,
    /// When the vendor says the window resets.
    pub resets_at: Option<i64>,
    /// Trust level of the sample.
    pub confidence: Confidence,
}

/// Which pool a reading belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum WindowKind {
    /// The rolling five-hour pool.
    FiveHour,
    /// The weekly pool. `anchor_ms` is the vendor purchase/period
    /// anchor when the vendor runs anchored phases (`ZCode` 7 d from
    /// purchase); `None` is a plain rolling 7 d window.
    Weekly {
        /// Phase anchor, UTC epoch milliseconds, if anchored.
        anchor_ms: Option<i64>,
    },
}

impl WindowKind {
    /// Stable storage name (`five_hour` | `weekly`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            WindowKind::FiveHour => "five_hour",
            WindowKind::Weekly { .. } => "weekly",
        }
    }

    /// Window length, milliseconds.
    #[must_use]
    pub const fn width_ms(self) -> i64 {
        match self {
            WindowKind::FiveHour => FIVE_HOURS_MS,
            WindowKind::Weekly { .. } => WEEK_MS,
        }
    }
}

/// The half-open-left, closed-right event interval `(t−W, t]` for a
/// rolling window (§Q3: the boundary rule both windows follow
/// identically). An event exactly at `t−W` is OUT; at `t` it is IN.
#[must_use]
pub fn window_bounds(kind: WindowKind, now_ms: i64) -> (i64, i64) {
    (now_ms.saturating_sub(kind.width_ms()), now_ms)
}

/// True when `ts` falls inside the window interval `(lo, hi]`.
#[must_use]
pub fn in_window(ts: i64, (lo, hi): (i64, i64)) -> bool {
    ts > lo && ts <= hi
}

/// An anchored weekly phase: the interval `(a+k·W, a+(k+1)·W]`
/// containing `now`, when an anchor exists (`ZCode` shape). Unanchored
/// windows roll — same as [`window_bounds`].
#[must_use]
pub fn weekly_phase_bounds(anchor_ms: Option<i64>, now_ms: i64) -> (i64, i64) {
    let kind = WindowKind::Weekly { anchor_ms };
    match anchor_ms {
        None => window_bounds(kind, now_ms),
        Some(anchor) => {
            let width = WEEK_MS;
            // Phase index of `now`, floored; anchors before the epoch
            // saturate to phase 0.
            let phase = now_ms
                .saturating_sub(anchor)
                .max(0)
                .checked_div(width)
                .unwrap_or(0);
            let start = anchor.saturating_add(phase.saturating_mul(width));
            (start, start.saturating_add(width))
        }
    }
}

/// Aggregated burn inside a window (the Computed/Estimated basis).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct WindowBurn {
    /// Token counters summed over in-window events.
    pub tokens: TokenCounts,
    /// Credits summed over in-window events (0 when no source reports
    /// credits).
    pub credits: f64,
    /// Event count (a request-granularity proxy).
    pub requests: u64,
}

/// A user-confirmed plan cap (§Q3: "user-confirmed plan cap"; with no
/// cap the engine exposes absolute burn and NO percentage — a
/// denominator is never invented).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct QuotaCap {
    /// Cap in tokens for the window, if confirmed.
    pub tokens: Option<u64>,
    /// Cap in vendor credits for the window, if confirmed.
    pub credits: Option<f64>,
}

impl QuotaCap {
    /// Settings-store encoding: `tokens=<n>` or `credits=<x>` (one
    /// unit per window; the user confirms what their plan bills).
    #[must_use]
    pub fn encode(self) -> Option<String> {
        match (self.tokens, self.credits) {
            (Some(tokens), _) => Some(format!("tokens={tokens}")),
            (None, Some(credits)) => Some(format!("credits={credits}")),
            (None, None) => None,
        }
    }

    /// Parses the settings-store encoding; malformed values return an
    /// empty cap (honest absence, never a guessed denominator).
    #[must_use]
    pub fn decode(value: &str) -> Self {
        if let Some(rest) = value.strip_prefix("tokens=") {
            return QuotaCap {
                tokens: rest.trim().parse::<u64>().ok(),
                credits: None,
            };
        }
        if let Some(rest) = value.strip_prefix("credits=") {
            return QuotaCap {
                tokens: None,
                credits: rest.trim().parse::<f64>().ok(),
            };
        }
        QuotaCap::default()
    }
}

/// The account's quota state in the §Q8 machine. `Unknown`/`Learning`
/// carry NO numeric percentage (never 0% — deep/09 line 516-517,
/// prior F2.4): the type enforces it, [`WindowReading`] holds the
/// `Option`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuotaState {
    /// No events, no samples: nothing is known.
    Unknown,
    /// First data arrived; not enough to trust a trend yet.
    Learning,
    /// A live basis exists (a Measured sample, or events to fold).
    Tracking,
    /// The Measured basis went quiet for >[`STALE_AFTER_MS`] — shown,
    /// not trusted as fresh.
    Stale,
    /// The vendor (or a Measured 100%) says the pool is spent.
    Exhausted,
}

/// What basis a percentage came from (decides the Measured vs
/// Computed/Estimated path, §Q3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PercentageBasis {
    /// Vendor `used_pct`, interpolated between samples with locally
    /// observed deltas (slope clamped, result clamped to `[0,100]`).
    Measured,
    /// `100 × burn / user-confirmed cap`.
    Computed,
}

/// One window's evaluated reading — the engine's output shape for a
/// single pool at a single instant.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WindowReading {
    /// Machine state.
    pub state: QuotaState,
    /// Percent used, ONLY in `Tracking`/`Stale`/`Exhausted` — `None`
    /// in `Unknown`/`Learning` (never 0%).
    pub percentage: Option<f64>,
    /// What the percentage is based on, when present.
    pub basis: Option<PercentageBasis>,
    /// Trust level of the reading as a whole.
    pub confidence: Confidence,
    /// Absolute burn inside the window (always present — the honest
    /// floor when no percentage exists).
    pub burn: WindowBurn,
    /// When the window resets, when estimable: Measured ⇒ vendor
    /// `resets_at` verbatim; rolling ⇒ earliest in-window event + W
    /// (an `≈`, AJ-E610 renders it so); anchored ⇒ phase end.
    pub resets_at: Option<i64>,
}

/// The subscription clock (D-12, composing AJ-E411's calendar core):
/// the account's renewal DATE with provenance and nothing else.
/// Deliberately boring — a date + who said so. No window computation
/// reads or writes it; no code path infers it. Dates are the
/// `YYYY-MM-DD` local-calendar form [`crate::renewal`] defines; the
/// account row's `renewal_date` column is the single storage source.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SubscriptionClock {
    /// The renewal date, `YYYY-MM-DD`, when one exists (AJ-E411:
    /// user-entered only — every provider's renewal search is
    /// negative as of 2026-10).
    pub renewal_date: Option<String>,
    /// True when the user typed the date (the D-12 sanctioned source).
    pub entered_by_user: bool,
    /// Whether a VERIFIED vendor feed supplied the date (none exists
    /// today; a future feed must be graded per AJ-E601 §6 before it
    /// may set this — the field exists so provenance is explicit).
    pub from_vendor_feed: bool,
}

/// The evaluated subscription state — `Unknown` unless a date exists
/// (D-12); a known date yields whole days until renewal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenewalState {
    /// No sanctioned source: no date on the account.
    Unknown,
    /// Days until the recorded renewal date (negative = overdue).
    RenewIn {
        /// Whole days from `today` to the renewal date.
        days: i64,
        /// Whether the user entered the date.
        entered_by_user: bool,
    },
}

impl SubscriptionClock {
    /// Evaluates the clock against an injected `today` (pure; reads
    /// nothing else — isolation is structural: no window function
    /// takes a clock, no clock function takes events). Malformed
    /// stored dates degrade to `Unknown`, never a guess.
    #[must_use]
    pub fn evaluate(&self, today: crate::renewal::CalendarDate) -> RenewalState {
        match self
            .renewal_date
            .as_deref()
            .and_then(crate::renewal::CalendarDate::parse)
        {
            None => RenewalState::Unknown,
            Some(date) => RenewalState::RenewIn {
                days: date.days_until(today),
                entered_by_user: self.entered_by_user,
            },
        }
    }
}

/// Sums the events inside one window for one account (§Q3: the fold).
/// Clock-skew tolerance: an event a few minutes "in the future"
/// simply lands inside `(t−W, t]` and sums — saturating arithmetic and
/// the ``[0,100]`` clamp downstream guarantee no negative headroom.
#[must_use]
pub fn window_burn<'a>(
    events: impl IntoIterator<Item = &'a UsageEvent>,
    account: &str,
    kind: WindowKind,
    now_ms: i64,
) -> WindowBurn {
    let (lo, hi) = match kind {
        WindowKind::FiveHour => window_bounds(kind, now_ms),
        WindowKind::Weekly { anchor_ms } => weekly_phase_bounds(anchor_ms, now_ms),
    };
    let mut burn = WindowBurn::default();
    for event in events {
        if event.account == account && in_window(event.ts, (lo, hi)) {
            let tokens = event.tokens;
            burn.tokens.input = burn.tokens.input.saturating_add(tokens.input);
            burn.tokens.output = burn.tokens.output.saturating_add(tokens.output);
            burn.tokens.cache_read = burn.tokens.cache_read.saturating_add(tokens.cache_read);
            burn.tokens.cache_write = burn.tokens.cache_write.saturating_add(tokens.cache_write);
            burn.tokens.reasoning = burn.tokens.reasoning.saturating_add(tokens.reasoning);
            burn.credits += event.credits.unwrap_or(0.0);
            burn.requests = burn.requests.saturating_add(1);
        }
    }
    burn
}

/// Requests an account logged in the half-open interval
/// `(from_ts, to_ts]` — the deltas the Measured interpolation scales.
#[must_use]
fn count_requests_between<'a>(
    events: impl IntoIterator<Item = &'a UsageEvent> + Clone,
    from_ts: i64,
    to_ts: i64,
) -> u64 {
    events
        .into_iter()
        .filter(|event| in_window(event.ts, (from_ts, to_ts)))
        .count() as u64
}

/// Computed percentage `100 × used / cap`, ONLY when a user-confirmed
/// cap exists in the burn's unit (§Q3); `None` otherwise. Clamped to
/// `[0, 100]`.
#[allow(clippy::cast_precision_loss)] // token counts exceed 2^52 never in practice
#[must_use]
pub fn computed_percentage(burn: &WindowBurn, cap: QuotaCap) -> Option<f64> {
    let (used, consumed) = if let Some(tokens) = cap.tokens {
        let total = burn.tokens.input
            + burn.tokens.output
            + burn.tokens.cache_read
            + burn.tokens.cache_write
            + burn.tokens.reasoning;
        (tokens as f64, total as f64)
    } else {
        cap.credits.map(|credit_cap| (credit_cap, burn.credits))?
    };
    if used <= 0.0 {
        return None;
    }
    Some((100.0 * consumed / used).clamp(0.0, 100.0))
}

/// The last observed %-per-request slope between two samples (§Q3
/// interpolation input): `(pct2 − pct1) / requests between their
/// timestamps`. `None` when no requests landed between the samples —
/// the vendor number then stands unchanged.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn slope_pct_per_request(
    first: &QuotaSample,
    second: &QuotaSample,
    requests_between: u64,
) -> Option<f64> {
    if requests_between == 0 {
        return None;
    }
    Some((second.used_pct - first.used_pct) / (requests_between as f64))
}

/// Measured interpolation (§Q3 verbatim): trust the vendor's last
/// `used_pct`; add locally observed request deltas scaled by the last
/// slope; clamp to `[0, 100]`. With no slope the vendor number stands.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn interpolate_percentage(
    last_sample: &QuotaSample,
    slope_per_request: Option<f64>,
    requests_since_sample: u64,
) -> f64 {
    let base = last_sample.used_pct;
    let delta = slope_per_request.map_or(0.0, |slope| slope * (requests_since_sample as f64));
    (base + delta).clamp(0.0, 100.0)
}

/// Estimated reset for a window (§Q3 "Reset"): Measured ⇒ the vendor
/// `resets_at` verbatim; otherwise rolling ⇒ earliest in-window event
/// + W (an `≈`); anchored ⇒ the phase end.
#[must_use]
pub fn estimated_reset(
    kind: WindowKind,
    events: &[UsageEvent],
    account: &str,
    now_ms: i64,
    measured_resets_at: Option<i64>,
) -> Option<i64> {
    if measured_resets_at.is_some() {
        return measured_resets_at;
    }
    let (lo, hi) = match kind {
        WindowKind::FiveHour => window_bounds(kind, now_ms),
        WindowKind::Weekly { anchor_ms } => weekly_phase_bounds(anchor_ms, now_ms),
    };
    if matches!(kind, WindowKind::Weekly { anchor_ms: Some(_) }) {
        return Some(hi);
    }
    let earliest = events
        .iter()
        .filter(|event| event.account == account && in_window(event.ts, (lo, hi)))
        .map(|event| event.ts)
        .min();
    earliest.map(|ts| ts.saturating_add(kind.width_ms()))
}

/// The per-account, per-window evaluation (§Q8 machine + §Q3 maths),
/// pure over the fetched event/sample tails with an injected clock.
///
/// State machine: `Unknown` (no data) → `Learning` (first data) →
/// `Tracking` (a Measured sample OR ≥2 events to fold) ⇄ `Stale`
/// (Measured basis quiet >[`STALE_AFTER_MS`]) → `Exhausted` (Measured
/// ≥100) → after the vendor reset passes with newer data → `Tracking`.
/// `Unknown`/`Learning` never carry a percentage.
#[must_use]
pub fn evaluate_window(
    events: &[UsageEvent],
    samples: &[QuotaSample],
    account: &str,
    kind: WindowKind,
    cap: QuotaCap,
    now_ms: i64,
) -> WindowReading {
    let account_events: Vec<&UsageEvent> = events
        .iter()
        .filter(|event| event.account == account)
        .collect();
    let burn = window_burn(account_events.iter().copied(), account, kind, now_ms);
    let latest_sample = samples
        .iter()
        .filter(|sample| sample.confidence == Confidence::Measured)
        .max_by_key(|sample| sample.ts);
    let last_activity = account_events
        .iter()
        .map(|event| event.ts)
        .chain(latest_sample.map(|sample| sample.ts))
        .max();

    // Exhausted: a Measured sample says the pool is spent and the
    // vendor reset has not passed yet.
    if let Some(sample) = latest_sample
        && sample.used_pct >= 100.0
        && sample.resets_at.is_none_or(|reset| reset > now_ms)
    {
        return WindowReading {
            state: QuotaState::Exhausted,
            percentage: Some(100.0),
            basis: Some(PercentageBasis::Measured),
            confidence: Confidence::Measured,
            burn,
            resets_at: sample.resets_at,
        };
    }

    let has_measured = latest_sample.is_some();
    let tracking = has_measured || account_events.len() >= 2;
    let state = if !has_measured && account_events.is_empty() {
        QuotaState::Unknown
    } else if !tracking {
        QuotaState::Learning
    } else if has_measured
        && let Some(last) = last_activity
        && now_ms.saturating_sub(last) > STALE_AFTER_MS
    {
        QuotaState::Stale
    } else {
        QuotaState::Tracking
    };

    let (percentage, basis, confidence, measured_resets_at) =
        if state == QuotaState::Unknown || state == QuotaState::Learning {
            (None, None, Confidence::default(), None)
        } else if let Some(sample) = latest_sample {
            // Measured path: vendor % verbatim, interpolated toward now
            // with locally observed request deltas scaled by the last
            // sample-to-sample slope.
            let previous = samples
                .iter()
                .filter(|s| s.confidence == Confidence::Measured && s.ts < sample.ts)
                .max_by_key(|s| s.ts);
            let slope = previous.and_then(|prev| {
                let between =
                    count_requests_between(account_events.iter().copied(), prev.ts, sample.ts);
                slope_pct_per_request(prev, sample, between)
            });
            let requests_since =
                count_requests_between(account_events.iter().copied(), sample.ts, i64::MAX);
            (
                Some(interpolate_percentage(sample, slope, requests_since)),
                Some(PercentageBasis::Measured),
                Confidence::Measured,
                sample.resets_at,
            )
        } else {
            // Computed path: fold + user-confirmed cap; no cap ⇒ absolute
            // burn only (percentage None — D-06 honest field).
            (
                computed_percentage(&burn, cap),
                Some(PercentageBasis::Computed),
                Confidence::Computed,
                None,
            )
        };
    // Reset estimates also in Learning: an anchored phase end is a
    // calendar fact, and a rolling earliest+W is the honest `≈`.
    // Unknown stays reset-less (nothing is known at all, D-06).
    let resets_at = if state == QuotaState::Tracking
        || state == QuotaState::Stale
        || state == QuotaState::Learning
    {
        estimated_reset(kind, events, account, now_ms, measured_resets_at)
    } else {
        measured_resets_at
    };
    WindowReading {
        state,
        percentage,
        basis,
        confidence,
        burn,
        resets_at,
    }
}

/// The whole account reading: both pools plus the separate
/// subscription clock (D-12) — the two clocks are SIBLINGS here,
/// never merged; AJ-E610 renders them with distinct treatments.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AccountQuota {
    /// The 5 h pool reading.
    pub five_hour: WindowReading,
    /// The weekly pool reading.
    pub weekly: WindowReading,
    /// The subscription clock evaluation (pure date arithmetic).
    pub renewal: RenewalState,
}

/// Everything [`evaluate_account`] needs, bundled (the two clocks'
/// inputs stay siblings here — never merged into one number).
#[derive(Clone, Copy)]
pub struct AccountEvaluation<'a> {
    /// The account's event tail (a 7 d fetch serves both pools).
    pub events: &'a [UsageEvent],
    /// The 5 h window's vendor samples (window-filtered at the store).
    pub samples_five_hour: &'a [QuotaSample],
    /// The weekly window's vendor samples.
    pub samples_weekly: &'a [QuotaSample],
    /// Account id (storage string form).
    pub account: &'a str,
    /// User-confirmed 5 h cap (a percentage exists only when this does).
    pub cap_five_hour: QuotaCap,
    /// User-confirmed weekly cap.
    pub cap_weekly: QuotaCap,
    /// Vendor weekly phase anchor when anchored (`ZCode`), else rolling.
    pub weekly_anchor_ms: Option<i64>,
    /// The separate subscription clock (D-12) — evaluated, never
    /// influenced by the windows.
    pub subscription: &'a SubscriptionClock,
    /// Injected wall clock, epoch milliseconds.
    pub now_ms: i64,
    /// Injected calendar day for the subscription clock.
    pub today: crate::renewal::CalendarDate,
}

/// Evaluates both windows and the subscription clock for one account.
#[must_use]
pub fn evaluate_account(input: AccountEvaluation<'_>) -> AccountQuota {
    AccountQuota {
        five_hour: evaluate_window(
            input.events,
            input.samples_five_hour,
            input.account,
            WindowKind::FiveHour,
            input.cap_five_hour,
            input.now_ms,
        ),
        weekly: evaluate_window(
            input.events,
            input.samples_weekly,
            input.account,
            WindowKind::Weekly {
                anchor_ms: input.weekly_anchor_ms,
            },
            input.cap_weekly,
            input.now_ms,
        ),
        renewal: input.subscription.evaluate(input.today),
    }
}
