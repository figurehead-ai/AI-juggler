//! AJ-E206 §8.2: undoing a completed switch, and the redaction decision it
//! records.

use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::LaunchMode;
use juggler_core::model::{SessionId, Timestamp};

use crate::commands::{lock_storage_arc, not_found_or_core};
use crate::error::{AppError, AppResult};
use crate::features::launch::dto::LaunchResult;
use crate::features::redaction::{redaction_global_of, redaction_override_of};
use crate::ids::unique_suffix;
use crate::state::AppState;

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn switch_undo_impl(state: &Arc<AppState>, edge_id: &str) -> AppResult<LaunchResult> {
    let context = undo_edge_context(state, edge_id)?;
    stop_continuation(state, context.target_session.clone())?;
    let resumed = undo_resume_source(state, &context)?;
    crate::usage_stats::switch_undone(&context.edge);
    Ok(resumed)
}

/// The read-side of a switch to undo (AJ-E206).
struct UndoContext {
    /// The work item both epochs belong to.
    work: juggler_core::model::WorkItemId,
    /// The source epoch (what undo resumes).
    from_epoch: juggler_core::model::Epoch,
    /// The confirmed switch's to-epoch.
    to_epoch: juggler_core::model::EpochId,
    /// The continuation instance (what undo stops), if any.
    target_session: Option<SessionId>,
    /// The original edge (the compensating edge's parent).
    edge: juggler_core::model::SwitchEdge,
    /// The source's native session id (the resume target).
    native_id: String,
}

/// Loads and validates the edge to undo: confirmed, both epochs
/// present, source session + native id resolvable.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the edge cannot be compensated.
fn undo_edge_context(state: &Arc<AppState>, edge_id: &str) -> AppResult<UndoContext> {
    use juggler_core::model::{EdgeState, EpochId, SwitchEdgeId};
    let undo_of = SwitchEdgeId::new(edge_id.to_owned());
    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let edge = graph.edge(&undo_of).map_err(|_| AppError::Io {
        context: "undo switch",
        message: "That switch isn't recorded any more, so there's nothing to undo".to_owned(),
    })?;
    let not_landed = || AppError::Io {
        context: "undo switch",
        message: "Only a switch that finished can be undone, and this one didn't".to_owned(),
    };
    if !matches!(edge.state, EdgeState::Confirmed) {
        return Err(not_landed());
    }
    let Some(to_epoch_id) = edge.to_epoch.clone() else {
        return Err(not_landed());
    };
    let epochs = graph.epochs_for_work(&edge.work)?;
    let find = |id: &EpochId| epochs.iter().find(|epoch| &epoch.id == id);
    let from_epoch = find(&edge.from_epoch)
        .ok_or_else(|| AppError::Io {
            context: "undo switch",
            message:
                "AI Juggler can't find where this work was before the switch, so it can't go back"
                    .to_owned(),
        })?
        .clone();
    let target_session = find(&to_epoch_id).and_then(|epoch| epoch.session_ref.clone());
    if from_epoch.session_ref.is_none() {
        // AJ-E2108: a switch from a session another app runs left that
        // session as it was: there is nothing here to go back to.
        let message = if from_epoch.native_id.is_some() {
            "The original is a session another app runs, and it's still there, untouched: \
             stop the new session from its page if you don't want it"
        } else {
            "The switch's source session is missing — resume it from Sessions"
        };
        return Err(AppError::Io {
            context: "undo switch",
            message: message.to_owned(),
        });
    }
    let Some(native_id) = from_epoch.native_id.clone().filter(|id| !id.is_empty()) else {
        return Err(AppError::Io {
            context: "undo switch",
            message: "The original has no conversation AI Juggler can resume".to_owned(),
        });
    };
    Ok(UndoContext {
        work: edge.work.clone(),
        from_epoch,
        to_epoch: to_epoch_id,
        target_session,
        edge,
        native_id,
    })
}

