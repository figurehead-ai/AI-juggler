//! The Subagents tab's commands (AJ-E1520: 0.2, the list and a
//! subagent's conversation; 0.3, the views, a subagent's page and a
//! project's subagents).

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::conversations::last_messages::{Ask, LastMessagesInfo};
use crate::paging::PageRequest;
use crate::state::AppState;

use super::dto::{SubagentDetailInfo, SubagentsInfo, SubagentsQuery};

/// A session's subagents (and theirs), a page of rows at a time: grouped
/// under the message that started them, each group a tree, in the view,
/// search, type and order asked. Any session the app shows — AI
/// Juggler's or another app's — by its account and the tool's id for
/// it; read from the tool's own records with its adapter, read-only.
///
/// # Errors
///
/// As [`super::list_impl`].
#[tauri::command]
pub async fn subagents_list(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
    query: Option<SubagentsQuery>,
    page: Option<PageRequest>,
) -> AppResult<SubagentsInfo> {
    crate::ipc::blocking(state.inner(), "the subagents", move |state| {
        super::list_impl(
            state,
            &account_id,
            &conversation_id,
            &query.unwrap_or_default(),
            &page.unwrap_or_default(),
        )
    })
    .await
}

/// AJ-E1520 (0.3): one subagent's own page: what it was given and
/// answered, its work, usage and cost, the processes it probably
/// started, and how to stop it.
///
/// # Errors
///
/// As [`super::detail::detail_impl`].
#[tauri::command]
pub async fn subagents_detail(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
    subagent_id: String,
) -> AppResult<SubagentDetailInfo> {
    crate::ipc::blocking(state.inner(), "the subagent", move |state| {
        super::detail::detail_impl(state, &account_id, &conversation_id, &subagent_id)
    })
    .await
}

/// AJ-E1520 (0.3): a project's subagents, its newest sessions' together,
/// as a session's tab shows them.
///
/// # Errors
///
/// As [`super::project::project_impl`].
#[tauri::command]
pub async fn subagents_project(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    query: Option<SubagentsQuery>,
    page: Option<PageRequest>,
) -> AppResult<SubagentsInfo> {
    crate::ipc::blocking(state.inner(), "the project's subagents", move |state| {
        super::project::project_impl(
            state,
            &project_id,
            &query.unwrap_or_default(),
            &page.unwrap_or_default(),
        )
    })
    .await
}

/// One subagent's conversation, read like the session's own last
/// messages (`before` pages back, `from` follows along, `known` answers
/// `unchanged` without a read), redacted, kept nowhere; off with them.
///
/// # Errors
///
/// As [`super::conversation_impl`].
#[tauri::command]
#[allow(clippy::too_many_arguments)] // the last-messages ask, as its own command takes it
pub async fn subagents_conversation(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    conversation_id: String,
    subagent_id: String,
    known: Option<String>,
    before: Option<u64>,
    from: Option<u64>,
    max: Option<u32>,
) -> AppResult<LastMessagesInfo> {
    let ask = Ask {
        known,
        before,
        from,
        max: max.and_then(|max| usize::try_from(max).ok()),
    };
    crate::ipc::blocking(state.inner(), "the subagent's messages", move |state| {
        super::conversation_impl(state, &account_id, &conversation_id, &subagent_id, &ask)
    })
    .await
}
