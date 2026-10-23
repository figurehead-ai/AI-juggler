//! Anti-fatigue alert policy (AJ-E611, deep/09 §Q5): the per-(account,
//! window) state machine `Armed → Warned → Critical → Exhausted →
//! (reset) → Armed` with NO backward transitions without a reset or
//! manual re-arm — which is what guarantees at-most-once semantics.
//! Forecast errors >2× never interrupt (silent calibration notes only,
//! AJ-E608). Non-actionable crossings batch into an hourly digest with
//! per-project mute + quiet hours. Every alert carries its confidence
//! badge; an Estimated 90% reads differently from Measured and the
//! copy says so.

/// The §Q5 phases, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum AlertPhase {
    /// Nothing crossed yet.
    #[default]
    Armed,
    /// TTE < 45 min crossed — tray/ring amber already fired.
    Warned,
    /// TTE < 15 min or ≥90% crossed — the actionable fired.
    Critical,
    /// The pool is spent (once per exhaustion).
    Exhausted,
}

/// One emission out of the machine (what the surface renders).
#[derive(Clone, PartialEq, Debug)]
pub enum AlertEmission {
    /// Nothing to show (the common case — fatigue never accumulates).
    None,
    /// TTE < 45 min: tray + ring amber, NO sound, once per window.
    TrayAmber {
        /// The badge copy ("≈45 min (estimate)" reads differently).
        confidence: crate::model::Confidence,
    },
    /// TTE < 15 min or ≥90%: the actionable — Hand over / Queue /
    /// Snooze 30m, once.
    Actionable {
        /// Badge for the copy.
        confidence: crate::model::Confidence,
    },
    /// Exhausted: notification + inbox row, once per exhaustion.
    ExhaustedNotification,
    /// Reset reached (a queued item): click-to-resume notification.
    ResetReached,
}

/// The machine state for one (account, window).
#[derive(Clone, PartialEq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AlertFsm {
    /// Current phase.
    pub phase: AlertPhase,
    /// Snooze deadline (the actionable re-fires only after it).
    pub snoozed_until_ms: Option<i64>,
    /// AJ-E1513: the reset instant of the window this machine fired in,
    /// epoch ms. Once it passes (or a later window is reported) the
    /// machine re-arms; without one it never re-arms on a guess.
    #[serde(default)]
    pub window_resets_at_ms: Option<i64>,
}

/// The thresholds (§Q5 table, verbatim).
pub const TTE_TRAY_MIN: f64 = 45.0;
/// The critical TTE bound.
pub const TTE_CRITICAL_MIN: f64 = 15.0;
/// The critical used-percentage bound.
pub const PCT_CRITICAL: f64 = 90.0;
/// Snooze length.
pub const SNOOZE_MS: i64 = 30 * 60 * 1000;

/// One tick's observations for a window.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct WindowObservation {
    /// Current TTE minutes, when a forecast exists.
    pub tte_min: Option<f64>,
    /// Current used percentage, when the state carries one.
    pub used_pct: Option<f64>,
    /// The reading's confidence (Estimated alerts read differently).
    pub confidence: crate::model::Confidence,
    /// The window's reset instant, epoch ms, when one is known.
    pub resets_at_ms: Option<i64>,
}

/// How much later a reported reset instant must be than the recorded
/// one to count as the next window: readings of one window jitter by
/// seconds, while the next window resets hours later.
pub const NEXT_WINDOW_SLACK_MS: i64 = 30 * 60 * 1000;

/// AJ-E1513: re-arms a machine whose window has reset — `now_ms` is
/// past the reset instant it recorded when it fired, or `resets_at_ms`
/// reports a later window. A machine with no recorded instant stays as
/// it is: it never re-arms on a guess.
#[must_use]
pub fn rearmed(fsm: &AlertFsm, resets_at_ms: Option<i64>, now_ms: i64) -> AlertFsm {
    match fsm.window_resets_at_ms {
        Some(end)
            if fsm.phase != AlertPhase::Armed
                && (now_ms >= end
                    || resets_at_ms.is_some_and(|next| next > end + NEXT_WINDOW_SLACK_MS)) =>
        {
            AlertFsm::default()
        }
        _ => fsm.clone(),
    }
}

