//! AJ-E209 Req.5: exporting an epoch's pack through the redaction policy.

use std::sync::Arc;

use juggler_core::model::{Account, Timestamp};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::redaction::redaction_global_of;
use crate::features::switch::back::transcript_side;
use crate::features::switch::resume::find_epoch_with_session;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// Core implementation shared with integration tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] per the command docs.
pub fn pack_export_impl(
    state: &Arc<AppState>,
    epoch_id: &str,
) -> AppResult<crate::features::handover::view::SavedHandoverInfo> {
    let (epoch, session_row, account) = find_epoch_with_session(state, epoch_id)?;
    let dir = export_dir(state);
    stage_session_pack(
        state,
        &session_row,
        &account,
        &PackIds {
            work: epoch.work.to_string(),
            epoch: epoch.id.to_string(),
            native: epoch.native_id.clone().unwrap_or_default(),
        },
        &dir,
    )?;
    crate::features::handover::view::saved_info(state, &dir)
}

/// A new folder for a saved handover, under `<app-data>/exports/`.
pub(crate) fn export_dir(state: &Arc<AppState>) -> std::path::PathBuf {
    state
        .data_dir
        .join("exports")
        .join(format!("pack-{}", unique_suffix()))
}

/// The lineage ids a staged pack carries.
pub(crate) struct PackIds {
    pub work: String,
    pub epoch: String,
    pub native: String,
}

/// Writes the redacted portable pack for one Instance into `dir` — the
/// shared body of a standalone export and of parking (AJ-E1003). The
/// GLOBAL redaction policy governs (unset blocks — no default applied).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the policy is unset, the session
/// has no transcript, or the pack can't be written.
pub(crate) fn stage_session_pack(
    state: &Arc<AppState>,
    session_row: &juggler_core::model::InstanceRecord,
    account: &Account,
    ids: &PackIds,
    dir: &std::path::Path,
) -> AppResult<()> {
    write_session_pack(state, session_row, account, ids, dir, None)
}

/// [`stage_session_pack`], given the turns when the caller already
/// parsed the transcript (AJ-E1513: staging reads it once).
pub(crate) fn write_session_pack(
    state: &Arc<AppState>,
    session_row: &juggler_core::model::InstanceRecord,
    account: &Account,
    ids: &PackIds,
    dir: &std::path::Path,
    turns: Option<Vec<juggler_handover::pack::IrTurn>>,
) -> AppResult<()> {
    use juggler_handover::pack::{ConversationMode, PackInput, PackSource};
    use juggler_handover::redaction::{RedactionApply, RedactionPolicy, redacted_pack};

    let policy = {
        let storage = lock_storage_arc(state)?;
        juggler_handover::redaction::resolve_policy(redaction_global_of(&storage), None)
            .ok_or_else(|| AppError::Io {
                context: "export pack",
                message: "Choose what happens to keys and passwords first, in Settings › Switching › Handovers and keys"
                    .to_owned(),
            })?
    };
    let reference = session_row
        .transcript_ref
        .clone()
        .ok_or_else(|| AppError::Io {
            context: "export pack",
            message: "This step has no conversation to save.".to_owned(),
        })?;
    let turns = turns.unwrap_or_else(|| {
        transcript_side(&reference.path)
            .map(|(_, turns)| turns)
            .unwrap_or_default()
    });
    let input = PackInput {
        pack_id: format!("pack-{}", unique_suffix()),
        created_at_ms: Timestamp::now().millis(),
        work_id: ids.work.clone(),
        edge_id: None,
        source: PackSource {
            provider: account.harness.to_string(),
            account_label: account.label.clone(),
            profile_name: None,
            native_session_id: ids.native.clone(),
            harness_version: None,
            epoch_id: ids.epoch.clone(),
        },
        target_hint: None,
        project: juggler_handover::pack::PackProject {
            root: session_row.cwd.to_string_lossy().into_owned(),
            project_id: session_row
                .project
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        },
        // AJ-E1002: the pack records the branch and, for a linked
        // worktree, its path (switches keep the binding).
        git: juggler_handover::pack::PackGit {
            branch: crate::worktree::branch_of(&session_row.cwd)
                .unwrap_or_else(|| "unknown".to_owned()),
            head_sha: None,
            dirty: crate::worktree::read_status(&session_row.cwd)
                .is_some_and(|status| status.changed + status.untracked > 0),
            diff_stat: crate::worktree::work_changes(&session_row.cwd)
                .and_then(|changes| changes.stat),
            worktree_path: crate::worktree::is_linked_worktree(&session_row.cwd)
                .then(|| session_row.cwd.to_string_lossy().into_owned()),
        },
        turns,
        compacted: false,
        artifacts: juggler_handover::pack::PackArtifacts::default(),
        fidelity: juggler_handover::pack::PackFidelity {
            level: "F1".to_owned(),
            score: 0.5,
            caps_applied: vec!["unvalidated-pair".to_owned()],
        },
        quota_at_creation: None,
        lineage: juggler_handover::pack::PackLineage::default(),
        // ADR-0009: the uncommitted changes ride `diffs/`, redacted with
        // the conversation (live lab 13: they never did).
        staged_files: crate::worktree::uncommitted_patch(&session_row.cwd)
            .map(|patch| vec![("diffs/uncommitted.patch".to_owned(), patch)])
            .unwrap_or_default(),
    };
    let apply = RedactionApply {
        strip: policy == RedactionPolicy::Strip,
        keep: std::collections::BTreeSet::new(),
    };
    let pack = redacted_pack(&input, ConversationMode::Distilled { last_n: 12 }, &apply);
    std::fs::create_dir_all(dir).map_err(|error| AppError::Io {
        context: "export pack",
        message: format!("could not create {}: {error}", dir.display()),
    })?;
    juggler_handover::pack::write_pack(dir, &pack).map_err(|error| AppError::Io {
        context: "export pack",
        message: error.to_string(),
    })?;
    if crate::usage_stats::recording() {
        #[allow(clippy::cast_precision_loss, reason = "a size into a bucket")]
        let kb = crate::usage_stats::dir_bytes(dir) as f64 / 1024.0;
        crate::usage_stats::observe("pack.size_kb", &[], kb);
    }
    Ok(())
}
