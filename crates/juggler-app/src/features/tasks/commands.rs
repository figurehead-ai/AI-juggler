//! The tasks' commands (E19). Only the app's own window calls them; an
//! agent can't (the bridge reaches tasks through its own calls, never
//! these). Each write is the user's: a task, a change, an assignment, a
//! link, an answer to a held ask, a session started for a task.

use std::sync::Arc;

use super::dto::{
    TaskDetail, TaskEdit, TaskInfo, TaskInput, TaskNeedInfo, TaskStartedInfo, TasksPage,
};
use crate::error::AppResult;
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
    cursor: Option<String>,
) -> AppResult<TasksPage> {
    crate::ipc::blocking(state.inner(), "the tasks", move |state| {
        super::views::list(
            state,
            &pool_id,
            filter.as_deref(),
            words.as_deref(),
            cursor.as_deref(),
        )
    })
    .await
}

/// A pool's epics and stories (the tree's folds).
///
/// # Errors
///
/// As [`super::views::plan`].
#[tauri::command]
pub async fn tasks_plan(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
) -> AppResult<Vec<TaskInfo>> {
    crate::ipc::blocking(state.inner(), "the plan", move |state| {
        super::views::plan(state, &pool_id)
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
    cursor: Option<String>,
) -> AppResult<TasksPage> {
    crate::ipc::blocking(state.inner(), "the plan", move |state| {
        super::views::children(state, &pool_id, &parent, cursor.as_deref())
    })
    .await
}

/// A session's tasks.
///
/// # Errors
///
/// As [`super::views::of_session`].
#[tauri::command]
pub async fn tasks_session(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Vec<TaskInfo>> {
    crate::ipc::blocking(state.inner(), "the session's tasks", move |state| {
        super::views::of_session(state, &session_id)
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

/// The user adds a task.
///
/// # Errors
///
/// As [`super::views::create`].
#[tauri::command]
pub async fn tasks_create(
    state: tauri::State<'_, Arc<AppState>>,
    input: TaskInput,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::views::create(state, input)
    })
    .await
}

/// The user changes a task.
///
/// # Errors
///
/// As [`super::views::edit`].
#[tauri::command]
pub async fn tasks_edit(
    state: tauri::State<'_, Arc<AppState>>,
    edit: TaskEdit,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::views::edit(state, &edit)
    })
    .await
}

/// The user gives a task to a member, to themselves or to no one.
///
/// # Errors
///
/// As [`super::views::assign`].
#[tauri::command]
pub async fn tasks_assign(
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
    to: String,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::views::assign(state, &task_id, &to)
    })
    .await
}

/// The user links two tasks, or takes the link back.
///
/// # Errors
///
/// As [`super::views::link`].
#[tauri::command]
pub async fn tasks_link(
    state: tauri::State<'_, Arc<AppState>>,
    task_id: String,
    other: String,
    kind: String,
    on: bool,
) -> AppResult<TaskDetail> {
    crate::ipc::blocking(state.inner(), "the task", move |state| {
        super::views::link(state, &task_id, &other, &kind, on)
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
