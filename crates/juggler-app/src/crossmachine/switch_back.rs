//! AJ-E706 Req.4 / AJ-E708 Req.4–5: switch-back across machines.
//!
//! D-05 holds across machines: the work comes back as a RESUME of the
//! original native session on the machine it came from, plus exactly one
//! summarised "While you were away" turn — never a copy-back over the
//! original. The machine holding the away work pushes that one turn
//! (after its user clicked); the original's machine decides.
//!
//! Lineage is never auto-merged (AJ-E708 Req.4): when the original moved
//! on while the work was away, the original's machine refuses with what
//! changed (counts and times, never content) and nothing runs until the
//! user picks — `Keep mine` (resume the original with this turn after its
//! newer turns), `Keep theirs` (leave both as they are; nothing is sent)
//! or `Keep both` (fork the original and resume the fork; the original's
//! own continuation is untouched).

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use juggler_core::model::{
    AccountId, EdgeActor, EdgeState, Epoch, EpochId, FailureStage, HarnessId, MachineId, SessionId,
    SessionStatus, SwitchEdge, SwitchEdgeId, SwitchMechanism, Timestamp, TranscriptRef, WorkItemId,
};
use juggler_link::node::{HandoverAccepted, HandoverRejection, PeerAccess};
use juggler_link::proto;

use super::{FleetSwitchOutcomeInfo, FleetSwitchProgressInfo, refuse};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// A cross-machine switch-back, as the wizard previews it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FleetSwitchBackPlanInfo {
    /// The machine the work came from.
    pub original_machine_id: String,
    /// Its label.
    pub original_machine_label: String,
    /// The original Account there.
    pub original_account_label: String,
    /// The away Account here.
    pub away_account_label: String,
    /// Turns added here since the work arrived.
    pub new_turns: u32,
    /// The one delta turn the original receives (exactly as sent).
    pub delta_preview: String,
    /// Why it can't run now (offline, view only), in plain words.
    pub blockers: Vec<String>,
}

/// Where an away Instance came from: its epoch here, the edge that
/// brought it, and the mirror of the original epoch on the other Mac.
struct AwayLineage {
    work: WorkItemId,
    away_epoch: Epoch,
    push_edge: SwitchEdge,
    original: Epoch,
    original_machine: MachineId,
}

fn away_lineage(state: &Arc<AppState>, session_id: &str) -> AppResult<Option<AwayLineage>> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let session = storage.sessions().get(&SessionId::new(session_id))?;
    let Some(project) = session.project else {
        return Ok(None);
    };
    let graph = storage.switch_graph();
    for work in graph.works_for_project(&project)? {
        let epochs = graph.epochs_for_work(&work.id)?;
        let Some(away_epoch) = epochs.iter().find(|epoch| {
            epoch.machine.is_none()
                && epoch.session_ref.as_ref().map(SessionId::as_str) == Some(session_id)
        }) else {
            continue;
        };
        let edges = graph.edges_for_work(&work.id)?;
        let Some(push_edge) = edges.iter().find(|edge| {
            matches!(edge.state, EdgeState::Confirmed)
                && edge.to_epoch.as_ref() == Some(&away_epoch.id)
        }) else {
            continue;
        };
        let Some(original) = epochs.iter().find(|epoch| epoch.id == push_edge.from_epoch) else {
            continue;
        };
        let Some(machine) = original.machine.clone() else {
            continue; // a local switch — the local switch-back owns it
        };
        return Ok(Some(AwayLineage {
            work: work.id.clone(),
            away_epoch: away_epoch.clone(),
            push_edge: push_edge.clone(),
            original: original.clone(),
            original_machine: machine,
        }));
    }
    Ok(None)
}

