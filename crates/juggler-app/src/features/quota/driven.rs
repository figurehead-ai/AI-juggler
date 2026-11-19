//! F-rc3g-1 (the 0.4.6 rc3f walk): a session AI Juggler runs reports its
//! account's limits as it works ([`RuntimeEvent::LimitsUpdated`]: Claude
//! Code's `rate_limit_event`, Codex's `account/rateLimits/updated`). That
//! was held only for the session's own panel, which said "5 hours: 36%
//! used" while the account's runway on the same page said "100% left, from
//! the Claude app, 1 h ago", until the account ran out.
//!
//! Here each update is kept as a reading of the folder the session runs on
//! ([`UsageSource::Driven`], "Codex's run in AI Juggler"), through the
//! quota store every other reading goes through, so the runway, the Out
//! marks, switch room and notices read the freshest reading AI Juggler
//! holds by the same newest-reading rule as any other.
//!
//! - Only the account's own windows: a 5-hour or weekly window, as long as
//!   the tool says, that limits every model ([`LimitWindow::model`] is
//!   `None`). A week on one model, or a window of another length, is the
//!   panel's only.
//! - Writes are bounded: a reading is stored when it differs from the last
//!   one stored for its folder; the same reading again only after
//!   [`SAME_AGAIN_MS`] (it says the reading still holds). Never once per
//!   event.
//!
//! [`RuntimeEvent::LimitsUpdated`]: juggler_adapter::runtime::RuntimeEvent::LimitsUpdated
//! [`LimitWindow::model`]: juggler_adapter::runtime::LimitWindow::model

use std::collections::HashMap;
use std::sync::Mutex;

use juggler_adapter::runtime::LimitsSnapshot;
use juggler_core::model::{Confidence, SessionId, Timestamp};
use juggler_core::quota::{QuotaSample, UsageSource, WindowKind};

use crate::state::AppState;

/// The least time before the same reading is stored again for a folder.
pub const SAME_AGAIN_MS: i64 = 60_000;

