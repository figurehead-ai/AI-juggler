//! The juggle (spec §7 T1/T2): which session a handover continues from, and
//! moving it to another account.

use std::path::PathBuf;
use std::sync::Arc;

use juggler_core::model::{Account, MachineId, SessionId, SessionStatus};

use crate::commands::{local_machine_id, lock_storage_arc, not_found_or_core};
use crate::error::AppResult;
use crate::features::launch::dto::LaunchResult;
use crate::features::switch::back::is_replay;
use crate::features::switch::execute::switch_execute_impl;
use crate::features::switch::replay::replay_header;
use crate::state::AppState;

/// Core implementation shared with tests.
///
/// # Errors
///
/// The hand-over source row + its owning account (the read-side
/// preamble of [`session_handover_impl`]).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the source session or
/// its account cannot be resolved.
pub(crate) fn handover_source(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<(juggler_core::model::InstanceRecord, Account)> {
    resolve_source(state, session_id, false)
}

/// [`handover_source`] that KEEPS the transcript link it finds — only
/// the switch itself does this (a preflight or a ranking look never
/// writes a guess down).
///
/// # Errors
///
/// As [`handover_source`].
pub(crate) fn handover_source_linked(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<(juggler_core::model::InstanceRecord, Account)> {
    resolve_source(state, session_id, true)
}

/// Whether a session's transcript link stands: it has one, and it is
/// another Mac's or its file is here.
fn link_stands(source: &juggler_core::model::InstanceRecord, here: &MachineId) -> bool {
    source
        .transcript_ref
        .as_ref()
        .is_some_and(|reference| source.machine != *here || reference.path.exists())
}

/// AJ-E1303: a session the app launched has no transcript link yet; the
/// Provider finds the file it created (same folder, created after the
/// launch). Never a guess: files linked to other sessions are excluded,
/// two matches find nothing, and another live, unlinked session of the
/// same account in the same folder makes it ambiguous (its file could
/// be the only one so far) — the switch then asks. The filesystem scan
/// runs without the storage lock. A link of this Mac's session whose file
/// is gone (0.2 lab: a pool card stayed "Unavailable") is looked for again
/// the same way, and replaced only when the file is found.
fn resolve_source(
    state: &Arc<AppState>,
    session_id: &str,
    persist: bool,
) -> AppResult<(juggler_core::model::InstanceRecord, Account)> {
    // AJ-E2108: a session another app runs is read from the index, by its
    // key; it has no row (and its file is known, so nothing is guessed).
    match crate::features::conversations::switch_source::parse_source_key(session_id) {
        Some((account, conversation)) => {
            crate::features::conversations::switch_source::as_switch_source(
                state,
                account,
                conversation,
                persist,
            )
        }
        None => resolve_row_source(state, session_id, persist),
    }
}

/// [`resolve_source`] for a session AI Juggler runs: its row, and the
/// transcript link found for it.
fn resolve_row_source(
    state: &Arc<AppState>,
    session_id: &str,
    persist: bool,
) -> AppResult<(juggler_core::model::InstanceRecord, Account)> {
    let here = local_machine_id(state);
    let stands = |source: &juggler_core::model::InstanceRecord| link_stands(source, &here);
    let (mut source, account_a, taken, ambiguous, replayed_into) = {
        let storage = lock_storage_arc(state)?;
        let source = storage
            .sessions()
            .get(&SessionId::new(session_id))
            .map_err(|error| not_found_or_core(error, "session", session_id))?;
        let source_profile = storage
            .profiles()
            .get(&source.profile)
            .map_err(|error| not_found_or_core(error, "profile", &source.profile.to_string()))?;
        let account_a = storage
            .accounts()
            .get(&source_profile.account)
            .map_err(|error| {
                not_found_or_core(error, "account", &source_profile.account.to_string())
            })?;
        if stands(&source) {
            return Ok((source, account_a));
        }
        let others: Vec<juggler_core::model::InstanceRecord> = storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter(|other| other.id != source.id)
            .collect();
        let account_recipes: std::collections::HashSet<String> = storage
            .profiles()
            .list_by_account(&account_a.id)?
            .into_iter()
            .map(|recipe| recipe.id.to_string())
            .collect();
        let taken: Vec<PathBuf> = others
            .iter()
            .filter_map(|other| other.transcript_ref.as_ref().map(|r| r.path.clone()))
            .collect();
        let ambiguous = others.iter().any(|other| {
            account_recipes.contains(other.profile.as_str())
                && other.transcript_ref.is_none()
                && other.cwd == source.cwd
                && !ended_before(other, source.started_at.millis())
        });
        // A replay started this session: its first message is known.
        let replayed_into = storage
            .switch_graph()
            .edges_for_session(source.id.as_str())
            .unwrap_or_default()
            .iter()
            .any(|(edge, outgoing)| !outgoing && is_replay(edge));
        (source, account_a, taken, ambiguous, replayed_into)
    };
    // AJ-E1540: a session that knows its conversation (picked up with
    // "Continue in AI Juggler") is found by that id, not by when its file
    // appeared: its file is older than the session record.
    let by_id = source.conversation.as_ref().and_then(|conversation| {
        crate::features::conversations::transcript_of(
            state,
            &account_a,
            conversation.as_str(),
            &taken,
        )
    });
    let by_time = || {
        if ambiguous {
            return None;
        }
        state.registry.get(&account_a.harness).and_then(|adapter| {
            adapter.locate_transcript(
                &account_a.state_dir,
                &source.cwd,
                source.started_at.millis(),
                &taken,
            )
        })
    };
    // AJ-E1541 (lab 8): when time alone can't tell (another session of the
    // tool began in the same folder), a replay's own first message can.
    let by_prompt = || {
        replayed_into
            .then(|| {
                crate::features::conversations::primed_conversation(
                    state,
                    &account_a,
                    &source.cwd,
                    source.started_at.millis().saturating_sub(2_000),
                    &replay_header(state, &account_a.harness),
                    &taken,
                )
            })
            .flatten()
    };
    let Some(found) = by_id.or_else(by_time).or_else(by_prompt) else {
        return Ok((source, account_a));
    };
    source.transcript_ref = Some(found);
    if persist {
        // Re-read under the lock: the watcher may have changed the
        // record (status, last activity) during the scan — only the link
        // is written, and only if nothing linked it meanwhile (a link
        // whose file is gone counts as none).
        let storage = lock_storage_arc(state)?;
        let mut fresh = storage.sessions().get(&source.id)?;
        if !stands(&fresh) {
            fresh.transcript_ref.clone_from(&source.transcript_ref);
            storage.sessions().update(&fresh)?;
        }
        source = fresh;
    }
    Ok((source, account_a))
}

/// Whether `session` had ended before `at_ms` (its last activity — a
/// session that ended earlier could not have written a file after it).
fn ended_before(session: &juggler_core::model::InstanceRecord, at_ms: i64) -> bool {
    let ended = matches!(
        session.status,
        SessionStatus::Finished
            | SessionStatus::Stopped
            | SessionStatus::Failed
            | SessionStatus::HandedOver
    );
    ended
        && session
            .last_activity
            .is_some_and(|last| last.millis() < at_ms.saturating_sub(2_000))
}

/// Resumes the session under the target account (see the module docs).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message when the source session, its
/// transcript, or the target profile cannot be resolved, or on copy or
/// launch failure.
pub fn session_handover_impl(
    state: &Arc<AppState>,
    session_id: &str,
    target_profile_id: &str,
) -> AppResult<LaunchResult> {
    // AJ-E205: the pipeline owns execution (freeze → stage → verify →
    // launch → record); the default freeze choice waits for the turn
    // boundary. The UI calls `switch_execute` with the user's choice.
    switch_execute_impl(
        state,
        session_id,
        target_profile_id,
        "finish-turn",
        None,
        &[],
    )
    .map(|switched| LaunchResult {
        session_id: switched.session_id,
        pid: switched.pid,
    })
}