/// The away turns since the work arrived, as the one delta turn.
fn delta_of(
    state: &Arc<AppState>,
    session_id: &str,
    lineage: &AwayLineage,
    away_label: &str,
) -> AppResult<(String, u32)> {
    use juggler_adapters::claude_transcript::MessageRole;
    use juggler_handover::pack::{IrRole, IrTurn};
    use juggler_handover::reconcile::ConversationReconciliation;
    use juggler_handover::switch_back::{DeltaInput, delta_turn_text, delta_turns};

    let (reference, base) = {
        let storage = crate::commands::lock_storage_arc(state)?;
        let session = storage.sessions().get(&SessionId::new(session_id))?;
        let base = storage
            .switch_graph()
            .manifest_for(&lineage.push_edge.id)?
            .into_iter()
            .find(|row| row.kind == "transcript");
        (session.transcript_ref, base)
    };
    let bytes = reference
        .as_ref()
        .and_then(|reference| std::fs::read(&reference.path).ok())
        .unwrap_or_default();
    let parsed = juggler_adapters::parse_any(&bytes);
    // The base: what arrived (T2) — or, for a replay, its prime turn.
    let base_uuids: HashSet<String> = match &base {
        Some(row) => {
            let cut = usize::try_from(row.bytes)
                .unwrap_or(usize::MAX)
                .min(bytes.len());
            // Claude lines carry `uuid`; Codex messages get their id
            // from the parser (item id or line) — both, so either
            // Provider's base is complete (AJ-E1303).
            let prefix = &bytes[..cut];
            juggler_adapter::handover::uuid_set(prefix)
                .into_iter()
                .chain(
                    juggler_adapters::parse_any(prefix)
                        .messages
                        .into_iter()
                        .filter_map(|message| message.uuid),
                )
                .collect()
        }
        None => parsed
            .messages
            .iter()
            .find(|message| matches!(message.role, MessageRole::User))
            .and_then(|message| message.uuid.clone())
            .into_iter()
            .collect(),
    };
    let away_turns: Vec<IrTurn> = parsed
        .messages
        .iter()
        .filter(|message| !message.sidechain)
        .map(|message| IrTurn {
            uuid: message.uuid.clone(),
            role: if matches!(message.role, MessageRole::User) {
                IrRole::User
            } else {
                IrRole::Assistant
            },
            text: message.text.clone(),
            at_ms: None,
            tokens: None,
        })
        .collect();
    let away_only: Vec<String> = away_turns
        .iter()
        .filter_map(|turn| turn.uuid.clone())
        .filter(|uuid| !base_uuids.contains(uuid))
        .collect();
    let input = DeltaInput {
        away_account: away_label.to_owned(),
        original_account: lineage.original.account.to_string(),
        away_turns,
        away_only_uuids: away_only.iter().cloned().collect(),
        conversation: ConversationReconciliation {
            base_uuids: base_uuids.len(),
            away_only_uuids: away_only,
            original_only_uuids: 0,
        },
        diff_stat: None,
        todo_note: None,
    };
    let turns = u32::try_from(delta_turns(&input).len()).unwrap_or(u32::MAX);
    Ok((delta_turn_text(&input), turns))
}

/// Core: the cross-machine switch-back plan for an away Instance, or
/// `None` when it didn't arrive from another machine.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn fleet_switch_back_plan_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<Option<FleetSwitchBackPlanInfo>> {
    let Some(lineage) = away_lineage(state, session_id)? else {
        return Ok(None);
    };
    let (_, away_account) = crate::commands::handover_source(state, session_id)?;
    let (delta, new_turns) = delta_of(state, session_id, &lineage, &away_account.label)?;
    let peer = crate::fleet::fleet_peers_impl(state)?
        .into_iter()
        .find(|peer| peer.machine_id == lineage.original_machine.as_str());
    let label = peer.as_ref().map_or_else(
        || lineage.original_machine.to_string(),
        |peer| peer.label.clone(),
    );
    let view = crate::remote::target_views(&state.link.remote)
        .into_iter()
        .find(|(id, _)| id == lineage.original_machine.as_str())
        .map(|(_, view)| view);
    let mut blockers = Vec::new();
    let connected = peer
        .as_ref()
        .is_some_and(|peer| peer.connection == "connected")
        && view.as_ref().is_some_and(|view| view.peer.is_some());
    if !connected {
        blockers.push(format!(
            "{label} is offline — switching back needs a live connection."
        ));
    }
    if view.as_ref().and_then(|view| view.permission.as_deref()) != Some("control") {
        blockers.push(format!(
            "{label} lets this Mac view only — switching back needs View + control (change it on {label})."
        ));
    }
    Ok(Some(FleetSwitchBackPlanInfo {
        original_machine_id: lineage.original_machine.to_string(),
        original_machine_label: label,
        original_account_label: lineage.original.account.to_string(),
        away_account_label: away_account.label,
        new_turns,
        delta_preview: delta,
        blockers,
    }))
}

