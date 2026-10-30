//! 0.3: a session that hits its usage limit and stays open says so.
//!
//! The exit classifier (AJ-E405, [`super::lifecycle`]) marks a session
//! [`SessionStatus::QuotaBlocked`] only when its process ends with a
//! rate-limit signature. An interactive tool (Claude Code in a terminal)
//! shows its limit message and stays open, so its session never ended and
//! was never marked: the limit row, the decision dialog, Park and the
//! session page's switch list never showed for it.
//!
//! Here an open session is marked from what its tool provides, never from
//! a tool id (ADR-0019):
//!
//! - **A terminal session** whose adapter declares
//!   [`Capabilities::LIMIT_HITS`]: the tool's own record of the limit its
//!   last turn stopped at, read from its transcript's tail
//!   ([`juggler_adapter::HarnessAdapter::limit_hit`]), only when the file
//!   changed. Looked at on the heuristic tick, when its output moves, and at
//!   once when its hooks say a turn failed.
//! - **A driven session**: its runtime's own word, a rate-limit error the
//!   tool won't retry; a new turn clears it.
//!
//! A hit counts only when it was written after the session started (an
//! older line in a resumed conversation never marks it again) and its reset
//! hasn't passed. The mark goes when the session carries on (the model
//! replied, or a driven session's new turn), or when the reset passes: the
//! account has room again. A warning that a limit is near is no hit (the
//! adapter's rule). Like the exit classifier, a hit is the tool's own fact,
//! so it marks at once, with no debounce; nothing acts on it but the user.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use juggler_adapter::Capabilities;
use juggler_adapter::live::LimitHit;
use juggler_adapter::runtime::{ErrorCategory, LimitsSnapshot, RuntimeEvent};
use juggler_core::model::{AttentionReason, SessionId, SessionStatus, Timestamp};
use juggler_core::quota::WindowKind;

use crate::state::AppState;

/// The attention rule of an open session at its limit (its process runs).
pub const RULE: &str = "limit_hit";
/// The exit classifier's rule: the session ended at its limit.
pub const EXIT_RULE: &str = "quota_exit";
/// How often a session whose output moves is looked at, at most.
pub const OUTPUT_EVERY: std::time::Duration = std::time::Duration::from_secs(1);

/// A transcript's tail as last read for a session: the file, its size and
/// time, and the hit it held.
#[derive(Clone, Debug)]
struct Seen {
    path: PathBuf,
    len: u64,
    modified: Option<SystemTime>,
    hit: Option<LimitHit>,
}

/// What the limit watch keeps, in memory only: each session's tail as
/// last read, and the reset of each driven session's mark.
#[derive(Debug, Default)]
pub struct LimitWatch {
    tails: Mutex<HashMap<String, Seen>>,
    /// Driven sessions marked by their runtime, with the reset when known.
    driven: Mutex<HashMap<String, Option<i64>>>,
    /// When each session was last looked at because its output moved.
    looked: Mutex<HashMap<String, std::time::Instant>>,
}

impl LimitWatch {
    /// The hit `path`'s tail holds, read again only when the file changed.
    fn tail_hit(
        &self,
        session: &str,
        path: &Path,
        read: impl FnOnce(&str) -> Option<LimitHit>,
    ) -> Option<LimitHit> {
        let Ok(meta) = std::fs::metadata(path) else {
            self.forget_tail(session);
            return None;
        };
        let (len, modified) = (meta.len(), meta.modified().ok());
        if let Ok(tails) = self.tails.lock()
            && let Some(seen) = tails.get(session)
            && seen.path == path
            && seen.len == len
            && seen.modified == modified
        {
            return seen.hit;
        }
        let hit = crate::features::conversations::asks::tail(path).and_then(|text| read(&text));
        if let Ok(mut tails) = self.tails.lock() {
            tails.insert(
                session.to_owned(),
                Seen {
                    path: path.to_path_buf(),
                    len,
                    modified,
                    hit,
                },
            );
        }
        hit
    }

