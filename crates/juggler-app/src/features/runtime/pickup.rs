//! AJ-E2310 — a driven session comes back after AI Juggler restarts or
//! crashes (the 0.1 "always on" rule).
//!
//! A driven session's process is AI Juggler's child on its pipes, so it
//! ends with AI Juggler. While it runs, one small mark per session is kept
//! in AI Juggler's own store ([`KEY`]): the tool's conversation id and
//! whether a turn runs. It holds no message text, and goes when the
//! session ends, is stopped, released or handed over.
//!
//! At the next start ([`after_restart`]), each marked session still live
//! on this Mac:
//! - **between turns** is resumed at once under the same id, in the same
//!   runtime (the tool resumes its own conversation; no turn is sent). That
//!   is coming back as it was (ADR-0003 point 5), never new work;
//! - **mid-turn** starts nothing: it says its turn was cut off, needs the
//!   user, and [`resume_impl`] (their click on Resume) resumes it, sending
//!   nothing either.
//!
//! A resume that can't drive starts the tool's terminal on the same
//! conversation with the reason, as every launch does; one that can't
//! start at all ends with the reason.

use std::collections::BTreeMap;
use std::sync::Arc;

use juggler_adapter::runtime::Runtime;
use juggler_core::model::{AttentionReason, SessionId, SessionStatus, Timestamp};
use juggler_core::storage::Storage;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The store's key for the marks (one JSON object, by session id).
pub const KEY: &str = "runtime.live";
/// The attention rule of a session whose turn was cut off.
pub const CUT_OFF_RULE: &str = "turn_cut_off";
/// What a cut-off session says.
pub const CUT_OFF_WORDS: &str = "Its turn was cut off when AI Juggler stopped";

/// What AI Juggler keeps of a running driven session: enough to resume
/// it, nothing it said.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Mark {
    /// The tool's own conversation id.
    pub native_id: String,
    /// A turn runs (or waits on an answer).
    pub in_turn: bool,
}

/// How a cut-off session's runtime is named until it is resumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutOff {
    /// How it runs, in words.
    pub label: &'static str,
    /// A Beta runtime.
    pub beta: bool,
}