/// Records the staged switch-back edge here before anything moves.
fn stage_back(
    state: &Arc<AppState>,
    lineage: &AwayLineage,
    away_account: &juggler_core::model::Account,
    edge: &SwitchEdgeId,
    keep_both: bool,
) -> AppResult<()> {
    let storage = crate::commands::lock_storage_arc(state)?;
    storage.switch_graph().append_edge(&SwitchEdge {
        id: edge.clone(),
        work: lineage.work.clone(),
        from_epoch: lineage.away_epoch.id.clone(),
        to_epoch: None,
        from_account: away_account.id.clone(),
        to_account: lineage.original.account.clone(),
        from_provider: away_account.harness.clone(),
        to_provider: lineage.original.provider.clone(),
        mechanism: SwitchMechanism::NativeResume,
        pack_id: None,
        fidelity_shown: Some(if keep_both { "F3" } else { "F4" }.to_owned()),
        manifest_sha256: None,
        state: EdgeState::Staged,
        actor: EdgeActor::User,
        parent_edge_a: Some(lineage.push_edge.id.clone()),
        parent_edge_b: None,
        created_at: Timestamp::now(),
        to_machine: Some(lineage.original_machine.clone()),
    })?;
    Ok(())
}

/// Core: switches an away Instance back to the original on the machine
/// it came from. `choice` is empty, or the user's pick after a
/// `diverged` refusal: `keep_mine` | `keep_both` (`Keep theirs` sends
/// nothing — the UI just closes).
///
/// # Errors
///
/// Returns the typed [`AppError`] when it isn't a cross-machine away
/// Instance, can't run now, or the push can't be delivered.
pub async fn fleet_switch_back_execute_impl(
    state: Arc<AppState>,
    session_id: String,
    choice: String,
    keep_away_running: bool,
    mut on_progress: impl FnMut(FleetSwitchProgressInfo) + Send,
) -> AppResult<FleetSwitchOutcomeInfo> {
    use prost::Message as _;

    if !matches!(choice.as_str(), "" | "keep_mine" | "keep_both") {
        return Err(AppError::Validation {
            subject: "switch back",
            message: format!("unknown choice `{choice}`"),
        });
    }
    let not_away = || AppError::NotFound {
        entity: "cross-machine switch",
        id: session_id.clone(),
    };
    let plan = fleet_switch_back_plan_impl(&state, &session_id)?.ok_or_else(not_away)?;
    if !plan.blockers.is_empty() {
        return Err(AppError::Validation {
            subject: "switch back",
            message: plan.blockers.join(" "),
        });
    }
    let lineage = away_lineage(&state, &session_id)?.ok_or_else(not_away)?;
    let (_, away_account) = crate::commands::handover_source(&state, &session_id)?;
    let suffix = crate::commands::unique_suffix();
    let edge = SwitchEdgeId::new(format!("edge-{suffix}-back"));
    let new_epoch = EpochId::new(format!("ep-{suffix}-back"));
    stage_back(
        &state,
        &lineage,
        &away_account,
        &edge,
        choice == "keep_both",
    )?;
    let bundle = proto::HandoverBundle {
        provider_id: away_account.harness.to_string(),
        delta_turn: plan.delta_preview.clone(),
        ..proto::HandoverBundle::default()
    };
    let back = proto::SwitchBack {
        original_epoch_id: lineage.original.id.to_string(),
        from_epoch_id: lineage.away_epoch.id.to_string(),
        edge_id: edge.to_string(),
        new_epoch_id: new_epoch.to_string(),
        work_id: lineage.work.to_string(),
        bundle_bytes: 0,
        divergence_choice: choice.clone(),
        from_provider_id: away_account.harness.to_string(),
        from_account_label: away_account.label.clone(),
    };
    let build = move |total| {
        proto::control_command::Action::SwitchBack(proto::SwitchBack {
            bundle_bytes: total,
            ..back
        })
    };
    let pushed = super::push(
        &state,
        lineage.original_machine.as_str(),
        &plan.original_machine_label,
        build,
        bundle.encode_to_vec(),
        &mut on_progress,
    )
    .await;
    let fail = |reason: &str| {
        if let Ok(storage) = state.storage.lock() {
            let _ = storage
                .switch_graph()
                .mark_failed(&edge, FailureStage::Launch, reason);
        }
    };
    let (outcome, sequence) = match pushed {
        Ok(pushed) => pushed,
        Err(error) => {
            fail(&error.wire().user_message);
            return Err(error);
        }
    };
    conclude_back(
        &state,
        &lineage,
        &edge,
        &new_epoch,
        &choice,
        &session_id,
        keep_away_running,
        outcome,
        sequence,
    )
}