/// Stops the continuation instance. Already-gone is fine (the stop
/// goal already holds); a REAL stop failure aborts before the source
/// resumes — never two live instances on one work item.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the stop ladder fails.
fn stop_continuation(state: &Arc<AppState>, target: Option<SessionId>) -> AppResult<()> {
    let Some(target_session) = target else {
        return Ok(());
    };
    let stopped = state
        .supervisor
        .stop(&target_session, Duration::from_secs(5));
    match stopped {
        Ok(()) | Err(juggler_supervisor::SupervisorError::UnknownSession(_)) => Ok(()),
        Err(error) => {
            crate::error_log::record("undo_switch", &error.to_string());
            Err(AppError::Io {
                context: "undo switch",
                message: "AI Juggler couldn't stop the new session, so nothing was undone. Stop it from its page and try again".to_owned(),
            })
        }
    }
}

/// Resumes the source: its account, its recipe, its own native id —
/// then records the compensating edge (`parent_edge_a` = the original;
/// to-epoch on the source account; launched+confirmed in one motion,
/// the supervisor accepting the spawn being the proof we can record
/// today).
///
/// # Errors
///
/// Returns the typed [`AppError`] when resolution, spawn, or graph
/// recording fails.
fn undo_resume_source(state: &Arc<AppState>, context: &UndoContext) -> AppResult<LaunchResult> {
    let UndoContext {
        from_epoch,
        native_id,
        ..
    } = context;
    let Some(source_session_ref) = from_epoch.session_ref.clone() else {
        return Err(AppError::Io {
            context: "undo switch",
            message: "The switch's source session is missing — resume it from Sessions".to_owned(),
        });
    };

    let (source, account_a, profile_a) = {
        let storage = lock_storage_arc(state)?;
        let source = storage
            .sessions()
            .get(&source_session_ref)
            .map_err(|error| {
                not_found_or_core(error, "session", &source_session_ref.to_string())
            })?;
        let account_a = storage
            .accounts()
            .get(&from_epoch.account)
            .map_err(|error| {
                not_found_or_core(error, "account", &from_epoch.account.to_string())
            })?;
        let profile_a = storage
            .profiles()
            .get(&source.profile)
            .map_err(|error| not_found_or_core(error, "profile", &source.profile.to_string()))?;
        (source, account_a, profile_a)
    };
    let adapter = state
        .registry
        .get(&account_a.harness)
        .ok_or_else(|| AppError::UnknownHarness(account_a.harness.to_string()))?;
    let new_session = SessionId::new(format!("sess-{}", unique_suffix()));
    let spec = adapter.launch_spec(
        &account_a,
        &profile_a,
        &source.cwd,
        &LaunchMode::Resume {
            session_id: native_id.clone(),
            delta_prompt: None,
        },
    )?;
    // The one path: the row first (ADR-0010 §1), then the bridge and the
    // account's key in the launch, under the supervisor.
    let started = crate::features::sessions::spawn::start_session(
        state,
        &crate::features::sessions::spawn::NewSession {
            id: &new_session,
            account: &account_a,
            profile: &profile_a.id,
            project: source.project.clone(),
            cwd: source.cwd.clone(),
            conversation: Some(juggler_core::model::ConversationId::new(
                native_id.to_owned(),
            )),
            transcript_ref: source.transcript_ref.clone(),
            origin: "relaunch",
        },
        crate::features::sessions::spawn::Runs::Terminal(spec),
    )?;

    record_compensating_edge(state, context, &new_session)?;
    // AJ-E1904: the task's history says the work came back.
    crate::features::tasks::switched::after_switch(
        state,
        &new_session,
        &context.edge.to_account,
        &context.edge.from_account,
    );

    Ok(LaunchResult {
        session_id: new_session.to_string(),
        pid: started.spawned.pid,
    })
}