/// Every mark.
#[must_use]
pub fn marks(storage: &Storage) -> BTreeMap<String, Mark> {
    storage
        .app_settings()
        .get(KEY)
        .ok()
        .flatten()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Sets `session`'s mark, or removes it (`None`); writes only a change.
pub fn note(storage: &Storage, session: &SessionId, mark: Option<Mark>) {
    let mut all = marks(storage);
    let changed = match mark {
        Some(mark) => all.insert(session.to_string(), mark.clone()) != Some(mark),
        None => all.remove(session.as_str()).is_some(),
    };
    if !changed {
        return;
    }
    let text = if all.is_empty() {
        String::new()
    } else {
        serde_json::to_string(&all).unwrap_or_default()
    };
    let _ = storage.app_settings().set(KEY, &text);
}

/// The mark a driver's state calls for: its conversation and whether a
/// turn runs, or none once it ended (or before the tool named it).
#[must_use]
pub fn mark_of(native_id: Option<&str>, in_turn: bool, ended: bool) -> Option<Mark> {
    if ended {
        return None;
    }
    native_id.map(|native_id| Mark {
        native_id: native_id.to_owned(),
        in_turn,
    })
}

/// The driver's state changed: its mark follows.
pub(super) fn follow(state: &Arc<AppState>, session: &SessionId, mark: Option<Mark>) {
    if let Ok(storage) = state.storage.lock() {
        note(&storage, session, mark);
    }
}

/// Whether `session` was live on this Mac when AI Juggler stopped (the
/// start-up pass leaves its stored state to [`after_restart`]).
#[must_use]
pub fn is_marked(storage: &Storage, session: &SessionId) -> bool {
    marks(storage).contains_key(session.as_str())
}

/// Once the app is up: each marked session comes back, or says its turn
/// was cut off. Resumes run on their own threads (start-up never waits on
/// a tool).
pub fn after_restart(state: &Arc<AppState>) {
    let (pending, records) = {
        let Ok(storage) = state.storage.lock() else {
            return;
        };
        let pending = marks(&storage);
        let records: BTreeMap<String, _> = pending
            .keys()
            .filter_map(|id| {
                let record = storage.sessions().get(&SessionId::new(id.as_str())).ok()?;
                Some((id.clone(), record))
            })
            .collect();
        (pending, records)
    };
    let machine = crate::commands::local_machine_id(state);
    for (id, mark) in pending {
        let session = SessionId::new(id.as_str());
        let live = records.get(&id).filter(|record| {
            record.machine == machine && !crate::commands::is_terminal(record.status)
        });
        if live.is_none() {
            follow(state, &session, None);
            continue;
        }
        if mark.in_turn {
            cut_off(state, &session);
        } else {
            let state = Arc::clone(state);
            let _ = std::thread::Builder::new()
                .name(format!("pickup-{id}"))
                .spawn(move || {
                    let _ = come_back(
                        &state,
                        &session,
                        &mark.native_id,
                        "came back after AI Juggler restarted",
                    );
                });
        }
    }
}

/// The session's turn was cut off: it needs the user, and says why.
fn cut_off(state: &Arc<AppState>, session: &SessionId) {
    let how = super::commands::owner(state, session.as_str())
        .ok()
        .and_then(|(_, _, account)| state.registry.get(&account.harness))
        .and_then(|adapter| {
            adapter.driven().map(|driven| CutOff {
                label: driven.label(),
                beta: driven.maturity() == juggler_adapter::runtime::Maturity::Beta,
            })
        });
    let Some(how) = how else {
        // Its tool can't be driven any more: it ends, and says why.
        end(
            state,
            session,
            "its turn was cut off when AI Juggler stopped, and its tool can't be resumed here",
        );
        return;
    };
    state.runtime.note_cut_off(session, how);
    if let Ok(storage) = state.storage.lock()
        && let Ok(mut record) = storage.sessions().get(session)
    {
        record.status = SessionStatus::WaitingUser;
        record.attention = Some(AttentionReason {
            rule: CUT_OFF_RULE.to_owned(),
            inputs: vec![CUT_OFF_WORDS.to_owned()],
            since_ms: Timestamp::now().millis(),
        });
        let _ = storage.sessions().update(&record);
        let _ = storage.sessions().record_lifecycle(
            session,
            "AI Juggler stopped during its turn: Resume continues it",
        );
    }
    state.runtime.notify(session.as_str(), true);
}

/// Ends `session` with `why` in its history, and forgets its mark.
fn end(state: &Arc<AppState>, session: &SessionId, why: &str) {
    state.runtime.clear_cut_off(session.as_str());
    if let Ok(storage) = state.storage.lock() {
        note(&storage, session, None);
        if let Ok(mut record) = storage.sessions().get(session) {
            record.status = SessionStatus::Stopped;
            record.attention = None;
            record.last_activity = Some(Timestamp::now());
            let _ = storage.sessions().update(&record);
        }
        let _ = storage.sessions().record_lifecycle(session, why);
    }
    state.runtime.notify(session.as_str(), true);
}

/// Resumes `native_id` under `session`'s own id, in its runtime (the
/// terminal, with the reason, when it can't be driven). Returns the
/// session that holds the conversation now: the same one, or a new one
/// when a driven start failed after its process ran (that id is spent).
fn come_back(
    state: &Arc<AppState>,
    session: &SessionId,
    native_id: &str,
    how: &str,
) -> AppResult<SessionId> {
    let started = (|| {
        let (record, profile, account) = super::commands::owner(state, session.as_str())?;
        let adapter = state
            .registry
            .get(&account.harness)
            .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
        let terminal = adapter.launch_spec(
            &account,
            &profile,
            &record.cwd,
            &juggler_adapter::LaunchMode::Resume {
                session_id: native_id.to_owned(),
                delta_prompt: None,
            },
        )?;
        // The one path: its own row is starting again before its process
        // does (intent before spawn), then the runtime.
        let started = crate::features::sessions::spawn::start_session(
            state,
            &crate::features::sessions::spawn::NewSession {
                id: session,
                account: &account,
                profile: &profile.id,
                project: record.project.clone(),
                cwd: record.cwd.clone(),
                conversation: Some(juggler_core::model::ConversationId::new(native_id)),
                transcript_ref: record.transcript_ref.clone(),
                origin: "resume",
            },
            crate::features::sessions::spawn::Runs::Planned {
                profile: &profile,
                resume: Some(native_id.to_owned()),
                requested: Some(Runtime::Driven),
                terminal,
            },
        )?;
        if started.session != *session {
            super::commands::record_continuation(state, &record)?;
        }
        Ok::<_, AppError>(started)
    })();
    let started = match started {
        Ok(started) => started,
        Err(error) => {
            let why = format!("couldn't come back: {}", error.wire().user_message);
            end(state, session, &why);
            return Err(error);
        }
    };
    state.runtime.clear_cut_off(session.as_str());
    if let Ok(storage) = state.storage.lock() {
        if started.session != *session {
            // Its conversation went on under a new id: the old one is done.
            note(&storage, session, None);
        }
        let words = match started.runtime {
            Runtime::Driven => format!("{how}: its conversation resumed"),
            Runtime::Terminal => format!("{how}: its conversation resumed in its terminal"),
        };
        let _ = storage
            .sessions()
            .record_lifecycle(&started.session, &words);
    }
    state.runtime.notify(session.as_str(), true);
    Ok(started.session)
}

/// The user's Resume on a session whose turn was cut off: its
/// conversation is resumed, and nothing is sent.
///
/// # Errors
///
/// [`AppError::Refused`] for a session whose turn wasn't cut off; the
/// launch's own errors.
pub fn resume_impl(
    state: &Arc<AppState>,
    session: &str,
) -> AppResult<super::commands::HandoverInfo> {
    if !state.runtime.is_cut_off(session) {
        return Err(AppError::Refused(
            "only a session whose turn was cut off can be resumed here".into(),
        ));
    }
    let id = SessionId::new(session);
    let native_id = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        marks(&storage)
            .get(session)
            .map(|mark| mark.native_id.clone())
            .ok_or_else(|| AppError::Refused("its conversation isn't known".into()))?
    };
    let now = come_back(state, &id, &native_id, "resumed after its turn was cut off")?;
    Ok(super::commands::HandoverInfo {
        session_id: now.to_string(),
    })
}

