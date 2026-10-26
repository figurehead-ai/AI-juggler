//! The messages' commands (E18). Only the app's own window calls them;
//! an agent can't (the bridge reaches messages through its own calls,
//! never these). Each one is the user's: reading, writing, Deliver,
//! pausing, answering a held ask, linking pools.

use std::sync::Arc;

use super::dto::{
    ComposeInput, DocContent, DocsPage, MessageInfo, MessageNeedInfo, ThreadPage, ThreadsPage,
};
use crate::error::AppResult;
use crate::state::AppState;
use juggler_core::model::SessionId;

/// One page of a pool's threads, or a session's.
///
/// # Errors
///
/// As [`super::views::threads`].
#[tauri::command]
pub async fn messages_threads(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: Option<String>,
    session_id: Option<String>,
    cursor: Option<String>,
) -> AppResult<ThreadsPage> {
    crate::ipc::blocking(state.inner(), "the messages", move |state| {
        super::views::threads(
            state,
            pool_id.as_deref(),
            session_id.as_deref(),
            cursor.as_deref(),
        )
    })
    .await
}

/// One page of a thread (its messages to the user are marked read).
///
/// # Errors
///
/// As [`super::views::thread`].
#[tauri::command]
pub async fn messages_thread(
    state: tauri::State<'_, Arc<AppState>>,
    thread_id: String,
    cursor: Option<String>,
) -> AppResult<ThreadPage> {
    crate::ipc::blocking(state.inner(), "the thread", move |state| {
        super::views::thread(state, &thread_id, cursor.as_deref())
    })
    .await
}

/// The user writes to a member or the pool.
///
/// # Errors
///
/// As [`super::views::compose`].
#[tauri::command]
pub async fn messages_send(
    state: tauri::State<'_, Arc<AppState>>,
    input: ComposeInput,
) -> AppResult<ThreadPage> {
    crate::ipc::blocking(state.inner(), "the message", move |state| {
        super::views::compose(state, input)
    })
    .await
}

/// The user's Deliver for one recipient of a message.
///
/// # Errors
///
/// As [`super::views::deliver`].
#[tauri::command]
pub async fn messages_deliver(
    state: tauri::State<'_, Arc<AppState>>,
    message_id: String,
    recipient: String,
) -> AppResult<ThreadPage> {
    crate::ipc::blocking(state.inner(), "the delivery", move |state| {
        super::views::deliver(state, &message_id, &recipient)
    })
    .await
}

/// The user's Deliver for every message waiting for a session (Needs
/// you's one click).
///
/// # Errors
///
/// As [`super::views::deliver_waiting`].
#[tauri::command]
pub async fn messages_deliver_waiting(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<u32> {
    crate::ipc::blocking(state.inner(), "the delivery", move |state| {
        super::views::deliver_waiting(state, &session_id)
    })
    .await
}

/// The user pauses, stops or resumes a thread.
///
/// # Errors
///
/// As [`super::views::set_thread`].
#[tauri::command]
pub async fn messages_set_thread(
    state: tauri::State<'_, Arc<AppState>>,
    thread_id: String,
    to: String,
) -> AppResult<ThreadPage> {
    crate::ipc::blocking(state.inner(), "the thread", move |state| {
        super::views::set_thread(state, &thread_id, &to)
    })
    .await
}

/// The user pauses or resumes a pool's messages.
///
/// # Errors
///
/// As [`super::views::set_pool`].
#[tauri::command]
pub async fn messages_set_pool(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    paused: bool,
) -> AppResult<ThreadsPage> {
    crate::ipc::blocking(state.inner(), "the pool's messages", move |state| {
        super::views::set_pool(state, &pool_id, paused)
    })
    .await
}

/// What about messages waits on the user (Needs you).
///
/// # Errors
///
/// As [`super::needs::needs`].
#[tauri::command]
pub async fn messages_needs(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<MessageNeedInfo>> {
    crate::ipc::blocking(state.inner(), "what waits on you", super::needs::needs).await
}

/// The user's answer to a held ask.
///
/// # Errors
///
/// As [`super::needs::answer`].
#[tauri::command]
pub async fn messages_answer(
    state: tauri::State<'_, Arc<AppState>>,
    ask_id: String,
    answer: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the answer", move |state| {
        super::needs::answer(state, &ask_id, &answer)
    })
    .await
}

/// One page of a pool's shared docs.
///
/// # Errors
///
/// As [`super::docs::list`].
#[tauri::command]
pub async fn messages_docs(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    cursor: Option<String>,
) -> AppResult<DocsPage> {
    crate::ipc::blocking(state.inner(), "the shared docs", move |state| {
        super::docs::list(state, &pool_id, cursor.as_deref())
    })
    .await
}

/// One shared doc's text.
///
/// # Errors
///
/// As [`super::docs::content`].
#[tauri::command]
pub async fn messages_doc(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    doc_id: String,
) -> AppResult<DocContent> {
    crate::ipc::blocking(state.inner(), "the shared doc", move |state| {
        super::docs::content(state, &pool_id, &doc_id)
    })
    .await
}

/// The user is typing in a session's message box, or stopped (a
/// delivery waits while they type).
///
/// # Errors
///
/// None today (kept for the IPC shape).
#[tauri::command]
pub async fn messages_typing(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    typing: bool,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the message box", move |state| {
        super::delivery::set_typing(state, &SessionId::new(session_id), typing);
        Ok(())
    })
    .await
}

/// A pool's messages with every word of `query`.
///
/// # Errors
///
/// As [`super::views::search`].
#[tauri::command]
pub async fn messages_search(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    query: String,
) -> AppResult<Vec<MessageInfo>> {
    crate::ipc::blocking(state.inner(), "the search", move |state| {
        super::views::search(state, &pool_id, &query)
    })
    .await
}

/// The pools linked with a pool.
///
/// # Errors
///
/// As [`super::views::links`].
#[tauri::command]
pub async fn messages_links(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
) -> AppResult<Vec<String>> {
    crate::ipc::blocking(state.inner(), "the pool's links", move |state| {
        super::views::links(state, &pool_id)
    })
    .await
}

/// The user links two pools, or takes the link back.
///
/// # Errors
///
/// As [`super::views::link`].
#[tauri::command]
pub async fn messages_link(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    other: String,
    on: bool,
) -> AppResult<Vec<String>> {
    crate::ipc::blocking(state.inner(), "the pools' link", move |state| {
        super::views::link(state, &pool_id, &other, on)
    })
    .await
}
