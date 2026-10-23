//! AJ-E1509 Req.1 — a session that crashes is started again and resumed,
//! when the user asked for that (Settings › Sessions, or a launch
//! setting's own choice): the same launch settings, the same folder, its
//! conversation resumed. Never after a finish, a stop the user made, a
//! usage limit or a failed sign-in (restarting can't fix those, and must
//! never work around a limit: ADR-0003 point 3). At most [`TRIES`] tries
//! in [`WINDOW_MS`]; then it stays ended and says why. Each try is in
//! both sessions' history.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use juggler_core::model::{AttentionReason, SessionId, SessionStatus};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Tries within [`WINDOW_MS`] before a session stays ended.
pub const TRIES: usize = 3;
/// The window the tries are counted in: 10 minutes.
pub const WINDOW_MS: i64 = 10 * 60 * 1000;
/// How long after a crash the restart starts: a session that crashes at
/// once can't spin.
const BACKOFF: Duration = Duration::from_secs(2);

/// How a session ended, as the restarter needs to know it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// It exited cleanly (code 0).
    Finished,
    /// The user stopped it (the stop ladder, a quit that stops sessions).
    StoppedByUser,
    /// Its output carried a usage-limit signature.
    UsageLimit,
    /// It couldn't sign in (its output, or its account's sign-in state).
    SignInFailed,
    /// Anything else: it crashed.
    Crashed,
}

impl Ending {
    /// From what the lifecycle forwarder knows, in this order: how it
    /// exited, whether the user stopped it, whether its output tail was
    /// classed a usage limit, and the tail itself (a sign-in signature).
    #[must_use]
    pub fn of(
        kind: &juggler_supervisor::ExitKind,
        stopped_by_user: bool,
        usage_limit: bool,
        tail: &str,
    ) -> Self {
        if kind.is_success() {
            Self::Finished
        } else if stopped_by_user {
            Self::StoppedByUser
        } else if usage_limit {
            Self::UsageLimit
        } else if juggler_supervisor::signin_signature(tail) {
            Self::SignInFailed
        } else {
            Self::Crashed
        }
    }
}

/// What happens after a session ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Start it again: try `n` of [`TRIES`].
    Restart(usize),
    /// Never restarted: it didn't crash, or restarting can't help (why).
    Never(&'static str),
    /// It crashed, and restarting is off for it.
    Off,
    /// It crashed again after [`TRIES`] tries in [`WINDOW_MS`]: it stays
    /// ended.
    GaveUp,
}

/// Pure: whether a session that ended as `ending` is started again, given
/// whether restarting is on for it and when its earlier tries were.
#[must_use]
pub fn decide(ending: Ending, enabled: bool, earlier: &[i64], now_ms: i64) -> Verdict {
    match ending {
        Ending::Finished => return Verdict::Never("it finished"),
        Ending::StoppedByUser => return Verdict::Never("you stopped it"),
        Ending::UsageLimit => return Verdict::Never("it reached a usage limit"),
        Ending::SignInFailed => return Verdict::Never("it couldn't sign in"),
        Ending::Crashed => {}
    }
    if !enabled {
        return Verdict::Off;
    }
    let recent = earlier
        .iter()
        .filter(|at| now_ms.saturating_sub(**at) < WINDOW_MS)
        .count();
    if recent >= TRIES {
        Verdict::GaveUp
    } else {
        Verdict::Restart(recent + 1)
    }
}

/// The tries so far, by lineage: a restarted session counts with the
/// sessions it replaced. In memory: an app restart starts the count again.
#[derive(Default, Debug)]
pub struct Restarts {
    tries: HashMap<String, Vec<i64>>,
    root_of: HashMap<String, String>,
}

impl Restarts {
    fn root(&self, id: &str) -> String {
        self.root_of
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_owned())
    }

    /// When the earlier tries of `id`'s lineage were.
    #[must_use]
    pub fn earlier(&self, id: &str) -> Vec<i64> {
        self.tries.get(&self.root(id)).cloned().unwrap_or_default()
    }

    /// `crashed` was started again as `replacement` at `at_ms`.
    pub fn record(&mut self, crashed: &str, replacement: &str, at_ms: i64) {
        let root = self.root(crashed);
        self.tries.entry(root.clone()).or_default().push(at_ms);
        self.root_of.insert(replacement.to_owned(), root);
    }
}

/// A launch setting's own choice (`None`: follow Settings › Sessions).
///
/// # Errors
///
/// [`AppError::NotFound`] for an unknown launch setting; storage errors.
pub fn set_launch_choice(
    state: &Arc<AppState>,
    recipe_id: &str,
    restart: Option<bool>,
) -> AppResult<()> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let mut recipe = storage
        .profiles()
        .get(&juggler_core::model::ProfileId::new(recipe_id))?;
    recipe.settings_overlay.restart_on_crash = restart;
    storage.profiles().update(&recipe)?;
    Ok(())
}

