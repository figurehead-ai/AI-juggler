//! "Prepare to uninstall" (Settings): before AI Juggler goes, everything
//! it changed outside its own data is put back — today, the status lines
//! of accounts measured outside AI Juggler (AJ-E1530), by the same rules
//! as turning that off — and what it leaves behind is listed: its data
//! folder, its Keychain items and the worktrees it made. Nothing is
//! deleted: the user decides, with the list in hand.

use std::path::Path;
use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::quota::capture::{home_relative, turn_off_impl};
use crate::state::AppState;

/// One account's status line, put back.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StatusLineBackInfo {
    /// The account's name.
    pub account: String,
    /// `restored`, `spliced` or `theirs` (as turning measuring off says),
    /// or `failed`.
    pub outcome: String,
    /// Why it failed, in words.
    #[ts(optional)]
    pub problem: Option<String>,
}

/// A worktree AI Juggler made.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WorktreeLeftInfo {
    /// Where it is (home-relative).
    pub path: String,
    /// Its branch.
    pub branch: String,
    /// It's still on disk.
    pub exists: bool,
}

/// What preparing did, and what's left for the user to remove.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UninstallPrepInfo {
    /// The status lines put back.
    pub status_lines: Vec<StatusLineBackInfo>,
    /// AI Juggler's data folder (home-relative): its database, backups,
    /// logs and the account copies it made.
    pub data_folder: String,
    /// The Keychain services AI Juggler's items are under.
    pub keychain_services: Vec<String>,
    /// The worktrees AI Juggler made.
    pub worktrees: Vec<WorktreeLeftInfo>,
}

/// Puts back what AI Juggler changed outside its data, and lists what's
/// left. Safe to run again: a status line already back is skipped.
///
/// # Errors
///
/// Storage errors only; a status line that can't be put back is reported
/// in the result.
pub fn prepare_impl(state: &Arc<AppState>) -> AppResult<UninstallPrepInfo> {
    let overlays = state.overlays_dir();
    let accounts = lock_storage_arc(state)?.accounts().list()?;
    let status_lines = accounts
        .iter()
        .filter(|account| {
            account.in_place
                && juggler_adapters::quota_ingest::outside_capture_record(&overlays, account.id.as_str())
                    .is_some()
        })
        .map(|account| match turn_off_impl(state, account.id.as_str()) {
            Ok(off) => StatusLineBackInfo { account: account.label.clone(), outcome: off.outcome, problem: None },
            Err(error) => StatusLineBackInfo {
                account: account.label.clone(),
                outcome: "failed".into(),
                problem: Some(error.to_string()),
            },
        })
        .collect();
    let edition = crate::edition::current();
    let worktrees = crate::worktree::created_worktrees(state)
        .into_iter()
        .map(|(path, branch)| WorktreeLeftInfo {
            exists: Path::new(&path).exists(),
            path: home_relative(Path::new(&path)),
            branch,
        })
        .collect();
    Ok(UninstallPrepInfo {
        status_lines,
        data_folder: home_relative(&state.data_dir),
        keychain_services: vec![
            edition.keychain().service().to_owned(),
            edition.vault_keychain().service().to_owned(),
        ],
        worktrees,
    })
}

/// Prepare to uninstall: put back what AI Juggler changed, list the rest.
///
/// # Errors
///
/// As [`prepare_impl`].
#[tauri::command]
pub async fn app_prepare_uninstall(state: tauri::State<'_, Arc<AppState>>) -> AppResult<UninstallPrepInfo> {
    crate::ipc::blocking(state.inner(), "preparing to uninstall", prepare_impl).await
}