/// Audits the switch-back here and records its outcome.
#[allow(clippy::too_many_arguments)]
fn conclude_back(
    state: &Arc<AppState>,
    lineage: &AwayLineage,
    edge: &SwitchEdgeId,
    new_epoch: &EpochId,
    choice: &str,
    session_id: &str,
    keep_away_running: bool,
    outcome: proto::HandoverProgress,
    sequence: u64,
) -> AppResult<FleetSwitchOutcomeInfo> {
    let fail = |reason: &str| {
        if let Ok(storage) = state.storage.lock() {
            let _ = storage
                .switch_graph()
                .mark_failed(edge, FailureStage::Launch, reason);
        }
    };
    let result = outcome.result.clone().unwrap_or_default();
    let accepted = outcome.phase == "done" && result.outcome == "accepted";
    crate::fleet::append_audit(
        state,
        &juggler_link::node::LinkAuditEvent {
            peer: lineage.original_machine.to_string(),
            kind: "switch_back_sent".to_owned(),
            reason_code: if accepted {
                "accepted".to_owned()
            } else {
                result.reason_code.clone()
            },
            sequence: Some(sequence),
            count: None,
        },
    );
    if !accepted && result.reason_code == "diverged" {
        let divergence = outcome.divergence.clone().unwrap_or_default();
        let label = crate::fleet::peer_label(state, lineage.original_machine.as_str())
            .unwrap_or_else(|| lineage.original_machine.to_string());
        remember_divergence(
            state,
            DivergenceRecordInfo {
                session_id: session_id.to_owned(),
                machine_id: lineage.original_machine.to_string(),
                machine_label: label,
                turns_since: divergence.turns_since,
                last_activity_ms: Some(divergence.last_activity_ms).filter(|at| *at > 0),
                summary: divergence.summary,
                at_ms: Timestamp::now().millis(),
            },
        );
    }
    if accepted {
        forget_divergence(state, session_id);
    }
    if !accepted {
        let reason = if outcome.phase == "cancelled" {
            "cancelled".to_owned()
        } else {
            result.reason_code.clone()
        };
        fail(&result.message);
        return Ok(FleetSwitchOutcomeInfo {
            accepted: false,
            reason_code: Some(reason),
            message: result.message,
            instance_id: None,
            edge_id: Some(edge.to_string()),
            divergence: outcome.divergence.map(super::divergence_info),
        });
    }
    record_returned(state, lineage, edge, new_epoch, choice == "keep_both")?;
    if !keep_away_running {
        let _ = state
            .supervisor
            .stop(&SessionId::new(session_id), Duration::from_secs(5));
        let storage = crate::commands::lock_storage_arc(state)?;
        if let Ok(mut away) = storage.sessions().get(&SessionId::new(session_id)) {
            away.status = SessionStatus::HandedOver;
            storage.sessions().update(&away)?;
        }
    }
    Ok(FleetSwitchOutcomeInfo {
        accepted: true,
        reason_code: None,
        message: result.message,
        instance_id: Some(outcome.instance_id).filter(|id| !id.is_empty()),
        edge_id: Some(edge.to_string()),
        divergence: None,
    })
}