/// Steps the machine with one observation. Pure — the caller owns
/// time and persistence. Returns the emission (at most one) and the
/// next state. Backward transitions never happen without a reset —
/// [`AlertSignal::Reset`], or the window's own reset ([`rearmed`]): a
/// re-crossing in the same window emits nothing (the at-most-once law).
#[must_use]
pub fn step(
    fsm: &AlertFsm,
    observation: WindowObservation,
    now_ms: i64,
) -> (AlertFsm, AlertEmission) {
    let fsm = &rearmed(fsm, observation.resets_at_ms, now_ms);
    let window_resets_at_ms = fsm.window_resets_at_ms.or(observation.resets_at_ms);
    let confidence = observation.confidence;
    let snoozed = fsm.snoozed_until_ms.is_some_and(|until| now_ms < until);

    // Exhaustion is terminal until reset — later observations emit
    // nothing (once per exhaustion).
    if fsm.phase == AlertPhase::Exhausted {
        return (fsm.clone(), AlertEmission::None);
    }

    let critical = observation
        .tte_min
        .is_some_and(|tte| tte < TTE_CRITICAL_MIN)
        || observation.used_pct.is_some_and(|pct| pct >= PCT_CRITICAL);
    let warned = observation.tte_min.is_some_and(|tte| tte < TTE_TRAY_MIN);

    if critical && !snoozed {
        let next = AlertFsm {
            phase: AlertPhase::Critical,
            snoozed_until_ms: fsm.snoozed_until_ms,
            window_resets_at_ms,
        };
        let emission = if fsm.phase == AlertPhase::Armed || fsm.phase == AlertPhase::Warned {
            AlertEmission::Actionable { confidence }
        } else {
            AlertEmission::None // already Critical: at-most-once
        };
        return (next, emission);
    }
    if critical && snoozed {
        // Snoozed: the digest batches the crossing (never a second
        // interruption).
        return (fsm.clone(), AlertEmission::None);
    }
    if warned {
        // Forward-only: an already-Critical machine never decays back
        // to Warned without a reset (the at-most-once law).
        let phase = if fsm.phase == AlertPhase::Critical {
            AlertPhase::Critical
        } else {
            AlertPhase::Warned
        };
        let next = AlertFsm {
            phase,
            snoozed_until_ms: fsm.snoozed_until_ms,
            window_resets_at_ms,
        };
        let emission = if fsm.phase == AlertPhase::Armed {
            AlertEmission::TrayAmber { confidence }
        } else {
            AlertEmission::None
        };
        return (next, emission);
    }
    (fsm.clone(), AlertEmission::None)
}

/// The machine's control signals (not observations).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AlertSignal {
    /// The pool is spent (`QuotaBlocked` / Measured 100%).
    Exhausted,
    /// The window reset — re-arms everything.
    Reset,
    /// The user snoozed the actionable (30 min).
    Snooze,
    /// A forecast missed by >2× — silent calibration only.
    ForecastMissed,
}

/// Applies a control signal. `Exhausted`/`Reset` are the only backward
/// movers (reset re-arms); `ForecastMissed` NEVER emits.
#[must_use]
pub fn signal(fsm: &AlertFsm, sig: AlertSignal, now_ms: i64) -> (AlertFsm, AlertEmission) {
    match sig {
        AlertSignal::Exhausted => {
            let next = AlertFsm {
                phase: AlertPhase::Exhausted,
                snoozed_until_ms: None,
                window_resets_at_ms: fsm.window_resets_at_ms,
            };
            let emission = if fsm.phase == AlertPhase::Exhausted {
                AlertEmission::None
            } else {
                AlertEmission::ExhaustedNotification
            };
            (next, emission)
        }
        AlertSignal::Reset => (AlertFsm::default(), AlertEmission::ResetReached),
        AlertSignal::Snooze => (
            AlertFsm {
                phase: fsm.phase,
                snoozed_until_ms: Some(now_ms + SNOOZE_MS),
                window_resets_at_ms: fsm.window_resets_at_ms,
            },
            AlertEmission::None,
        ),
        AlertSignal::ForecastMissed => (fsm.clone(), AlertEmission::None),
    }
}