/// Whether restarting is on for `session`: its launch setting's own
/// choice, else Settings › Sessions.
fn enabled_for(storage: &juggler_core::storage::Storage, session: &SessionId) -> bool {
    let own = storage
        .sessions()
        .get(session)
        .ok()
        .and_then(|record| storage.profiles().get(&record.profile).ok())
        .and_then(|profile| profile.settings_overlay.restart_on_crash);
    own.unwrap_or_else(|| crate::settings_policy::always_on(storage).restart_on_crash)
}

/// Whether `session`'s account can't sign in now (credentials expired or
/// half-made): restarting can't fix that.
fn cannot_sign_in(state: &Arc<AppState>, session: &SessionId) -> bool {
    use juggler_adapter::environment::LoginState;
    let Ok(storage) = state.storage.lock() else {
        return false;
    };
    let Some(account) = storage
        .sessions()
        .get(session)
        .ok()
        .and_then(|record| storage.profiles().get(&record.profile).ok())
        .and_then(|profile| storage.accounts().get(&profile.account).ok())
    else {
        return false;
    };
    drop(storage);
    state.registry.get(&account.harness).is_some_and(|adapter| {
        matches!(
            adapter.login_state(&account.state_dir),
            LoginState::Expired | LoginState::Partial | LoginState::SsoPending
        )
    })
}

/// After `session` ended (its status already stored): starts it again on
/// a thread of its own when [`decide`] says so; when it has crashed too
/// often, says why on the session. Never blocks the caller.
pub fn after_exit(state: &Arc<AppState>, session: &SessionId, ending: Ending) {
    if ending != Ending::Crashed {
        return;
    }
    let ending = if cannot_sign_in(state, session) {
        Ending::SignInFailed
    } else {
        ending
    };
    let enabled = state
        .storage
        .lock()
        .is_ok_and(|storage| enabled_for(&storage, session));
    let earlier = state
        .restarts
        .lock()
        .map(|restarts| restarts.earlier(session.as_str()))
        .unwrap_or_default();
    match decide(ending, enabled, &earlier, crate::commands::now_millis()) {
        Verdict::Restart(attempt) => {
            let state = Arc::clone(state);
            let session = session.clone();
            std::thread::spawn(move || {
                std::thread::sleep(BACKOFF);
                restart(&state, &session, attempt);
            });
        }
        Verdict::GaveUp => say_why(state, session),
        Verdict::Never(_) | Verdict::Off => {}
    }
}

fn tries_words(attempt: usize) -> String {
    format!("try {attempt} of {TRIES}")
}

/// Starts `crashed` again and records the try on both sessions.
fn restart(state: &Arc<AppState>, crashed: &SessionId, attempt: usize) {
    let outcome = start_again(state, crashed);
    // The try is counted before it's noted: whoever reads the note (the
    // next crash's verdict, a test) finds it counted.
    if let Ok((replacement, _)) = &outcome
        && let Ok(mut restarts) = state.restarts.lock()
    {
        restarts.record(
            crashed.as_str(),
            replacement.as_str(),
            crate::commands::now_millis(),
        );
    }
    let Ok(storage) = state.storage.lock() else {
        return;
    };
    match outcome {
        Ok((replacement, resumed)) => {
            let how = if resumed {
                "its conversation resumed"
            } else {
                "started fresh: its conversation wasn't found"
            };
            let _ = storage.sessions().record_lifecycle(
                crashed,
                &format!("restarted after a crash ({}, {how})", tries_words(attempt)),
            );
            let _ = storage.sessions().record_lifecycle(
                &replacement,
                &format!(
                    "started again after {crashed} crashed ({})",
                    tries_words(attempt)
                ),
            );
        }
        Err(error) => {
            let _ = storage.sessions().record_lifecycle(
                crashed,
                &format!(
                    "couldn't restart after a crash: {}",
                    error.wire().user_message
                ),
            );
        }
    }
}

