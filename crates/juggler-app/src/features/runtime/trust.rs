//! Lab 2 (D-40, 2026-10-25): a tool's folder-trust question, asked in AI
//! Juggler. Some tools ask whether to trust a folder and save the answer to
//! the user's own config (Codex's terminal does). AI Juggler never lets that
//! write happen for the user:
//! - it asks the tool's own question itself, once per tool and folder, and
//!   keeps the answer in its own store ([`KEY`]);
//! - each launch of that tool in that folder, driven or in its terminal
//!   (a hand-over too), gets the answer in memory
//!   (`HarnessAdapter::apply_folder_trust`);
//! - with no answer yet, the launch passes "not trusted" in memory (the
//!   tool's restricted mode, nothing saved);
//! - a folder the user already decided in the tool itself gets nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::HarnessAdapter;
use juggler_core::model::{Account, ProfileId};
use juggler_core::storage::Storage;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The store's key for the answers (one JSON object: `tool:folder` →
/// `trusted` or `untrusted`).
pub const KEY: &str = "runtime.trust";

/// What the window shows to ask, and the answer so far.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FolderTrustInfo {
    /// The folder, as the launch uses it.
    pub folder: String,
    /// The tool's question.
    pub question: String,
    /// The tool's words for trusting it.
    pub trust: String,
    /// The tool's words for not trusting it.
    pub restrict: String,
    /// What the choice does, in one line.
    pub note: String,
    /// `trusted`, `untrusted`, `in_tool` (decided in the tool itself), or
    /// `null`: not asked yet.
    pub choice: Option<String>,
}

fn folder_key(harness: &str, cwd: &Path) -> String {
    let folder = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    format!("{harness}:{}", folder.display())
}

fn answers(storage: &Storage) -> BTreeMap<String, String> {
    storage
        .app_settings()
        .get(KEY)
        .ok()
        .flatten()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// The user's answer for `harness` in `cwd`, when they gave one.
#[must_use]
pub fn choice(storage: &Storage, harness: &str, cwd: &Path) -> Option<bool> {
    answers(storage)
        .get(&folder_key(harness, cwd))
        .map(|word| word == "trusted")
}

/// Keeps the user's answer.
///
/// # Errors
///
/// Storage failures.
pub fn choose(storage: &Storage, harness: &str, cwd: &Path, trusted: bool) -> AppResult<()> {
    let mut all = answers(storage);
    all.insert(
        folder_key(harness, cwd),
        if trusted { "trusted" } else { "untrusted" }.to_owned(),
    );
    let text = serde_json::to_string(&all).map_err(|e| AppError::Refused(e.to_string()))?;
    storage.app_settings().set(KEY, &text)?;
    Ok(())
}

/// What a launch of `adapter` in `cwd` passes: `None` for a tool that asks
/// nothing or a folder decided in the tool; else the answer (not trusted
/// until the user says otherwise).
#[must_use]
pub fn plan(
    storage: &Storage,
    adapter: &dyn HarnessAdapter,
    account: &Account,
    cwd: &Path,
) -> Option<bool> {
    adapter.folder_trust()?;
    if adapter.folder_trust_decided(account, cwd) {
        return None;
    }
    Some(choice(storage, adapter.id().as_str(), cwd).unwrap_or(false))
}

/// The launch settings' tool and folder (`cwd`, else its own default).
fn target(
    state: &Arc<AppState>,
    profile: &str,
    cwd: Option<&str>,
) -> AppResult<Option<(juggler_adapter::SharedAdapter, Account, PathBuf)>> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let profile = storage.profiles().get(&ProfileId::new(profile))?;
    let account = storage.accounts().get(&profile.account)?;
    let Some(folder) = cwd.map(PathBuf::from).or(profile.default_cwd) else {
        return Ok(None);
    };
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    Ok(Some((adapter, account, folder)))
}

/// The question for a launch of `profile` in `cwd`, with the answer so far;
/// `None` when its tool asks nothing.
///
/// # Errors
///
/// An unknown profile or account; storage failures.
pub fn trust_impl(
    state: &Arc<AppState>,
    profile: &str,
    cwd: Option<&str>,
) -> AppResult<Option<FolderTrustInfo>> {
    let Some((adapter, account, folder)) = target(state, profile, cwd)? else {
        return Ok(None);
    };
    let Some(words) = adapter.folder_trust() else {
        return Ok(None);
    };
    let choice = if adapter.folder_trust_decided(&account, &folder) {
        Some("in_tool".to_owned())
    } else {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        choice(&storage, adapter.id().as_str(), &folder)
            .map(|trusted| if trusted { "trusted" } else { "untrusted" }.to_owned())
    };
    Ok(Some(FolderTrustInfo {
        folder: folder.display().to_string(),
        question: words.question.to_owned(),
        trust: words.trust.to_owned(),
        restrict: words.restrict.to_owned(),
        note: words.note.to_owned(),
        choice,
    }))
}

/// The user's answer (their click), kept for the tool and folder.
///
/// # Errors
///
/// As [`trust_impl`]; [`AppError::Refused`] for a tool that asks nothing.
pub fn trust_choose_impl(
    state: &Arc<AppState>,
    profile: &str,
    cwd: Option<&str>,
    trusted: bool,
) -> AppResult<Option<FolderTrustInfo>> {
    let Some((adapter, _, folder)) = target(state, profile, cwd)? else {
        return Err(AppError::Refused(
            "these launch settings have no folder".into(),
        ));
    };
    if adapter.folder_trust().is_none() {
        return Err(AppError::Refused(
            "this tool doesn't ask about folders".into(),
        ));
    }
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        choose(&storage, adapter.id().as_str(), &folder, trusted)?;
    }
    trust_impl(state, profile, cwd)
}

/// The folder-trust question for a launch.
///
/// # Errors
///
/// As [`trust_impl`].
#[tauri::command]
pub async fn runtime_trust(
    state: tauri::State<'_, Arc<AppState>>,
    profile_id: String,
    cwd: Option<String>,
) -> AppResult<Option<FolderTrustInfo>> {
    crate::ipc::blocking(state.inner(), "the folder's trust", move |state| {
        trust_impl(state, &profile_id, cwd.as_deref())
    })
    .await
}

/// The user's answer to the folder-trust question.
///
/// # Errors
///
/// As [`trust_choose_impl`].
#[tauri::command]
pub async fn runtime_trust_choose(
    state: tauri::State<'_, Arc<AppState>>,
    profile_id: String,
    cwd: Option<String>,
    trusted: bool,
) -> AppResult<Option<FolderTrustInfo>> {
    crate::ipc::blocking(state.inner(), "the folder's trust", move |state| {
        trust_choose_impl(state, &profile_id, cwd.as_deref(), trusted)
    })
    .await
}
