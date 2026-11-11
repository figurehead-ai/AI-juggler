//! The Switch Graph's edges: staged, failed and succeeded, and the supervisor
//! launch hook.

use std::sync::Arc;

use juggler_adapter::LaunchMode;
use juggler_core::model::{Account, SessionId, SessionStatus, Timestamp};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::ids::unique_suffix;
use crate::state::AppState;

/// The supervisor launch hook: builds the resume spec for the
/// INSTALLED native id (a collision fork changes it) and spawns under
/// the app's supervisor — the only spawn path (suggest-only law). The
/// continuation's row is written first (intent before spawn, ADR-0010).
pub(crate) fn make_launch_hook<'a>(
    state: &'a Arc<AppState>,
    adapter: juggler_adapter::SharedAdapter,
    account_b: &'a juggler_core::model::Account,
    profile_b: &'a juggler_core::model::LaunchRecipe,
    source: &'a juggler_core::model::InstanceRecord,
    new_session: &'a SessionId,
    desktop: Option<&'a str>,
) -> impl FnMut(&juggler_adapter::handover::InstallOutcome) -> Result<u32, String> + 'a {
    move |install: &juggler_adapter::handover::InstallOutcome| -> Result<u32, String> {
        let native_id = install.reference.native_id.clone().unwrap_or_default();
        // 0.3.5 Desktop: the profile takes it from its folder, where the
        // install put it; "switched" once it recorded it.
        if let Some(place) = desktop {
            return crate::features::places::handoff::hand_off(
                state,
                place,
                &native_id,
                &crate::features::places::handoff::Real,
                crate::features::places::handoff::Waits::default(),
            );
        }
        let spec = crate::features::launch::resume_gate::launch_spec(
            adapter.as_ref(),
            account_b,
            profile_b,
            &source.cwd,
            &LaunchMode::Resume {
                session_id: native_id.clone(),
                delta_prompt: None,
            },
        )
        .map_err(|error| error.to_string())?;
        let started = crate::features::sessions::spawn::start_session(
            state,
            &crate::features::sessions::spawn::NewSession {
                id: new_session,
                account: account_b,
                profile: &profile_b.id,
                project: source.project.clone(),
                cwd: source.cwd.clone(),
                conversation: Some(juggler_core::model::ConversationId::new(native_id.clone())),
                transcript_ref: Some(juggler_core::model::TranscriptRef {
                    path: install.reference.path.clone(),
                    native_id: Some(native_id),
                }),
                origin: "switch",
            },
            crate::features::sessions::spawn::Runs::Terminal(spec),
        )
        .map_err(|error| error.to_string())?;
        Ok(started.spawned.pid)
    }
}

/// AJ-E2108: the source's own session row, when it has one. A session
/// another app runs has none: the graph keeps it by its account and the
/// tool's id for it, and nothing marks it handed over (it isn't AI
/// Juggler's to mark).
pub(crate) fn source_row(source: &juggler_core::model::InstanceRecord) -> Option<&SessionId> {
    (!crate::features::conversations::switch_source::is_source_key(source.id.as_str()))
        .then_some(&source.id)
}

/// Whether `epoch` is `source`'s: by its row, or for a session another
/// app runs by its account and conversation, so every switch from it
/// starts from the one epoch.
fn is_source_epoch(
    epoch: &juggler_core::model::Epoch,
    source: &juggler_core::model::InstanceRecord,
    reference: &juggler_core::model::TranscriptRef,
    account: &Account,
) -> bool {
    match source_row(source) {
        Some(row) => epoch.session_ref.as_ref() == Some(row),
        None => {
            epoch.session_ref.is_none()
                && epoch.account == account.id
                && epoch.native_id.is_some()
                && epoch.native_id == reference.native_id
        }
    }
}

