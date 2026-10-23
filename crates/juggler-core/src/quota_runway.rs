//! AJ-E1512: a window's runway in the units people plan with. From the
//! vendor's own readings of the current window (Measured `quota_samples`)
//! and the tokens AI Juggler saw between them, it works out what a
//! typical session of this account costs in percent of the window, how
//! many such sessions are left, and the burn-down: the readings so far,
//! the recent pace, when it runs out at that pace (if before the reset),
//! and the reset. Never a guess dressed as a reading: with too little to
//! go on, the part that can't be worked out stays `None`.

use crate::model::{Confidence, TokenCounts};
use crate::quota::{QuotaSample, UsageEvent};
use crate::quota_forecast::{confidence_rank, median_session_burn};

/// Readings this close to the window's reset belong to the same window.
const SAME_RESET_MS: i64 = 60_000;
/// The pace is measured over at most this much of the recent past…
const PACE_SPAN_MS: i64 = 60 * 60_000;
/// …and needs at least this much of it.
const PACE_MIN_MS: i64 = 10 * 60_000;
/// Percent-per-token needs the window to have moved at least this much.
const MIN_DELTA_PCT: f64 = 1.0;
/// A typical session needs this many past sessions to be typical.
const MIN_SESSIONS: usize = 3;
/// The most points the burn-down carries (thinned evenly, newest kept).
const MAX_POINTS: usize = 120;

/// One reading on the burn-down.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize)]
pub struct BurnPoint {
    /// When, epoch ms.
    pub at_ms: i64,
    /// Percent of the window used then.
    pub used_pct: f64,
}

/// A window's runway.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct Runway {
    /// The window's readings so far, oldest first.
    pub series: Vec<BurnPoint>,
    /// When the window began (its reset minus its length).
    pub starts_ms: i64,
    /// When it resets.
    pub resets_at_ms: i64,
    /// Percent used now (the latest reading).
    pub used_pct: f64,
    /// The recent pace, percent of the window per hour, when measurable.
    pub pct_per_hour: Option<f64>,
    /// When it runs out at that pace — only when that's before the reset.
    pub exhausts_at_ms: Option<i64>,
    /// What a typical session of this account costs, percent of the
    /// window, when enough is known.
    pub session_pct: Option<f64>,
    /// Whole typical sessions left in the window, when `session_pct` is.
    pub sessions_left: Option<u32>,
    /// The past sessions the typical one is the median of.
    pub sessions_counted: usize,
    /// The lowest confidence of what went in, never above Computed: the
    /// figures are worked out, not read.
    pub confidence: Confidence,
}

fn tokens(counts: &TokenCounts) -> u64 {
    counts.input + counts.output + counts.cache_read + counts.cache_write + counts.reasoning
}

#[allow(clippy::cast_precision_loss)]
fn per_hour(delta_pct: f64, span_ms: i64) -> f64 {
    delta_pct * 3_600_000.0 / span_ms as f64
}

/// Evenly thins `points` to at most `max`, keeping the first and the last.
fn thinned(points: Vec<BurnPoint>, max: usize) -> Vec<BurnPoint> {
    if points.len() <= max || max < 2 {
        return points;
    }
    let last = points.len() - 1;
    (0..max).map(|i| points[i * last / (max - 1)]).collect()
}

/// The runway of the window `samples` describe (one window kind, one
/// account), `window_ms` long, at `now_ms`. `events` are the account's
/// usage events (a week is plenty): the tokens between readings, and the
/// past sessions the typical one comes from. `None` when there's no
/// Measured reading of a window that hasn't reset yet.
#[must_use]
pub fn runway(
    samples: &[QuotaSample],
    events: &[UsageEvent],
    window_ms: i64,
    now_ms: i64,
) -> Option<Runway> {
    let measured =
        |sample: &&QuotaSample| sample.confidence == Confidence::Measured && sample.ts <= now_ms;
    let latest = samples
        .iter()
        .filter(measured)
        .max_by_key(|sample| sample.ts)?;
    let resets_at = latest.resets_at.filter(|at| *at > now_ms)?;
    let mut window: Vec<&QuotaSample> = samples
        .iter()
        .filter(measured)
        .filter(|sample| {
            sample
                .resets_at
                .is_some_and(|at| (at - resets_at).abs() <= SAME_RESET_MS)
        })
        .collect();
    window.sort_by_key(|sample| sample.ts);
    let used = latest.used_pct.clamp(0.0, 100.0);

    // The pace: the newest reading against the oldest within the last hour.
    let since = latest.ts - PACE_SPAN_MS;
    let pace_from = window.iter().find(|sample| sample.ts >= since).copied();
    let pct_per_hour = pace_from
        .filter(|first| latest.ts - first.ts >= PACE_MIN_MS)
        .map(|first| {
            per_hour(
                (latest.used_pct - first.used_pct).max(0.0),
                latest.ts - first.ts,
            )
        });
    let exhausts_at_ms = pct_per_hour.filter(|rate| *rate > 0.0).and_then(|rate| {
        #[allow(clippy::cast_possible_truncation)]
        let at = now_ms + ((100.0 - used) / rate * 3_600_000.0) as i64;
        (at < resets_at).then_some(at)
    });

    // Percent per token: the window's movement against the tokens seen
    // between its first and latest reading.
    let first = window.first().copied().unwrap_or(latest);
    let delta_pct = latest.used_pct - first.used_pct;
    let between: Vec<&UsageEvent> = events
        .iter()
        .filter(|event| event.ts > first.ts && event.ts <= latest.ts)
        .collect();
    let between_tokens: u64 = between.iter().map(|event| tokens(&event.tokens)).sum();
    #[allow(clippy::cast_precision_loss)]
    let pct_per_token = (delta_pct >= MIN_DELTA_PCT && between_tokens > 0)
        .then(|| delta_pct / between_tokens as f64);

    // A typical session: the median of the account's past sessions.
    let mut sessions: Vec<&str> = events
        .iter()
        .filter_map(|event| event.session.as_deref())
        .collect();
    sessions.sort_unstable();
    sessions.dedup();
    let counted = sessions
        .iter()
        .filter(|session| {
            events.iter().any(|event| {
                event.session.as_deref() == Some(**session) && tokens(&event.tokens) > 0
            })
        })
        .count();
    let median = (counted >= MIN_SESSIONS)
        .then(|| median_session_burn(events, &sessions))
        .flatten();
    #[allow(clippy::cast_precision_loss)]
    let session_pct = pct_per_token
        .zip(median)
        .map(|(per_token, median)| per_token * median as f64)
        .filter(|pct| *pct > 0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let sessions_left =
        session_pct.map(|pct| ((100.0 - used) / pct).floor().clamp(0.0, 999.0) as u32);

    let confidence = between
        .iter()
        .map(|event| event.confidence)
        .chain([Confidence::Measured, Confidence::Computed])
        .min_by_key(|confidence| confidence_rank(*confidence))
        .unwrap_or(Confidence::Computed);
    let series = thinned(
        window
            .iter()
            .map(|sample| BurnPoint {
                at_ms: sample.ts,
                used_pct: sample.used_pct.clamp(0.0, 100.0),
            })
            .collect(),
        MAX_POINTS,
    );
    Some(Runway {
        series,
        starts_ms: resets_at - window_ms,
        resets_at_ms: resets_at,
        used_pct: used,
        pct_per_hour,
        exhausts_at_ms,
        session_pct,
        sessions_left,
        sessions_counted: counted,
        confidence,
    })
}