/// The hourly digest buffer: non-actionable crossings batch here;
/// per-project mute + quiet hours gate the flush.
#[derive(Clone, PartialEq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Digest {
    /// Batched entries (project-keyed for the mute rule).
    pub entries: Vec<DigestEntry>,
    /// Last flush.
    pub last_flush_ms: Option<i64>,
}

/// One digest line.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct DigestEntry {
    /// Project (or account) the crossing belongs to.
    pub key: String,
    /// The copy ("crossed 90% (estimate)").
    pub text: String,
}

/// Flush window (hourly by default, deep/09 §Q5).
pub const DIGEST_EVERY_MS: i64 = 3_600_000;

/// Adds a non-actionable crossing.
#[must_use]
pub fn digest_add(digest: &Digest, key: &str, text: &str) -> Digest {
    let mut next = digest.clone();
    next.entries.push(DigestEntry {
        key: key.to_owned(),
        text: text.to_owned(),
    });
    next
}

/// The local hours in which the digest waits (it is deferred, never
/// dropped). The user sets them in Settings › Notifications; the
/// default is 22:00–07:00.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct QuietHours {
    /// First quiet hour, 0–23.
    pub start: u32,
    /// First hour that is no longer quiet, 0–23. `start == end` means no
    /// quiet hours at all.
    pub end: u32,
}

impl QuietHours {
    /// 22:00–07:00 local.
    pub const DEFAULT: Self = Self { start: 22, end: 7 };

    /// Whether `hour` (0–23, local) falls in the quiet hours. A span
    /// may wrap past midnight.
    #[must_use]
    pub fn contains(self, hour: u32) -> bool {
        match self.start.cmp(&self.end) {
            std::cmp::Ordering::Equal => false,
            std::cmp::Ordering::Less => (self.start..self.end).contains(&hour),
            std::cmp::Ordering::Greater => hour >= self.start || hour < self.end,
        }
    }
}

impl Default for QuietHours {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Flushes the digest if due: hourly cadence, muted keys dropped,
/// quiet hours (22:00–07:00 local — the caller passes the local hour)
/// defer without dropping.
#[must_use]
pub fn digest_flush(
    digest: &Digest,
    now_ms: i64,
    muted_keys: &[String],
    local_hour: u32,
) -> (Digest, Vec<DigestEntry>) {
    digest_flush_with(digest, now_ms, muted_keys, local_hour, QuietHours::DEFAULT)
}

/// [`digest_flush`] with the user's own quiet hours.
#[must_use]
pub fn digest_flush_with(
    digest: &Digest,
    now_ms: i64,
    muted_keys: &[String],
    local_hour: u32,
    quiet_hours: QuietHours,
) -> (Digest, Vec<DigestEntry>) {
    let due = digest
        .last_flush_ms
        .is_none_or(|last| now_ms.saturating_sub(last) >= DIGEST_EVERY_MS);
    let quiet = quiet_hours.contains(local_hour);
    if !due || quiet {
        return (digest.clone(), Vec::new()); // deferred, never dropped
    }
    let flushed: Vec<DigestEntry> = digest
        .entries
        .iter()
        .filter(|entry| !muted_keys.contains(&entry.key))
        .cloned()
        .collect();
    (
        Digest {
            entries: Vec::new(),
            last_flush_ms: Some(now_ms),
        },
        flushed,
    )
}