/// The crashed session again, on its own launch settings and folder: its
/// conversation resumed when one is known, else started fresh. Returns
/// the new session and whether it resumed.
fn start_again(state: &Arc<AppState>, crashed: &SessionId) -> AppResult<(SessionId, bool)> {
    let (record, account) = crate::commands::handover_source_linked(state, crashed.as_str())?;
    let reference = record
        .transcript_ref
        .clone()
        .filter(|reference| reference.native_id.is_some());
    let resumable = reference.zip(record.project.clone());
    if let Some((reference, project_id)) = resumable {
        let (profile, project) = {
            let storage = crate::commands::lock_storage_arc(state)?;
            (
                storage.profiles().get(&record.profile)?,
                storage.projects().get(&project_id)?,
            )
        };
        let native = reference.native_id.clone().unwrap_or_default();
        let (session, _) = crate::crossmachine::launch_here(
            state,
            &profile,
            &account,
            &project,
            &juggler_adapter::LaunchMode::Resume {
                session_id: native,
                delta_prompt: None,
            },
            Some(reference),
            "restart",
        )?;
        return Ok((session, true));
    }
    let cwd = record.cwd.to_str().map(str::to_owned).ok_or_else(|| {
        AppError::Refused("its folder's name can't be passed to the tool".to_owned())
    })?;
    let launched = crate::commands::launch_profile(state, record.profile.as_str(), Some(cwd))?;
    Ok((SessionId::new(launched.session_id), false))
}

/// It crashed again after [`TRIES`] tries in [`WINDOW_MS`]: it stays
/// ended, and says so on its row and in its history.
fn say_why(state: &Arc<AppState>, session: &SessionId) {
    let Ok(storage) = state.storage.lock() else {
        return;
    };
    let words = format!("crashed {TRIES} times in 10 minutes, so it wasn't started again");
    let _ = storage.sessions().record_lifecycle(session, &words);
    if let Ok(mut record) = storage.sessions().get(session)
        && record.status == SessionStatus::Failed
    {
        record.attention = Some(AttentionReason {
            rule: "restart_gave_up".to_owned(),
            inputs: vec![words],
            since_ms: crate::commands::now_millis(),
        });
        let _ = storage.sessions().update(&record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;
    use juggler_supervisor::ExitKind;

    const CRASH: Ending = Ending::Crashed;

    #[test]
    fn never_after_a_finish_a_stop_a_limit_or_a_failed_sign_in() {
        for (ending, why) in [
            (
                Ending::of(&ExitKind::Code(0), false, false, ""),
                "it finished",
            ),
            (
                Ending::of(&ExitKind::Code(1), true, false, ""),
                "you stopped it",
            ),
            (
                Ending::of(&ExitKind::Code(1), false, true, ""),
                "it reached a usage limit",
            ),
            (
                Ending::of(
                    &ExitKind::Code(1),
                    false,
                    false,
                    "Error: OAuth token has expired",
                ),
                "it couldn't sign in",
            ),
            // A stop the user made wins over what the output said.
            (
                Ending::of(&ExitKind::Code(1), true, true, "Invalid API key"),
                "you stopped it",
            ),
        ] {
            assert_eq!(decide(ending, true, &[], NOW), Verdict::Never(why));
        }
    }

    #[test]
    fn any_other_failure_is_a_crash() {
        assert_eq!(
            Ending::of(&ExitKind::Code(134), false, false, "panicked at"),
            CRASH
        );
        assert_eq!(
            Ending::of(&ExitKind::Signaled(Some(11)), false, false, ""),
            CRASH
        );
    }

    #[test]
    fn a_crash_restarts_only_when_it_is_on() {
        assert_eq!(decide(CRASH, false, &[], NOW), Verdict::Off);
        assert_eq!(decide(CRASH, true, &[], NOW), Verdict::Restart(1));
    }

    #[test]
    fn three_tries_in_ten_minutes_then_it_stays_ended() {
        let minute = 60_000;
        assert_eq!(
            decide(CRASH, true, &[NOW - minute], NOW),
            Verdict::Restart(2)
        );
        assert_eq!(
            decide(CRASH, true, &[NOW - 2 * minute, NOW - minute], NOW),
            Verdict::Restart(3)
        );
        let three = [NOW - 9 * minute, NOW - 5 * minute, NOW - minute];
        assert_eq!(decide(CRASH, true, &three, NOW), Verdict::GaveUp);
        // Tries older than ten minutes no longer count.
        let old = [NOW - 30 * minute, NOW - 20 * minute, NOW - 11 * minute];
        assert_eq!(decide(CRASH, true, &old, NOW), Verdict::Restart(1));
    }

    #[test]
    fn a_restarted_session_counts_with_the_ones_it_replaced() {
        let mut restarts = Restarts::default();
        restarts.record("a", "b", NOW - 2);
        restarts.record("b", "c", NOW - 1);
        assert_eq!(restarts.earlier("c"), vec![NOW - 2, NOW - 1]);
        assert_eq!(restarts.earlier("a"), vec![NOW - 2, NOW - 1]);
        assert!(restarts.earlier("other").is_empty());
    }
}