/// The original's machine took it: mirror its new epoch here and
/// confirm the edge.
fn record_returned(
    state: &Arc<AppState>,
    lineage: &AwayLineage,
    edge: &SwitchEdgeId,
    new_epoch: &EpochId,
    keep_both: bool,
) -> AppResult<()> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    graph.mirror_epoch(&Epoch {
        id: new_epoch.clone(),
        work: lineage.work.clone(),
        session_ref: None,
        native_id: lineage.original.native_id.clone().filter(|_| !keep_both),
        account: lineage.original.account.clone(),
        provider: lineage.original.provider.clone(),
        resumed_from_epoch: Some(lineage.original.id.clone()),
        machine: Some(lineage.original_machine.clone()),
    })?;
    graph.mark_launched(edge, new_epoch)?;
    graph.mark_confirmed(edge)?;
    Ok(())
}

// --------------------------------------------------- pending divergences

/// The app setting holding unresolved divergences by away Instance.
const DIVERGENCES_KEY: &str = "fleet_divergences";

/// One unresolved divergence: a switch-back the original's machine
/// refused because both sides moved on (AJ-E708 Req.4–5). It stays —
/// and blocks destructive actions on the Instance — until the user picks.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DivergenceRecordInfo {
    /// The away Instance here.
    pub session_id: String,
    /// The machine holding the original.
    pub machine_id: String,
    /// Its label.
    pub machine_label: String,
    /// Turns the original gained there since the work left.
    pub turns_since: u32,
    /// Its last activity there, epoch ms (that machine's clock).
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// Plain words.
    pub summary: String,
    /// When this Mac learned of it, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

