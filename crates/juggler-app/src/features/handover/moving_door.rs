//! AJ-E2321 (the review's A5.3, A5.5, B5.5): the Moves door. Every move
//! AI Juggler knows of, read back from its journal, with what can be done
//! to it by its id: Undo (a committed move whose backups are kept, or an
//! interrupted one), Finish (an interrupted one that passed its checks
//! before the apps reopened), and its timeline for Copy details. Shown on
//! the conversation's page and in Settings › Desktop apps.

use std::path::Path;
use std::sync::Arc;

use juggler_handover::moving::{Phase, Recovery, Transaction};

use crate::features::places::hold;
use crate::state::AppState;

/// A move AI Juggler knows of, for its door (Undo, Finish, Copy details).
#[allow(
    clippy::struct_excessive_bools,
    reason = "wire DTO: each flag is its own fact the door shows"
)]
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveRecordInfo {
    /// Its id: every action names it.
    pub move_id: String,
    /// The moved conversation's id in its tool.
    pub conversation: String,
    /// The conversation's title in its app, when it has one.
    pub title: Option<String>,
    /// 0.5.1 F91: a repair of its worktree's ownership (Settings ›
    /// Desktop apps › Moving conversations), not a move.
    pub repair: bool,
    /// Where it moved from: the App's name, else its folder.
    pub from: String,
    /// Where it moved to.
    pub to: String,
    /// `committed`, `undone`, or `interrupted`.
    pub state: String,
    /// When (epoch ms).
    #[ts(type = "number")]
    pub at_ms: i64,
    /// For an interrupted one: what recovery found, in words.
    pub detail: Option<String>,
    /// Undo is offered (its backups are there, no later move owns its
    /// files).
    pub can_undo: bool,
    /// Why a committed move can't be undone any more, in words.
    pub undo_why: Option<String>,
    /// Finish is offered: it stopped after its checks passed, before the
    /// apps reopened.
    pub can_finish: bool,
    /// AJ-E2325: a committed move's backup is there (Undo restores from
    /// it; Delete backup lets it go).
    pub has_backup: bool,
    /// AJ-E2325: until when the policy keeps it (epoch ms); `None` with a
    /// backup: until the person deletes it.
    #[ts(type = "number | null")]
    pub kept_until_ms: Option<i64>,
    /// AJ-E2325: what deleting its backup gives back to the disk, in
    /// bytes (at most, when `backup_exact` is false: clones share blocks).
    #[ts(type = "number | null")]
    pub backup_bytes: Option<u64>,
    /// `backup_bytes` is exactly what deleting frees.
    pub backup_exact: bool,
    /// 0.5.1 F92: AI Juggler stopped during it and settled it at its next
    /// start: what happened, in words (and which Apps started again).
    pub recovered: Option<String>,
    /// Its journal, one line per event, for Copy details.
    pub timeline: Vec<String>,
    /// What it keeps where it was on purpose (output folders old messages
    /// name by their full path) and its own folder (backups, kept
    /// branches, work kept aside by Undo), `~`-shortened: cleanup keeps
    /// each while the move is listed.
    pub kept: Vec<String>,
}

/// What a move's journal and the disk say of it: its state's word and,
/// for one left part way, why.
fn state_of(txn: &Transaction) -> (&'static str, Option<String>) {
    let steps = txn.manifest.plan.steps.len();
    match txn.recover() {
        Ok(Recovery::Settled(Phase::Committed)) => ("committed", None),
        Ok(Recovery::Settled(Phase::Undone)) => ("undone", None),
        Ok(Recovery::Settled(phase)) if phase < Phase::Applying => (
            "interrupted",
            Some("it stopped before it changed anything: Undo lets it go".to_owned()),
        ),
        Ok(Recovery::Settled(Phase::Verified)) => (
            "interrupted",
            Some("it stopped after its checks passed, before the apps opened again: Finish checks it once more and keeps it; Undo puts both accounts back".to_owned()),
        ),
        Ok(Recovery::Settled(_)) => (
            "interrupted",
            Some("it stopped after its changes and before its last check: Finish checks it and keeps it; Undo puts both accounts back".to_owned()),
        ),
        Ok(Recovery::Applying { next }) => (
            "interrupted",
            Some(format!(
                "it stopped at step {} of {steps}: Finish goes on from there; Undo puts both accounts back",
                (next + 1).min(steps)
            )),
        ),
        Ok(Recovery::Undoing) => (
            "interrupted",
            Some("Undo stopped part way: Undo again finishes it".to_owned()),
        ),
        Ok(Recovery::Unclear(why)) => ("interrupted", Some(why)),
        Err(error) => ("interrupted", Some(error.to_string())),
    }
}