    fn forget_tail(&self, session: &str) {
        if let Ok(mut tails) = self.tails.lock() {
            tails.remove(session);
        }
    }

    /// Whether `session` may be looked at again for its output: at most
    /// once per [`OUTPUT_EVERY`] (output moves many times a second).
    fn due(&self, session: &str) -> bool {
        let now = std::time::Instant::now();
        let Ok(mut looked) = self.looked.lock() else {
            return false;
        };
        let due = looked
            .get(session)
            .is_none_or(|at| now.saturating_duration_since(*at) >= OUTPUT_EVERY);
        if due {
            looked.insert(session.to_owned(), now);
        }
        due
    }

    /// Forgets a session that ended.
    pub fn forget(&self, session: &str) {
        self.forget_tail(session);
        if let Ok(mut looked) = self.looked.lock() {
            looked.remove(session);
        }
        if let Ok(mut driven) = self.driven.lock() {
            driven.remove(session);
        }
    }
}

/// Whether `hit` holds now for a session started at `started_ms`: written
/// after it started (a hit already in a resumed conversation is history),
/// and its reset, when known, still ahead.
#[must_use]
pub fn holds(hit: &LimitHit, started_ms: i64, now_ms: i64) -> bool {
    hit.at_ms >= started_ms && hit.resets_at_ms.is_none_or(|reset| reset > now_ms)
}

/// What a session's mark should become: `Some(true)` to mark it (an open
/// session whose hit holds), `Some(false)` to clear it (marked here, and
/// the hit no longer holds), `None` to leave it. A session the exit
/// classifier marked, or in any other state, is never touched.
#[must_use]
pub fn wanted(status: SessionStatus, rule: Option<&str>, hit_holds: bool) -> Option<bool> {
    match status {
        SessionStatus::Starting
        | SessionStatus::Running
        | SessionStatus::WaitingUser
        | SessionStatus::Stuck
            if hit_holds =>
        {
            Some(true)
        }
        SessionStatus::QuotaBlocked if rule == Some(RULE) && !hit_holds => Some(false),
        _ => None,
    }
}

/// The window that ran out, in words (the reason's input).
fn window_words(window: Option<WindowKind>) -> &'static str {
    match window {
        Some(WindowKind::FiveHour) => "5-hour limit",
        Some(WindowKind::Weekly { .. }) => "weekly limit",
        _ => "usage limit",
    }
}

/// What the watch needs of a session's record.
struct Facts {
    status: SessionStatus,
    rule: Option<String>,
    started_ms: i64,
    transcript: Option<PathBuf>,
    caps: Capabilities,
    adapter: Option<juggler_adapter::SharedAdapter>,
}

fn facts(state: &Arc<AppState>, session: &SessionId) -> Option<Facts> {
    let storage = state.storage.lock().ok()?;
    let record = storage.sessions().get(session).ok()?;
    let harness = storage
        .profiles()
        .get(&record.profile)
        .ok()
        .and_then(|profile| storage.accounts().get(&profile.account).ok())
        .map(|account| account.harness);
    drop(storage);
    let adapter = harness.and_then(|harness| state.registry.get(&harness));
    Some(Facts {
        status: record.status,
        rule: record.attention.map(|reason| reason.rule),
        started_ms: record.started_at.millis(),
        transcript: record.transcript_ref.map(|reference| reference.path),
        caps: adapter
            .as_ref()
            .map_or_else(Capabilities::empty, |adapter| adapter.capabilities()),
        adapter,
    })
}

