//! Sessions as a feature module (ADR-0019 §3). AJ-E1537: what a
//! session's menu needs from the backend that no other command gives.

pub mod commands;
pub mod dto;
pub mod restart;
pub mod rows;

use std::sync::Arc;

use juggler_core::model::SessionId;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The copyable line that resumes session `session_id`'s conversation in
/// a terminal, on its own account, from its own folder. Built by the
/// tool's adapter (no tool's command is written here or in the UI).
///
/// # Errors
///
/// [`AppError::Refused`] when the session has no conversation to resume,
/// or its tool can't resume from a terminal; lookups as storage reports.
pub fn resume_command_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<String> {
    let (record, account) = {
        let storage = lock_storage_arc(state)?;
        let record = storage.sessions().get(&SessionId::new(session_id))?;
        let profile = storage.profiles().get(&record.profile)?;
        let account = storage.accounts().get(&profile.account)?;
        (record, account)
    };
    let conversation = record.conversation.as_ref().ok_or_else(|| {
        AppError::Refused("this session has no conversation to resume yet".to_owned())
    })?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    juggler_adapter::adapter::shell_resume_snippet(
        adapter.as_ref(),
        &account.state_dir,
        conversation.as_str(),
        &record.cwd,
    )
    .ok_or_else(|| {
        AppError::Refused(format!(
            "{} can't resume a session from a terminal",
            adapter.spec().display_name
        ))
    })
}