/// Whether a later move (in any state but undone) touches `txn`'s files:
/// it owns them now.
pub(super) fn moved_again(all: &[Transaction], txn: &Transaction) -> bool {
    later(all, txn).is_some()
}

/// The first later move (or repair) that owns `txn`'s files now.
fn later<'a>(all: &'a [Transaction], txn: &Transaction) -> Option<&'a Transaction> {
    all.iter().find(|other| {
        other.manifest.id != txn.manifest.id
            && other.manifest.created_ms >= txn.manifest.created_ms
            && other.manifest.phase != Phase::Undone
            && other.overlaps(txn)
    })
}

/// The App on `side` of the move (its folder when no App AI Juggler
/// knows of had it).
fn side_name(state: &Arc<AppState>, txn: &Transaction, host: Option<&Path>, tool: &Path) -> String {
    let named = txn.manifest.places.iter().find_map(|id| {
        crate::features::places::with_book(state, |book| book.places.get(id).cloned())
            .ok()
            .flatten()
            .filter(|place| host.is_some() && place.instance_dir.as_deref() == host)
            .map(|place| crate::features::places::named(state, &place))
    });
    named.unwrap_or_else(|| shown(host.unwrap_or(tool)))
}

/// Whether `txn` is committed with its backups there.
pub(super) fn committed_backup(txn: &Transaction) -> bool {
    txn.manifest.phase == Phase::Committed
        && txn.backups_kept()
        && txn.manifest.entries.iter().any(|e| e.backup.is_some())
}

/// Every move AI Juggler knows of, newest first, with any left part way
/// read back from its journal.
#[must_use]
pub fn moves(state: &Arc<AppState>) -> Vec<MoveRecordInfo> {
    let all = hold::all_moves(&state.data_dir);
    let mut out: Vec<MoveRecordInfo> = all
        .iter()
        .map(|txn| {
            let (label, detail) = state_of(txn);
            let size = committed_backup(txn)
                .then(|| super::retention::backup_size(txn))
                .flatten();
            let manifest = &txn.manifest;
            let request = &manifest.request;
            let repair = super::repair::is_repair(txn);
            let undo_why = match label {
                // 0.5.1 F91: a later repair of the same files says so.
                "committed" if later(&all, txn).is_some_and(super::repair::is_repair) => Some(
                    "its worktree's ownership was repaired since: undo the repair first".to_owned(),
                ),
                "committed" if moved_again(&all, txn) => {
                    Some("moved again since: undo the later move first".to_owned())
                }
                "committed" if !txn.backups_kept() => {
                    Some("its backup was deleted, so it can't be undone".to_owned())
                }
                _ => None,
            };
            MoveRecordInfo {
                move_id: manifest.id.clone(),
                conversation: request.conversation.clone(),
                title: manifest
                    .plan
                    .members
                    .iter()
                    .find(|m| m.conversation == request.conversation)
                    .and_then(|m| m.title.clone())
                    .or_else(|| super::repair::repair_title(txn)),
                repair,
                from: side_name(
                    state,
                    txn,
                    request.source.host_dir.as_deref(),
                    &request.source.tool_dir,
                ),
                to: side_name(
                    state,
                    txn,
                    request.target.host_dir.as_deref(),
                    &request.target.tool_dir,
                ),
                state: label.to_owned(),
                at_ms: manifest.created_ms,
                detail: detail.or(manifest.problem.clone()),
                can_undo: match label {
                    "committed" => undo_why.is_none(),
                    "interrupted" => true,
                    _ => false,
                },
                undo_why,
                // AJ-E2321 R4: also one that stopped in or after its changes.
                can_finish: label == "interrupted"
                    && matches!(
                        txn.recover(),
                        Ok(Recovery::Applying { .. }
                            | Recovery::Settled(Phase::Applied | Phase::Verified))
                    ),
                has_backup: committed_backup(txn),
                kept_until_ms: committed_backup(txn)
                    .then(|| super::retention::kept_until_of(state, txn))
                    .flatten(),
                backup_bytes: size.map(|size| size.bytes),
                backup_exact: size.is_some_and(|size| size.exact),
                recovered: super::recover::recovered_words(txn),
                timeline: txn
                    .journal()
                    .iter()
                    .map(|line| super::timeline::line_words(line, manifest.created_ms))
                    .collect(),
                kept: txn.dependencies().iter().map(|path| shown(path)).collect(),
            }
        })
        .collect();
    out.reverse();
    out
}

/// `path` with the home folder as `~`.
pub(super) fn shown(path: &Path) -> String {
    match juggler_platform::user::home().and_then(|home| {
        path.strip_prefix(home)
            .ok()
            .map(std::path::Path::to_path_buf)
    }) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}