/// Marks `session` at its limit, or clears its mark; `true` when its
/// stored state changed. The stored status moves only from (or to) the
/// states [`wanted`] names.
fn settle(state: &Arc<AppState>, session: &SessionId, hit: Option<&LimitHit>, mark: bool) -> bool {
    let status = {
        let Ok(storage) = state.storage.lock() else {
            return false;
        };
        let Ok(mut record) = storage.sessions().get(session) else {
            return false;
        };
        let rule = record.attention.as_ref().map(|reason| reason.rule.as_str());
        let holds = mark && hit.is_some();
        if wanted(record.status, rule, holds) != Some(mark) {
            return false;
        }
        if let (true, Some(hit)) = (mark, hit) {
            record.status = SessionStatus::QuotaBlocked;
            record.attention = Some(AttentionReason {
                rule: RULE.to_owned(),
                inputs: vec![window_words(hit.window).to_owned()],
                since_ms: hit.at_ms,
            });
        } else {
            record.status = SessionStatus::Running;
            record.attention = None;
        }
        record.last_activity = Some(Timestamp::now());
        if storage.sessions().update(&record).is_err() {
            return false;
        }
        crate::usage_stats::instance_state_entered(&storage, &record);
        record.status
    };
    // Needs you, Home's limit row and the notification follow the status.
    crate::features::notifications::session_status(state, session.as_str(), status.as_str());
    true
}

/// Looks at one session's transcript for a limit hit (a terminal session
/// whose tool records them); `true` when its stored state changed. A
/// driven session's mark is its runtime's ([`driven`]), and only expires
/// here. A session whose process ended is the exit classifier's.
pub fn check(state: &Arc<AppState>, session: &SessionId) -> bool {
    let Some(facts) = facts(state, session) else {
        return false;
    };
    let now = Timestamp::now().millis();
    if state.runtime.is_driven(session) {
        let reset = state
            .limits
            .driven
            .lock()
            .ok()
            .and_then(|driven| driven.get(session.as_str()).copied());
        let expired = reset.flatten().is_some_and(|reset| reset <= now);
        if expired && facts.rule.as_deref() == Some(RULE) {
            state.limits.forget(session.as_str());
            return settle(state, session, None, false);
        }
        return false;
    }
    if !facts.caps.contains(Capabilities::LIMIT_HITS) || state.supervisor.is_finished(session) {
        return false;
    }
    let (Some(adapter), Some(path)) = (facts.adapter, facts.transcript) else {
        return false;
    };
    let hit = state
        .limits
        .tail_hit(session.as_str(), &path, |tail| adapter.limit_hit(tail));
    let hit_holds = hit
        .as_ref()
        .is_some_and(|hit| holds(hit, facts.started_ms, now));
    match wanted(facts.status, facts.rule.as_deref(), hit_holds) {
        Some(true) => settle(state, session, hit.as_ref(), true),
        Some(false) => settle(state, session, None, false),
        None => false,
    }
}

/// [`check`] for a session whose output moved, at most once per
/// [`OUTPUT_EVERY`]; `true` when its stored state changed.
pub fn output_moved(state: &Arc<AppState>, session: &SessionId) -> bool {
    state.limits.due(session.as_str()) && check(state, session)
}

/// [`check`] for every session still open; `true` when any changed.
pub fn tick(state: &Arc<AppState>) -> bool {
    let open: Vec<SessionId> = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.sessions().list(None).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|record| crate::commands::is_active(record.status))
        .map(|record| record.id)
        .collect();
    let mut changed = false;
    for session in &open {
        changed |= check(state, session);
    }
    changed
}

/// The reset a driven session's limits name for the limit it reached: the
/// latest reset of a window used up, when the tool says.
fn reset_of(limits: Option<&LimitsSnapshot>) -> Option<i64> {
    limits?
        .windows
        .iter()
        .filter(|window| window.used_pct >= 100.0)
        .filter_map(|window| window.resets_at_ms)
        .max()
}

/// A driven session's event: a rate-limit error the tool won't retry
/// marks it at its limit; a new turn clears the mark. `true` when its
/// stored state changed.
pub fn driven(
    state: &Arc<AppState>,
    session: &SessionId,
    event: &RuntimeEvent,
    limits: Option<&LimitsSnapshot>,
) -> bool {
    match event {
        RuntimeEvent::Error {
            category: ErrorCategory::RateLimit,
            will_retry: false,
            ..
        } => {
            let hit = LimitHit {
                at_ms: Timestamp::now().millis(),
                resets_at_ms: reset_of(limits),
                window: None,
            };
            if let Ok(mut driven) = state.limits.driven.lock() {
                driven.insert(session.to_string(), hit.resets_at_ms);
            }
            settle(state, session, Some(&hit), true)
        }
        RuntimeEvent::TurnStarted { .. } => {
            let marked = state
                .limits
                .driven
                .lock()
                .ok()
                .and_then(|mut driven| driven.remove(session.as_str()))
                .is_some();
            marked && settle(state, session, None, false)
        }
        _ => false,
    }
}