/// Records the pre-execution graph state: the work item (created on
/// first switch), the SOURCE epoch, and the Staged edge. Returns
/// `(work, from_epoch, edge)` ids.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub(crate) fn record_edge_staged(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    reference: &juggler_core::model::TranscriptRef,
    account_a: &juggler_core::model::Account,
    account_b: &juggler_core::model::Account,
    mechanism: juggler_core::model::SwitchMechanism,
) -> AppResult<(
    juggler_core::model::WorkItemId,
    juggler_core::model::EpochId,
    juggler_core::model::SwitchEdgeId,
)> {
    use juggler_core::model::{Epoch, EpochId, SwitchEdge, SwitchEdgeId};

    let storage = lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let Some(project) = source.project.as_ref() else {
        return Err(AppError::Io {
            context: "hand over",
            message: "AI Juggler doesn't know which project this session belongs to, so it can't switch it".into(),
        });
    };
    // AJ-E1541: a session that itself came from a switch already has an
    // epoch (the one that switch created for it). This switch starts from
    // that epoch, in its work, so the chain A → B → Codex stays one path
    // and switching back can walk all of it. A new epoch only for a session
    // no switch has touched.
    let works = graph.works_for_project(project)?;
    let mut existing = None;
    for work in works.iter().rev() {
        if let Some(epoch) = graph
            .epochs_for_work(&work.id)?
            .into_iter()
            .rev()
            .find(|epoch| is_source_epoch(epoch, source, reference, account_a))
        {
            existing = Some((work.clone(), epoch));
            break;
        }
    }
    let (work, from) = if let Some(found) = existing {
        found
    } else {
        // AJ-E1702: a session's first switch starts its own Work item, so
        // what hangs off it (a pool) follows this line of work only.
        let work = graph.create_work(project, "Work")?;
        let from = Epoch {
            id: EpochId::new(format!("ep-{}", unique_suffix())),
            work: work.id.clone(),
            session_ref: source_row(source).cloned(),
            native_id: reference.native_id.clone(),
            account: account_a.id.clone(),
            provider: account_a.harness.clone(),
            resumed_from_epoch: None,
            machine: None,
        };
        graph.insert_epoch(&from)?;
        (work, from)
    };
    let edge_id = SwitchEdgeId::new(format!("edge-{}", unique_suffix()));
    graph.append_edge(&SwitchEdge {
        id: edge_id.clone(),
        work: work.id.clone(),
        from_epoch: from.id.clone(),
        to_epoch: None,
        from_account: account_a.id.clone(),
        to_account: account_b.id.clone(),
        from_provider: account_a.harness.clone(),
        to_provider: account_b.harness.clone(),
        mechanism,
        pack_id: None,
        fidelity_shown: Some("F1".to_owned()),
        manifest_sha256: None,
        state: juggler_core::model::EdgeState::Staged,
        actor: juggler_core::model::EdgeActor::User,
        parent_edge_a: None,
        parent_edge_b: None,
        created_at: Timestamp::now(),
        to_machine: None,
    })?;
    Ok((work.id.clone(), from.id.clone(), edge_id))
}

/// Transitions the edge to `Failed{stage, reason}` — the same edge,
/// never a rewrite (S-G1); the source is safe by the pipeline's
/// contract.
/// The edge keeps the failure's stage and its words — the Switch Graph
/// shows the reason as the dialog said it; the detail is in the log.
pub(crate) fn record_edge_failed(
    state: &Arc<AppState>,
    edge: &juggler_core::model::SwitchEdgeId,
    error: &AppError,
) {
    let AppError::Switch { failure, words } = error else {
        return;
    };
    if let Ok(storage) = lock_storage_arc(state) {
        let _ = storage
            .switch_graph()
            .mark_failed(edge, failure.stage(), words);
    }
}

/// The graph ids one executing switch carries through recording.
pub(crate) struct EdgeCtx {
    /// The work item.
    pub(crate) work: juggler_core::model::WorkItemId,
    /// The source epoch.
    pub(crate) from_epoch: juggler_core::model::EpochId,
    /// The staged edge.
    pub(crate) edge: juggler_core::model::SwitchEdgeId,
    /// The continuation Instance id.
    pub(crate) session: SessionId,
}

/// Records the succeeded switch: the source rests (`HandedOver`, when it
/// has a row), the TARGET epoch is created for the continuation Instance
/// (its row was written by the launch hook, before its spawn), the edge
/// launches + confirms (the supervisor accepting the spawn is the
/// confirmation provable today), and the manifest rows land.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub(crate) fn record_edge_succeeded(
    state: &Arc<AppState>,
    source: &juggler_core::model::InstanceRecord,
    record: &juggler_handover::pipeline::SwitchRecord,
    target: &juggler_core::model::Account,
    ctx: &EdgeCtx,
) -> AppResult<()> {
    let native_id = record
        .install
        .reference
        .native_id
        .clone()
        .unwrap_or_default();
    let storage = lock_storage_arc(state)?;
    land(&storage, source, ctx, target, Some(native_id))?;
    let graph = storage.switch_graph();
    // D-76: a compacted source or a left-out start lowers what it reached.
    graph.set_fidelity_reached(&ctx.edge, &record.fidelity_label)?;
    graph.append_manifest(
        &ctx.edge,
        &record
            .source_hashes
            .iter()
            .map(juggler_handover::manifest::ManifestRow::entry)
            .collect::<Vec<_>>(),
    )?;
    Ok(())
}

/// A switch that reached its target: the source rests (`HandedOver`) when
/// it has a row, the target epoch starts on `target` (with the native id
/// when the vendor gave one already), and the edge launches and confirms.
///
/// # Errors
///
/// Storage failures.
pub(crate) fn land(
    storage: &juggler_core::storage::Storage,
    source: &juggler_core::model::InstanceRecord,
    ctx: &EdgeCtx,
    target: &juggler_core::model::Account,
    native_id: Option<String>,
) -> AppResult<()> {
    use juggler_core::model::{Epoch, EpochId};

    if source_row(source).is_some() {
        let mut handed = source.clone();
        handed.status = SessionStatus::HandedOver;
        storage.sessions().update(&handed)?;
    }
    let graph = storage.switch_graph();
    let to_epoch = EpochId::new(format!("ep-{}", unique_suffix()));
    graph.insert_epoch(&Epoch {
        id: to_epoch.clone(),
        work: ctx.work.clone(),
        session_ref: Some(ctx.session.clone()),
        native_id,
        account: target.id.clone(),
        provider: target.harness.clone(),
        resumed_from_epoch: Some(ctx.from_epoch.clone()),
        machine: None,
    })?;
    graph.mark_launched(&ctx.edge, &to_epoch)?;
    graph.mark_confirmed(&ctx.edge)?;
    Ok(())
}