fn divergences(state: &Arc<AppState>) -> Vec<DivergenceRecordInfo> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(DIVERGENCES_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_divergences(state: &Arc<AppState>, rows: &[DivergenceRecordInfo]) -> AppResult<()> {
    let raw = serde_json::to_string(rows).map_err(|e| AppError::Io {
        context: "fleet divergences",
        message: e.to_string(),
    })?;
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(DIVERGENCES_KEY, &raw)?;
    Ok(())
}

fn remember_divergence(state: &Arc<AppState>, row: DivergenceRecordInfo) {
    let mut rows = divergences(state);
    rows.retain(|existing| existing.session_id != row.session_id);
    rows.push(row);
    let _ = save_divergences(state, &rows);
}

fn forget_divergence(state: &Arc<AppState>, session_id: &str) {
    let mut rows = divergences(state);
    let before = rows.len();
    rows.retain(|existing| existing.session_id != session_id);
    if rows.len() != before {
        let _ = save_divergences(state, &rows);
    }
}

/// Whether `session_id` has an unresolved divergence.
#[must_use]
pub fn is_diverged(state: &Arc<AppState>, session_id: &str) -> bool {
    divergences(state)
        .iter()
        .any(|row| row.session_id == session_id)
}

/// Core: every unresolved divergence (the sync chip's "Diverged — N
/// needs you").
#[must_use]
pub fn fleet_divergences_impl(state: &Arc<AppState>) -> Vec<DivergenceRecordInfo> {
    divergences(state)
}

/// Core: "Keep theirs" — leave both sides as they are; nothing is sent.
/// The choice is recorded (audited) and the block lifts.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] when there is no such divergence.
pub fn fleet_divergence_keep_theirs_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<()> {
    let row = divergences(state)
        .into_iter()
        .find(|row| row.session_id == session_id)
        .ok_or_else(|| AppError::NotFound {
            entity: "divergence",
            id: session_id.to_owned(),
        })?;
    forget_divergence(state, session_id);
    crate::fleet::append_audit(
        state,
        &juggler_link::node::LinkAuditEvent {
            peer: row.machine_id,
            kind: "divergence_kept_theirs".to_owned(),
            reason_code: String::new(),
            sequence: None,
            count: Some(u64::from(row.turns_since)),
        },
    );
    Ok(())
}

// --------------------------------------------------------------- the node

/// What the original moved on by since the work left: turns appended
/// (the jsonl is append-only) — or, if its bytes up to then changed, a
/// rewrite. `None` = untouched.
fn divergence_of(
    current: &[u8],
    at_handover: &juggler_core::model::ManifestEntry,
) -> Option<(u32, String)> {
    let cut = usize::try_from(at_handover.bytes).unwrap_or(usize::MAX);
    if current.len() < cut
        || juggler_adapter::handover::sha256_hex(&current[..cut]) != at_handover.sha256
    {
        return Some((
            u32::try_from(juggler_adapter::handover::uuid_set(current).len()).unwrap_or(u32::MAX),
            "the original was rewritten".to_owned(),
        ));
    }
    if current.len() == cut {
        return None;
    }
    let turns = u32::try_from(juggler_adapter::handover::uuid_set(&current[cut..]).len())
        .unwrap_or(u32::MAX);
    Some((turns, format!("{turns} new turns")))
}

/// The original epoch here, its Instance, and the transcript row the
/// push recorded when the work left.
fn original_here(
    state: &Arc<AppState>,
    back: &proto::SwitchBack,
) -> Result<
    (
        Epoch,
        juggler_core::model::InstanceRecord,
        juggler_core::model::ManifestEntry,
    ),
    HandoverRejection,
> {
    let unknown = || refuse("unknown_record", "That work's original isn't on this Mac.");
    let storage = state
        .storage
        .lock()
        .map_err(|_| refuse("internal", "Storage unavailable."))?;
    let graph = storage.switch_graph();
    let original = graph
        .epoch(&EpochId::new(back.original_epoch_id.clone()))
        .ok()
        .flatten()
        .filter(|epoch| epoch.machine.is_none())
        .ok_or_else(unknown)?;
    let session = original
        .session_ref
        .as_ref()
        .and_then(|id| storage.sessions().get(id).ok())
        .ok_or_else(unknown)?;
    let row = graph
        .edges_for_work(&original.work)
        .unwrap_or_default()
        .into_iter()
        .find(|edge| {
            edge.from_epoch == original.id
                && edge.to_epoch.as_ref().map(EpochId::as_str) == Some(back.from_epoch_id.as_str())
        })
        .and_then(|edge| graph.manifest_for(&edge.id).ok())
        .and_then(|rows| rows.into_iter().find(|row| row.kind == "transcript"))
        .ok_or_else(unknown)?;
    Ok((original, session, row))
}

/// AJ-E706 node side: resume the original native session here with the
/// one delta turn — refusing (with what changed) when it diverged and
/// the user hasn't chosen.
pub(crate) fn switch_back_here(
    state: &Arc<AppState>,
    peer: &PeerAccess,
    back: &proto::SwitchBack,
    bundle: &proto::HandoverBundle,
) -> Result<HandoverAccepted, HandoverRejection> {
    use juggler_adapter::LaunchMode;

    let unknown = || refuse("unknown_record", "That work's original isn't on this Mac.");
    let (original, session, at_handover) = original_here(state, back)?;
    if matches!(
        session.status,
        SessionStatus::Starting
            | SessionStatus::Running
            | SessionStatus::WaitingUser
            | SessionStatus::Stuck
    ) {
        return Err(refuse(
            "busy",
            "The original is still running on this Mac — stop it first.",
        ));
    }
    let reference = session
        .transcript_ref
        .clone()
        .filter(|reference| reference.path.is_file())
        .ok_or_else(|| {
            refuse(
                "original_gone",
                "The original session is gone from this Mac.",
            )
        })?;
    let native = original
        .native_id
        .clone()
        .or_else(|| reference.native_id.clone())
        .ok_or_else(unknown)?;
    let current = std::fs::read(&reference.path)
        .map_err(|_| refuse("original_gone", "The original session couldn't be read."))?;
    if let Some((turns, summary)) = divergence_of(&current, &at_handover)
        && back.divergence_choice.is_empty()
    {
        let last_activity_ms = session
            .last_activity
            .map(Timestamp::millis)
            .or_else(|| file_mtime_ms(&reference.path))
            .unwrap_or(0);
        return Err(HandoverRejection {
            reason_code: "diverged".to_owned(),
            message: format!(
                "The original moved on here since the work left ({summary}) — choose what to keep."
            ),
            divergence: Some(proto::Divergence {
                turns_since: turns,
                last_activity_ms,
                summary,
            }),
        });
    }
    let (profile, account) =
        crate::commands::find_profile_with_account_arc(state, session.profile.as_str())
            .map_err(|_| unknown())?;
    let project = session
        .project
        .as_ref()
        .and_then(|id| state.storage.lock().ok()?.projects().get(id).ok())
        .ok_or_else(unknown)?;
    // Keep both: a fork of the original resumes; the original's own
    // continuation stays exactly as it is.
    let keep_both = back.divergence_choice == "keep_both";
    let (resume_id, resume_ref) = if keep_both {
        fork_original(&reference.path)?
    } else {
        (native, reference)
    };
    let (instance, _) = super::launch_here(
        state,
        &profile,
        &account,
        &project,
        &LaunchMode::Resume {
            session_id: resume_id.clone(),
            delta_prompt: Some(bundle.delta_turn.clone()).filter(|text| !text.is_empty()),
        },
        Some(resume_ref),
        "switch_back",
    )
    .map_err(super::launch_refusal)?;
    record_back(state, peer, back, &original, &account, &instance, resume_id)
        .map_err(|e| refuse("internal", e.wire().user_message))?;
    Ok(HandoverAccepted {
        message: if keep_both {
            format!(
                "Resumed a copy of the original under {} — the original stays as it was.",
                account.label
            )
        } else {
            format!("Resumed the original under {}.", account.label)
        },
        instance_id: instance.to_string(),
    })
}

fn file_mtime_ms(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let since = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    i64::try_from(since.as_millis()).ok()
}

/// Copies the original transcript to a new native id beside it (the
/// uuid tree kept, the session id rewritten — F-H-03's fork).
fn fork_original(path: &Path) -> Result<(String, TranscriptRef), HandoverRejection> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| refuse("internal", "Couldn't create the copy."))?;
    let hex: String = bytes.iter().fold(String::new(), |mut out, byte| {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
        out
    });
    let fork = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    );
    let original = std::fs::read(path)
        .map_err(|_| refuse("original_gone", "The original couldn't be read."))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    // AJ-E1304: a Codex rollout carries its thread id in its name and in
    // its lines; a Claude transcript is `<id>.jsonl` with `sessionId`.
    let codex_thread = (juggler_adapters::transcript::detect_format(&original)
        == juggler_adapters::transcript::TranscriptFormat::CodexRollout)
        .then(|| juggler_adapters::codex_transcript::rollout_thread_id(&original, &name))
        .flatten();
    let (target, bytes) = match codex_thread {
        // A rollout's name must carry its thread id, or the forked name
        // would be the original's (and overwrite it).
        Some(thread) if name.contains(&thread) => (
            path.with_file_name(name.replace(&thread, &fork)),
            juggler_adapters::codex_handover::fork_rollout(&original, &thread, &fork),
        ),
        Some(_) => {
            return Err(refuse(
                "internal",
                "This session's file name doesn't carry its id — it can't be copied safely.",
            ));
        }
        None => (
            path.with_file_name(format!("{fork}.jsonl")),
            juggler_adapter::handover::fork_session_id(&original, &fork),
        ),
    };
    if target == path {
        return Err(refuse("internal", "Couldn't write the copy."));
    }
    // Copy-never-move: a new file only — never over an existing one.
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .and_then(|mut out| std::io::Write::write_all(&mut out, &bytes))
        .map_err(|_| refuse("internal", "Couldn't write the copy."))?;
    Ok((
        fork.clone(),
        TranscriptRef {
            path: target,
            native_id: Some(fork),
        },
    ))
}