/// The user's End on a session whose turn was cut off: it ends, and its
/// conversation stays in the tool's history.
///
/// # Errors
///
/// [`AppError::Refused`] for a session whose turn wasn't cut off.
pub fn end_impl(state: &Arc<AppState>, session: &str) -> AppResult<()> {
    if !state.runtime.is_cut_off(session) {
        return Err(AppError::Refused(
            "only a session whose turn was cut off can be ended here".into(),
        ));
    }
    end(
        state,
        &SessionId::new(session),
        "ended after its turn was cut off",
    );
    Ok(())
}

/// Resumes a session whose turn was cut off (the user's click).
///
/// # Errors
///
/// As [`resume_impl`].
#[tauri::command]
pub async fn runtime_resume(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<super::commands::HandoverInfo> {
    crate::ipc::blocking(state.inner(), "the resume", move |state| {
        resume_impl(state, &session_id)
    })
    .await
}

/// Ends a session whose turn was cut off (the user's click).
///
/// # Errors
///
/// As [`end_impl`].
#[tauri::command]
pub async fn runtime_end(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "ending it", move |state| {
        end_impl(state, &session_id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_holds_the_conversation_and_the_turn_and_nothing_said() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("s.sqlite")).unwrap();
        let session = SessionId::new("sess-1");
        assert!(marks(&storage).is_empty());
        note(&storage, &session, mark_of(Some("thread-1"), false, false));
        note(
            &storage,
            &SessionId::new("sess-2"),
            mark_of(Some("thread-2"), true, false),
        );
        let text = storage.app_settings().get(KEY).unwrap().unwrap();
        assert_eq!(
            text,
            r#"{"sess-1":{"native_id":"thread-1","in_turn":false},"sess-2":{"native_id":"thread-2","in_turn":true}}"#
        );
        // Ended, or not named yet: no mark.
        assert_eq!(mark_of(Some("thread-1"), true, true), None);
        assert_eq!(mark_of(None, true, false), None);
        note(&storage, &session, None);
        note(&storage, &SessionId::new("sess-2"), None);
        assert_eq!(
            storage.app_settings().get(KEY).unwrap(),
            None,
            "the key goes with the last"
        );
    }
}
