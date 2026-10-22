//! The conversations feature's commands.

use std::collections::HashMap;
use std::sync::Arc;

use super::dto::ConversationInfo;
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// Every account's conversations, newest first: each with its account,
/// project (worktrees folded into their repository), title, tokens,
/// whether a running session is on it, and the AI Juggler session it is
/// when AI Juggler started it.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn list_impl(state: &Arc<AppState>) -> AppResult<Vec<ConversationInfo>> {
    // The first ask after a start reads the files itself, unless the
    // background pass already is.
    super::read_if_cold(state)?;
    let (labels, projects, sessions) = {
        let storage = lock_storage_arc(state)?;
        let labels: HashMap<String, String> = storage
            .accounts()
            .list()?
            .into_iter()
            .map(|account| (account.id.to_string(), account.label))
            .collect();
        let projects: HashMap<std::path::PathBuf, String> = storage
            .projects()
            .list()?
            .into_iter()
            .map(|project| (project.root, project.id.to_string()))
            .collect();
        let sessions: HashMap<String, String> = storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter_map(|record| {
                Some((
                    record.conversation?.as_str().to_owned(),
                    record.id.to_string(),
                ))
            })
            .collect();
        (labels, projects, sessions)
    };
    Ok(state
        .conversations
        .all()?
        .into_iter()
        .map(|conversation| {
            let summary = &conversation.summary;
            let root = conversation
                .folder
                .as_deref()
                .map(juggler_core::storage::project_root);
            ConversationInfo {
                live: state.conversations.is_live(
                    &conversation.account_id,
                    &conversation.tool,
                    &summary.id,
                ),
                session_id: sessions.get(&summary.id).cloned(),
                project_id: root.and_then(|root| projects.get(&root).cloned()),
                account_label: labels
                    .get(&conversation.account_id)
                    .cloned()
                    .unwrap_or_default(),
                id: summary.id.clone(),
                tool: conversation.tool.clone(),
                account_id: conversation.account_id.clone(),
                cwd: summary
                    .cwd
                    .as_ref()
                    .map(|cwd| cwd.to_string_lossy().into_owned()),
                title: summary.title.clone(),
                git_branch: summary.git_branch.clone(),
                started_ms: summary.started_ms,
                last_activity_ms: summary.last_ms,
                tokens: crate::commands::tokens_info(summary.tokens),
            }
        })
        .collect())
}

/// Every account's conversations (AJ-E1529): the rail, a project's page,
/// Accounts' running counts and the go-to palette read this.
///
/// # Errors
///
/// As [`list_impl`].
#[tauri::command]
pub async fn conversations_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ConversationInfo>> {
    crate::ipc::blocking(state.inner(), "the conversations", list_impl).await
}

/// The copyable resume line for a conversation started outside AI Juggler
/// (its account's and its own ids; nothing else from the UI).
///
/// # Errors
///
/// As [`super::resume_command_impl`].
#[tauri::command]
pub async fn conversations_resume_command(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "the resume command", move |state| {
        super::resume_command_impl(state, &account_id, &conversation_id)
    })
    .await
}

/// "Continue in AI Juggler" on a conversation started outside it.
///
/// # Errors
///
/// As [`super::continue_impl`].
#[tauri::command]
pub async fn conversations_continue(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
) -> AppResult<crate::commands::LaunchResult> {
    let result = crate::ipc::blocking(state.inner(), "continuing the conversation", move |state| {
        super::continue_impl(state, &account_id, &conversation_id)
    })
    .await?;
    crate::commands::emit_sessions(&app, state.inner())?;
    Ok(result)
}