/// Records the switch-back here under the console's ids.
fn record_back(
    state: &Arc<AppState>,
    peer: &PeerAccess,
    back: &proto::SwitchBack,
    original: &Epoch,
    account: &juggler_core::model::Account,
    instance: &SessionId,
    native_id: String,
) -> AppResult<()> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let graph = storage.switch_graph();
    let away = EpochId::new(back.from_epoch_id.clone());
    graph.mirror_epoch(&Epoch {
        id: away.clone(),
        work: original.work.clone(),
        session_ref: None,
        native_id: None,
        account: AccountId::new(back.from_account_label.clone()),
        provider: HarnessId::new(back.from_provider_id.clone()),
        resumed_from_epoch: Some(original.id.clone()),
        machine: Some(MachineId::new(peer.machine_id.clone())),
    })?;
    let edge = SwitchEdgeId::new(back.edge_id.clone());
    graph.append_edge(&SwitchEdge {
        id: edge.clone(),
        work: original.work.clone(),
        from_epoch: away,
        to_epoch: None,
        from_account: AccountId::new(back.from_account_label.clone()),
        to_account: account.id.clone(),
        from_provider: HarnessId::new(back.from_provider_id.clone()),
        to_provider: account.harness.clone(),
        mechanism: SwitchMechanism::NativeResume,
        pack_id: None,
        fidelity_shown: Some(
            if back.divergence_choice == "keep_both" {
                "F3"
            } else {
                "F4"
            }
            .to_owned(),
        ),
        manifest_sha256: None,
        state: EdgeState::Staged,
        actor: EdgeActor::User,
        parent_edge_a: None,
        parent_edge_b: None,
        created_at: Timestamp::now(),
        to_machine: None,
    })?;
    let new_epoch = EpochId::new(back.new_epoch_id.clone());
    graph.insert_epoch(&Epoch {
        id: new_epoch.clone(),
        work: original.work.clone(),
        session_ref: Some(instance.clone()),
        native_id: Some(native_id),
        account: account.id.clone(),
        provider: account.harness.clone(),
        resumed_from_epoch: Some(original.id.clone()),
        machine: None,
    })?;
    graph.mark_launched(&edge, &new_epoch)?;
    graph.mark_confirmed(&edge)?;
    Ok(())
}

