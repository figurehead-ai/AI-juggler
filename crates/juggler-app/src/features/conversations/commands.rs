//! The conversations feature's commands.

use std::collections::HashMap;
use std::sync::Arc;

use super::dto::{ConversationInfo, GoToOutcomeInfo, PlaceInfo};
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
    let (labels, folders, projects, sessions, homes) = {
        let storage = lock_storage_arc(state)?;
        let homes = storage.projects().worktree_homes()?;
        let accounts = storage.accounts().list()?;
        let folders: HashMap<std::path::PathBuf, String> = accounts
            .iter()
            .map(|account| {
                (
                    std::fs::canonicalize(&account.state_dir)
                        .unwrap_or_else(|_| account.state_dir.clone()),
                    account.label.clone(),
                )
            })
            .collect();
        let labels: HashMap<String, String> = accounts
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
        (labels, folders, projects, sessions, homes)
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
                .map(|folder| juggler_core::storage::project_root_with(folder, &homes));
            let mut info = ConversationInfo {
                live: false,
                places: Vec::new(),
                pid: None,
                state: None,
                probably: false,
                viewed_ms: None,
                background: false,
                inactive: state.conversations.is_inactive(
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
            };
            if let Some(entry) = state.conversations.live_entry(
                &conversation.account_id,
                &conversation.tool,
                &summary.id,
            ) {
                with_live(&mut info, &entry, &folders);
            }
            info
        })
        .collect())
}

/// AJ-E2101: what the live view says of an open conversation: where it's
/// open, its state, when it was last viewed, and the app's own title (what
/// its sidebar shows).
fn with_live(
    info: &mut ConversationInfo,
    entry: &super::scan::LiveEntry,
    folders: &HashMap<std::path::PathBuf, String>,
) {
    info.live = true;
    info.inactive = false;
    info.places = entry
        .places
        .iter()
        .map(|place| place_info(place, folders))
        .collect();
    info.pid = info.places.first().map(|place| place.pid);
    info.state = entry.state.map(|state| state.word().to_owned());
    info.probably = entry.probably && info.state.is_some();
    info.viewed_ms = entry.viewed_ms;
    info.background = entry.background;
    if let Some(title) = &entry.app_title {
        info.title = Some(title.clone());
    }
}

/// One place in words: the app, then its instance or the profile.
fn place_info(
    place: &super::scan::Place,
    folders: &HashMap<std::path::PathBuf, String>,
) -> PlaceInfo {
    let profile = place.profile.as_ref().map(|dir| {
        folders.get(dir).cloned().unwrap_or_else(|| {
            dir.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    });
    let app = place
        .host
        .as_ref()
        .map(|host| host.name.clone())
        .unwrap_or_default();
    let instance = place.host.as_ref().and_then(|host| host.instance.clone());
    let label = [Some(app.clone()), instance.clone()]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    PlaceInfo {
        app_id: place.host.as_ref().and_then(|host| host.app_id.clone()),
        app,
        profile,
        instance,
        pid: place.pid,
        kind: place.host.as_ref().and_then(|host| host.kind).map(|kind| {
            match kind {
                juggler_adapter::live::HostKind::Desktop => "desktop",
                juggler_adapter::live::HostKind::Terminal => "terminal",
                juggler_adapter::live::HostKind::Editor => "editor",
            }
            .to_owned()
        }),
        app_pid: place.host.as_ref().map(|host| host.pid),
        label,
    }
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

/// AJ-E2102: Go to it, for any session open now (AI Juggler's or not):
/// brings its app forward by the app's own process, then what the app
/// allows, within 1.5 s, off the main thread. Never raises a permission
/// prompt. `pid` picks one place when it's open in several.
///
/// # Errors
///
/// As [`super::goto::go_to_impl`].
#[tauri::command]
pub async fn conversations_go_to(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
    pid: Option<u32>,
) -> AppResult<GoToOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "going to the session", move |state| {
        super::goto::go_to_impl(
            state,
            &account_id,
            &conversation_id,
            pid,
            Arc::new(super::goto::MacDesktop),
        )
    })
    .await
}

/// AJ-E2104: a place's Open: brings the app whose process is `app_pid`
/// forward when an open session runs in it (never launches it), off the
/// main thread, within 1.5 s.
///
/// # Errors
///
/// As [`super::goto::bring_forward_impl`].
#[tauri::command]
pub async fn conversations_bring_forward(
    state: tauri::State<'_, Arc<AppState>>,
    app_pid: u32,
) -> AppResult<GoToOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "bringing the app forward", move |state| {
        super::goto::bring_forward_impl(state, app_pid, Arc::new(super::goto::MacDesktop))
    })
    .await
}

/// AJ-E2108: a session's last 2–3 messages, read from the end of its
/// transcript when its page opens, with keys and tokens taken out; kept
/// nowhere. Nothing is read when the setting is off.
///
/// # Errors
///
/// As [`super::last_messages::last_messages_impl`].
#[tauri::command]
pub async fn conversations_last_messages(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
) -> AppResult<super::last_messages::LastMessagesInfo> {
    crate::ipc::blocking(state.inner(), "the last messages", move |state| {
        super::last_messages::last_messages_impl(state, &account_id, &conversation_id)
    })
    .await
}