/// At a session's exit: the reason to keep it at its limit when it was
/// marked open at it (it ended there: the exit classifier's rule, so the
/// session reads as ended), else `None`. Its tail and mark are forgotten.
#[must_use]
pub fn at_exit(state: &Arc<AppState>, session: &SessionId) -> Option<AttentionReason> {
    state.limits.forget(session.as_str());
    let storage = state.storage.lock().ok()?;
    let record = storage.sessions().get(session).ok()?;
    let reason = record.attention?;
    (record.status == SessionStatus::QuotaBlocked && reason.rule == RULE).then(|| AttentionReason {
        rule: EXIT_RULE.to_owned(),
        ..reason
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_792_000_000_000;

    fn hit(at_ms: i64, resets_at_ms: Option<i64>) -> LimitHit {
        LimitHit {
            at_ms,
            resets_at_ms,
            window: Some(WindowKind::FiveHour),
        }
    }

    #[test]
    fn a_hit_holds_from_after_the_start_until_its_reset() {
        let started = T0;
        assert!(holds(
            &hit(T0 + 60_000, Some(T0 + 3_600_000)),
            started,
            T0 + 120_000
        ));
        // An older line in a resumed conversation never marks it again.
        assert!(!holds(
            &hit(T0 - 60_000, Some(T0 + 3_600_000)),
            started,
            T0 + 120_000
        ));
        // The reset passed: the account has room again.
        assert!(!holds(
            &hit(T0 + 60_000, Some(T0 + 3_600_000)),
            started,
            T0 + 3_600_000
        ));
        // No reset recorded: it holds until the session carries on.
        assert!(holds(&hit(T0 + 60_000, None), started, T0 + 86_400_000));
    }

    #[test]
    fn only_an_open_session_is_marked_and_only_its_own_mark_cleared() {
        use SessionStatus as S;
        for open in [S::Starting, S::Running, S::WaitingUser, S::Stuck] {
            assert_eq!(wanted(open, None, true), Some(true));
            assert_eq!(wanted(open, None, false), None);
        }
        assert_eq!(wanted(S::QuotaBlocked, Some(RULE), false), Some(false));
        assert_eq!(wanted(S::QuotaBlocked, Some(RULE), true), None);
        // The exit classifier's mark is never cleared here.
        assert_eq!(wanted(S::QuotaBlocked, Some(EXIT_RULE), false), None);
        for ended in [S::Finished, S::Stopped, S::Failed, S::HandedOver] {
            assert_eq!(wanted(ended, None, true), None);
        }
    }

    #[test]
    fn a_driven_reset_is_the_latest_of_the_windows_used_up() {
        use juggler_adapter::runtime::LimitWindow;
        let window = |used_pct: f64, resets_at_ms: Option<i64>| LimitWindow {
            id: "w".into(),
            window_mins: None,
            used_pct,
            resets_at_ms,
        };
        let limits = LimitsSnapshot {
            windows: vec![
                window(100.0, Some(T0 + 10)),
                window(40.0, Some(T0 + 99)),
                window(100.0, Some(T0 + 20)),
            ],
            plan: None,
            reached: true,
        };
        assert_eq!(reset_of(Some(&limits)), Some(T0 + 20));
        assert_eq!(reset_of(None), None);
        assert_eq!(
            window_words(Some(WindowKind::Weekly { anchor_ms: None })),
            "weekly limit"
        );
        assert_eq!(window_words(None), "usage limit");
    }
}