// ---------------------------------------------------------- the commands

/// AJ-E706: the cross-machine switch-back plan for an away Instance.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_switch_back_plan_impl`].
#[tauri::command]
pub fn fleet_switch_back_plan(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Option<FleetSwitchBackPlanInfo>> {
    fleet_switch_back_plan_impl(state.inner(), &session_id)
}

/// AJ-E708: every unresolved divergence.
#[tauri::command]
#[must_use]
pub fn fleet_divergences(state: tauri::State<'_, Arc<AppState>>) -> Vec<DivergenceRecordInfo> {
    fleet_divergences_impl(state.inner())
}

/// AJ-E708: "Keep theirs" for one divergence (nothing is sent).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_divergence_keep_theirs_impl`].
#[tauri::command]
pub fn fleet_divergence_keep_theirs(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<()> {
    fleet_divergence_keep_theirs_impl(state.inner(), &session_id)
}

/// AJ-E706: switches an away Instance back to its original machine.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_switch_back_execute_impl`].
#[tauri::command]
pub async fn fleet_switch_back_execute(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    choice: String,
    keep_away_running: bool,
) -> AppResult<FleetSwitchOutcomeInfo> {
    use tauri::Emitter as _;
    fleet_switch_back_execute_impl(
        Arc::clone(state.inner()),
        session_id,
        choice,
        keep_away_running,
        move |progress| {
            let _ = app.emit(super::EVENT_FLEET_SWITCH_PROGRESS, progress);
        },
    )
    .await
}

#[cfg(test)]
mod fork_tests {
    use super::fork_original;

    const THREAD: &str = "019a1b2c-0000-7000-8000-00000000c0de";

    fn rollout(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(
            &path,
            format!(
                "{{\"timestamp\":\"t\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{THREAD}\",\"cwd\":\"/w\"}}}}\n"
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn a_codex_fork_is_a_new_file_and_the_original_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = rollout(
            dir.path(),
            &format!("rollout-2026-10-20T08-00-00-{THREAD}.jsonl"),
        );
        let before = std::fs::read(&path).unwrap();
        let (fork, reference) = fork_original(&path).unwrap();
        assert_ne!(reference.path, path);
        assert!(reference.path.to_string_lossy().contains(&fork));
        assert!(
            std::fs::read_to_string(&reference.path)
                .unwrap()
                .contains(&fork)
        );
        assert_eq!(std::fs::read(&path).unwrap(), before, "copy, never move");
    }

    #[test]
    fn a_rollout_whose_name_lacks_its_id_is_refused_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = rollout(dir.path(), "rollout-renamed.jsonl");
        let before = std::fs::read(&path).unwrap();
        assert!(fork_original(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