/// One stored reading, as compared: each window's name, use and reset.
type Reading = Vec<(&'static str, u64, Option<i64>)>;

/// The last reading stored per folder, and when: held in memory only.
#[derive(Debug, Default)]
pub struct DrivenReadings {
    last: Mutex<HashMap<String, (Reading, i64)>>,
}

/// The account windows `limits` names, as readings taken at `now_ms`: a
/// 5-hour or weekly window on every model, at most one of each.
#[must_use]
pub fn samples_of(limits: &LimitsSnapshot, now_ms: i64) -> Vec<(WindowKind, QuotaSample)> {
    let kinds = [WindowKind::FiveHour, WindowKind::Weekly { anchor_ms: None }];
    let mut out: Vec<(WindowKind, QuotaSample)> = Vec::new();
    for window in limits
        .windows
        .iter()
        .filter(|window| window.model.is_none())
    {
        let Some(mins) = window.window_mins else {
            continue;
        };
        let Some(kind) = kinds
            .into_iter()
            .find(|kind| kind.width_ms() == i64::from(mins) * 60_000)
        else {
            continue;
        };
        if out.iter().any(|(seen, _)| seen.as_str() == kind.as_str()) {
            continue;
        }
        out.push((
            kind,
            QuotaSample {
                ts: now_ms,
                used_pct: window.used_pct.clamp(0.0, 100.0),
                resets_at: window.resets_at_ms,
                confidence: Confidence::Measured,
                held_until: None,
            },
        ));
    }
    out
}

fn compared(samples: &[(WindowKind, QuotaSample)]) -> Reading {
    samples
        .iter()
        .map(|(kind, sample)| (kind.as_str(), sample.used_pct.to_bits(), sample.resets_at))
        .collect()
}

/// Keeps `session`'s limits update as a reading of the folder it runs on.
/// `None` while the session's record isn't stored yet (the driver tries
/// again); `Some(true)` when a reading was stored, `Some(false)` when
/// there was nothing to store (no account window, the same reading stored
/// under a minute ago, or no folder).
pub fn note(state: &AppState, session: &SessionId, limits: &LimitsSnapshot) -> Option<bool> {
    note_at(state, session, limits, Timestamp::now().millis())
}

/// [`note`] at `now_ms`.
fn note_at(
    state: &AppState,
    session: &SessionId,
    limits: &LimitsSnapshot,
    now_ms: i64,
) -> Option<bool> {
    let samples = samples_of(limits, now_ms);
    if samples.is_empty() {
        return Some(false);
    }
    let reading = compared(&samples);
    let Ok(storage) = state.storage.lock() else {
        return Some(false);
    };
    let record = storage.sessions().get(session).ok()?;
    let Ok(profile) = storage.profiles().get(&record.profile) else {
        return Some(false);
    };
    let folder = profile.account.as_str().to_owned();
    let Ok(mut last) = state.driven_readings.last.lock() else {
        return Some(false);
    };
    if last
        .get(&folder)
        .is_some_and(|(kept, at)| *kept == reading && now_ms.saturating_sub(*at) < SAME_AGAIN_MS)
    {
        return Some(false);
    }
    let rows: Vec<_> = samples
        .into_iter()
        .map(|(kind, sample)| (folder.clone(), kind, sample, UsageSource::Driven))
        .collect();
    if storage.usage().insert_sourced_samples(&rows).is_err() {
        return Some(false);
    }
    last.insert(folder, (reading, now_ms));
    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::runtime::LimitWindow;
    use juggler_core::model::SessionStatus;

    const T0: i64 = 1_795_000_000_000;

    fn window(mins: Option<u32>, used_pct: f64, model: Option<&str>) -> LimitWindow {
        LimitWindow {
            id: "w".into(),
            window_mins: mins,
            used_pct,
            resets_at_ms: Some(T0 + 3_600_000),
            model: model.map(str::to_owned),
        }
    }

    fn limits(five_hour: f64) -> LimitsSnapshot {
        LimitsSnapshot {
            windows: vec![
                window(Some(300), five_hour, None),
                window(Some(10_080), 5.0, None),
                window(Some(10_080), 40.0, Some("opus")),
            ],
            plan: None,
            reached: false,
        }
    }

    #[test]
    fn only_the_accounts_own_windows_are_readings() {
        let samples = samples_of(&limits(36.0), T0);
        let kinds: Vec<(&str, f64)> = samples
            .iter()
            .map(|(kind, sample)| (kind.as_str(), sample.used_pct))
            .collect();
        assert_eq!(kinds, [("five_hour", 36.0), ("weekly", 5.0)]);
        assert!(samples.iter().all(|(_, sample)| sample.ts == T0
            && sample.confidence == Confidence::Measured
            && sample.resets_at == Some(T0 + 3_600_000)));
        // A window of another length, or none given: the panel's only.
        let other = LimitsSnapshot {
            windows: vec![window(Some(60), 10.0, None), window(None, 10.0, None)],
            ..LimitsSnapshot::default()
        };
        assert!(samples_of(&other, T0).is_empty());
    }

    /// A driven session on folder `acc`, stored as the driver finds it.
    fn driven_session(state: &AppState) -> SessionId {
        let storage = state.storage.lock().unwrap();
        let dir = state.data_dir.join("acc");
        storage
            .accounts()
            .insert(&crate::test_support::account(
                "acc",
                "tool-x",
                "Lab",
                dir.clone(),
            ))
            .unwrap();
        storage
            .profiles()
            .insert(&crate::test_support::launch_settings("p", "lab", "acc"))
            .unwrap();
        storage
            .sessions()
            .insert(&crate::test_support::instance(
                "s",
                "p",
                SessionStatus::Running,
                dir,
            ))
            .unwrap();
        SessionId::new("s")
    }

    fn stored(state: &AppState) -> Vec<(QuotaSample, Option<UsageSource>)> {
        state
            .storage
            .lock()
            .unwrap()
            .usage()
            .sourced_samples_since("acc", WindowKind::FiveHour, 0)
            .unwrap()
    }

    #[test]
    fn a_reading_is_stored_on_change_and_the_same_one_at_most_once_a_minute() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::bare_state(dir.path().join("data"));
        // Before the session's record is stored: tried again later.
        assert_eq!(
            note_at(&state, &SessionId::new("s"), &limits(36.0), T0),
            None
        );
        let session = driven_session(&state);

        assert_eq!(note_at(&state, &session, &limits(36.0), T0), Some(true));
        let first = stored(&state);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0.used_pct, 36.0);
        assert_eq!(first[0].1, Some(UsageSource::Driven));

        // The same reading again within the minute: nothing is written.
        assert_eq!(
            note_at(&state, &session, &limits(36.0), T0 + 5_000),
            Some(false)
        );
        assert_eq!(
            note_at(&state, &session, &limits(36.0), T0 + 59_999),
            Some(false)
        );
        assert_eq!(stored(&state), first, "not a row, not a last-seen");

        // A new one is stored at once.
        assert_eq!(
            note_at(&state, &session, &limits(37.0), T0 + 60_100),
            Some(true)
        );
        assert_eq!(stored(&state).last().unwrap().0.used_pct, 37.0);
        // The same one a minute on: stored, it still holds.
        assert_eq!(
            note_at(&state, &session, &limits(37.0), T0 + 120_100),
            Some(true)
        );
        assert_eq!(stored(&state).last().unwrap().0.ts, T0 + 120_100);

        // Nothing the account's: nothing stored, nothing remembered.
        let model_only = LimitsSnapshot {
            windows: vec![window(Some(10_080), 90.0, Some("opus"))],
            ..LimitsSnapshot::default()
        };
        assert_eq!(
            note_at(&state, &session, &model_only, T0 + 130_000),
            Some(false)
        );
    }
}
