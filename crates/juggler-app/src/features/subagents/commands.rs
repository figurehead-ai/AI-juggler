//! The Subagents tab's commands (AJ-E1520, 0.2).

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::conversations::last_messages::{Ask, LastMessagesInfo};
use crate::paging::PageRequest;
use crate::state::AppState;

/// A session's subagents (and theirs), a page at a time: newest first,
/// each followed by the ones it started. Any session the app shows —
/// AI Juggler's or another app's — by its account and the tool's id for
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
    page: Option<PageRequest>,
) -> AppResult<super::dto::SubagentsInfo> {
    crate::ipc::blocking(state.inner(), "the subagents", move |state| {
        super::list_impl(
            state,
            &account_id,
            &conversation_id,
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
