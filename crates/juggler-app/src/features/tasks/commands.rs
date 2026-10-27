//! The tasks' commands (E19). Only the app's own window calls them; an
//! agent can't (the bridge reaches tasks through its own calls, never
//! these). The window reads; agents write the tasks (D-44). Its only
//! writes are the user's: approve or send back a task in review, free a
//! stuck one, cancel one, answer an agent's held ask or request, start a
//! session for a task, look an issue up. None creates, edits, assigns or
//! links a task.

use std::sync::Arc;

use super::dto::{TaskCounts, TaskDetail, TaskInfo, TaskNeedInfo, TaskStartedInfo};
use crate::error::AppResult;
use crate::paging::{Page, PageRequest};
use crate::state::AppState;

/// One page of a pool's tasks, ready work first.
///
/// # Errors
///
/// As [`super::views::list`].
#[tauri::command]
pub async fn tasks_list(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    filter: Option<String>,
    words: Option<String>,
    page: PageRequest,
) -> AppResult<Page<TaskInfo>> {
    crate::ipc::blocking(state.inner(), "the tasks", move |state| {
        super::views::list(state, &pool_id, filter.as_deref(), words.as_deref(), &page)
    })
    .await
}

/// How many of a pool's tasks are in each status.
///
/// # Errors
///
/// As [`super::views::counts`].
#[tauri::command]
pub async fn tasks_counts(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
) -> AppResult<TaskCounts> {
    crate::ipc::blocking(state.inner(), "the tasks", move |state| {
        super::views::counts(state, &pool_id)
    })
    .await
}

/// One page of a pool's epics and stories (the tree's folds).
///
/// # Errors
///
/// As [`super::views::plan`].
#[tauri::command]
pub async fn tasks_plan(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    page: PageRequest,
) -> AppResult<Page<TaskInfo>> {
    crate::ipc::blocking(state.inner(), "the plan", move |state| {
        super::views::plan(state, &pool_id, &page)
    })
    .await
}

/// One page of the items under an epic or story.
///
/// # Errors
///
/// As [`super::views::children`].
#[tauri::command]
pub async fn tasks_children(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    parent: String,
    page: PageRequest,
) -> AppResult<Page<TaskInfo>> {
    crate::ipc::blocking(state.inner(), "the plan", move |state| {
        super::views::children(state, &pool_id, &parent, &page)
    })
    .await
}

/// One page of a session's tasks.
///
/// # Errors
///
/// As [`super::views::of_session`].
#[tauri::command]
pub async fn tasks_session(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    page: PageRequest,
) -> AppResult<Page<TaskInfo>> {
    crate::ipc::blocking(state.inner(), "the session's tasks", move |state| {
        super::views::of_session(state, &session_id, &page)
    })
    .await
}

/// One task in full.
///
/// # Errors
///
/// As [`super::views::detail`].
#[tauri::command]
pub async fn tasks_detail(
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
    cursor: Option<String>,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::views::detail(state, &task_id, cursor.as_deref())
    })
    .await
}

/// The user approves a task in review (D-44): it's done.
///
/// # Errors
///
/// As [`super::oversight::approve`].
#[tauri::command]
pub async fn tasks_approve(
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::oversight::approve(state, &task_id)
    })
    .await
}

/// The user sends a task in review back to in progress with a note its
/// holder gets (D-44).
///
/// # Errors
///
/// As [`super::oversight::send_back`].
#[tauri::command]
pub async fn tasks_send_back(
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
    note: String,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::oversight::send_back(state, &task_id, &note)
    })
    .await
}

/// The user frees a task whose holder's session ended or crashed (D-44).
///
/// # Errors
///
/// As [`super::oversight::free`].
#[tauri::command]
pub async fn tasks_free(
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::oversight::free(state, &task_id)
    })
    .await
}

/// The user cancels an open task (D-44).
///
/// # Errors
///
/// As [`super::oversight::cancel`].
#[tauri::command]
pub async fn tasks_cancel(
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::oversight::cancel(state, &task_id)
    })
    .await
}

/// What about tasks waits on the user (Needs you).
///
/// # Errors
///
/// As [`super::needs::needs`].
#[tauri::command]
pub async fn tasks_needs(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<TaskNeedInfo>> {
    crate::ipc::blocking(state.inner(), "what waits on you", super::needs::needs).await
}

/// The user's answer to a held change.
///
/// # Errors
///
/// As [`super::needs::answer`].
#[tauri::command]
pub async fn tasks_answer(
    state: tauri::State<'_, Arc<AppState>>,
    need_id: String,
    answer: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the answer", move |state| {
        super::needs::answer(state, &need_id, &answer)
    })
    .await
}

/// The user dismisses a request for a new session.
///
/// # Errors
///
/// As [`super::needs::dismiss`].
#[tauri::command]
pub async fn tasks_dismiss(
    state: tauri::State<'_, Arc<AppState>>,
    need_id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the request", move |state| {
        super::needs::dismiss(state, &need_id)
    })
    .await
}

/// The user starts a session for a task, on the launch settings they
/// picked (D-31).
///
/// # Errors
///
/// As [`super::launch::start`].
#[tauri::command]
pub async fn tasks_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
    profile_id: String,
    need_id: Option<String>,
) -> AppResult<TaskStartedInfo> {
    let started = crate::ipc::blocking(state.inner(), "the session", move |state| {
        super::launch::start(state, &task_id, &profile_id, need_id.as_deref())
    })
    .await?;
    crate::commands::emit_sessions(&app, state.inner())?;
    Ok(started)
}

/// The user looks a task's issue up (through their own `gh`).
///
/// # Errors
///
/// As [`super::issues::look_up`].
#[tauri::command]
pub async fn tasks_look_up(
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the issue", move |state| {
        super::issues::look_up(state, &task_id)
    })
    .await
}