/// Records the undo: the resumed source instance's epoch, and
/// the compensating edge (`parent_edge_a` = the original; launched +
/// confirmed in one motion — the supervisor accepting the spawn is
/// the proof we can record today).
///
/// # Errors
///
/// Returns the typed [`AppError`] when storage or graph writes fail.
fn record_compensating_edge(
    state: &Arc<AppState>,
    context: &UndoContext,
    new_session: &SessionId,
) -> AppResult<()> {
    use juggler_core::model::{Epoch, EpochId, SwitchEdge, SwitchEdgeId, SwitchMechanism};
    let UndoContext {
        work,
        to_epoch,
        edge,
        native_id,
        ..
    } = context;
    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let epoch_undo = EpochId::new(format!("ep-{}", unique_suffix()));
    graph.insert_epoch(&Epoch {
        id: epoch_undo.clone(),
        work: work.clone(),
        session_ref: Some(new_session.clone()),
        native_id: Some(native_id.to_owned()),
        account: edge.from_account.clone(),
        provider: edge.from_provider.clone(),
        resumed_from_epoch: Some(to_epoch.clone()),
        machine: None,
    })?;
    let undo_edge_id = SwitchEdgeId::new(format!("edge-{}", unique_suffix()));
    graph.append_edge(&SwitchEdge {
        id: undo_edge_id.clone(),
        work: work.clone(),
        from_epoch: to_epoch.clone(),
        // Filled by mark_launched below (S-G1: append never sets it).
        to_epoch: None,
        from_account: edge.to_account.clone(),
        to_account: edge.from_account.clone(),
        from_provider: edge.to_provider.clone(),
        to_provider: edge.from_provider.clone(),
        // Same account, same native file: T1 native resume.
        mechanism: SwitchMechanism::NativeResume,
        pack_id: None,
        fidelity_shown: Some("F4".to_owned()),
        manifest_sha256: None,
        state: juggler_core::model::EdgeState::Staged,
        actor: juggler_core::model::EdgeActor::User,
        parent_edge_a: Some(edge.id.clone()),
        parent_edge_b: None,
        created_at: Timestamp::now(),
        to_machine: None,
    })?;
    graph.mark_launched(&undo_edge_id, &epoch_undo)?;
    graph.mark_confirmed(&undo_edge_id)?;
    Ok(())
}

/// AJ-E208: records the switch's redaction decision on the edge —
/// the effective policy, the per-switch choice (Ask), and how many
/// items the user un-redacted for THIS switch only (never a
/// remembered-policy change). A storage failure never fails the
/// switch (the decision is diagnostics; the switch already landed).
pub(crate) fn record_redaction_decision(
    state: &Arc<AppState>,
    edge_id: &juggler_core::model::SwitchEdgeId,
    provider: &juggler_core::model::HarnessId,
    redaction_choice: Option<&str>,
    redaction_unredact: &[u32],
) {
    let Ok(storage) = lock_storage_arc(state) else {
        return;
    };
    let policy = {
        let global = redaction_global_of(&storage);
        let r#override = redaction_override_of(&storage, provider.as_str());
        juggler_handover::redaction::resolve_policy(global, r#override)
    };
    let chosen = redaction_choice.and_then(juggler_handover::redaction::RedactionPolicy::from_wire);
    if redaction_choice.is_some() {
        // AJ-E1103: the user reviewed redaction on this switch.
        let changed = !redaction_unredact.is_empty() || (chosen.is_some() && chosen != policy);
        crate::usage_stats::count(
            "pack.redaction_reviewed",
            &[("outcome", if changed { "changed" } else { "kept" })],
        );
    }
    let applied = chosen
        .or(policy)
        .map_or_else(|| "unset".to_owned(), |parsed| parsed.as_str().to_owned());
    let rows = vec![
        juggler_core::model::ManifestEntry {
            rel_path: "redaction/policy".to_owned(),
            sha256: String::new(),
            bytes: 0,
            kind: "redaction".to_owned(),
            disposition: applied,
        },
        juggler_core::model::ManifestEntry {
            rel_path: "redaction/unredacted".to_owned(),
            sha256: String::new(),
            bytes: u64::try_from(redaction_unredact.len()).unwrap_or_default(),
            kind: "redaction-unredact".to_owned(),
            disposition: "this-switch-only".to_owned(),
        },
    ];
    let _ = storage.switch_graph().append_manifest(edge_id, &rows);
}
