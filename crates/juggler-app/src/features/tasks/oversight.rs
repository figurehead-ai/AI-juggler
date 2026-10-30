//! D-44 (2026-10-27): the user's four actions on a task. Agents write and
//! run the tasks; the user sees and tracks them, and manages them through
//! the agents. In the app the user only:
//!
//! - **approves** a task its holder marked in review (it's done), or
//!   **sends it back** to in progress with a short note, which its holder
//!   gets as the user's message (E18: delivered at its next turn when it
//!   runs in AI Juggler, in its inbox otherwise) and reads in the task's
//!   history (`task_get`);
//! - **frees** a task whose holder's session ended or crashed (none of its
//!   sessions runs, as the sessions' own records say), so another agent
//!   can take it;
//! - **cancels** an open task;
//! - starts a session for a task ([`super::launch`], D-31).
//!
//! Each checks the task as it is when it's done (`TasksRepo::user_action`,
//! the rules in `juggler_core::tasks::may_user`), never only what the
//! window showed, and is logged in the task's history as the user's.

use std::sync::Arc;

use juggler_core::storage::{MemberRef, Task};
use juggler_core::tasks::{self, NOTE_MAX, UserAction};

use super::Actor;
use super::dto::TaskDetail;
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

fn refused(words: impl Into<String>) -> AppError {
    AppError::Refused(words.into())
}

/// Does `action` to a task as the user, against its holder's sessions as
/// they are now; the task as it is after.
fn act(
    state: &Arc<AppState>,
    task_id: &str,
    action: UserAction,
    note: Option<&str>,
) -> AppResult<Task> {
    let storage = lock_storage_arc(state)?;
    let repo = storage.tasks();
    let task = repo
        .get(task_id.trim())?
        .ok_or_else(|| refused("That task isn't there any more"))?;
    let holder = task.assignee_ref.as_deref();
    let holder_running =
        holder.is_some_and(|h| crate::features::messages::live_session_of(&storage, h).is_some());
    let changed = repo
        .user_action(
            &task.id,
            action,
            note,
            holder,
            holder_running,
            Actor::user().who(),
        )
        .map_err(|error| match error {
            juggler_core::CoreError::Validation { message, .. } => refused(message),
            other => other.into(),
        })?;
    drop(storage);
    state.tasks.notify(&changed.pool_id);
    Ok(changed)
}

/// The user approves a task in review: it's done.
///
/// # Errors
///
/// [`AppError::Refused`] for a task that isn't in review (or isn't there).
pub fn approve(state: &Arc<AppState>, task_id: &str) -> AppResult<TaskDetail> {
    let task = act(state, task_id, UserAction::Approve, None)?;
    super::views::detail(state, &task.id, None)
}

/// The user sends a task in review back to in progress, its holder kept,
/// with `note`: in its history, and to its holder as the user's message.
///
/// # Errors
///
/// [`AppError::Refused`] for an empty or too long note, or a task that
/// isn't in review.
pub fn send_back(state: &Arc<AppState>, task_id: &str, note: &str) -> AppResult<TaskDetail> {
    let note = tasks::clean_text(note, NOTE_MAX)
        .map_err(|_| refused(format!("a note is at most {NOTE_MAX} characters")))?;
    if note.trim().is_empty() {
        return Err(refused("Say what should change: a short note"));
    }
    let task = act(state, task_id, UserAction::SendBack, Some(&note))?;
    tell_holder(state, &task, &note);
    super::views::detail(state, &task.id, None)
}

/// The user frees a task whose holder's session ended or crashed: no one
/// holds it, so another agent can take it (in progress is ready again).
///
/// # Errors
///
/// [`AppError::Refused`] while a session of its holder runs, or when no
/// one holds it.
pub fn free(state: &Arc<AppState>, task_id: &str) -> AppResult<TaskDetail> {
    let task = act(state, task_id, UserAction::Free, None)?;
    super::views::detail(state, &task.id, None)
}

/// The user cancels an open task.
///
/// # Errors
///
/// [`AppError::Refused`] for a task that's done or cancelled already.
pub fn cancel(state: &Arc<AppState>, task_id: &str) -> AppResult<TaskDetail> {
    let task = act(state, task_id, UserAction::Cancel, None)?;
    super::views::detail(state, &task.id, None)
}

/// The send-back note reaches the task's holder the way the user's words
/// reach an agent (E18's compose, with Deliver: the user's click is the
/// permission, D-29): at its session's next turn when it runs in AI
/// Juggler, else in its inbox for whichever of its sessions reads it next.
/// A holder no longer in the pool reads it in the task's history.
fn tell_holder(state: &Arc<AppState>, task: &Task, note: &str) {
    let Some(holder) = task.assignee_ref.as_deref() else {
        return;
    };
    let anchor = lock_storage_arc(state).ok().and_then(|storage| {
        let works = storage.switch_graph().works_by_session().ok()?;
        Some(MemberRef::parse(holder)?.anchor(&works).to_wire())
    });
    let Some(anchor) = anchor else {
        return;
    };
    let Some(member) = crate::features::messages::roster(state, &task.pool_id)
        .ok()
        .and_then(|members| members.into_iter().find(|m| m.anchor == anchor))
    else {
        return;
    };
    let _ = crate::features::messages::views::compose(
        state,
        crate::features::messages::dto::ComposeInput {
            pool_id: task.pool_id.clone(),
            to: member.session_id,
            thread_id: None,
            kind: Some("note".to_owned()),
            // D-25: the subject by its title; the body's hint names the
            // id for the agent's own tool.
            subject: Some(format!(
                "Sent back · {}",
                juggler_core::messaging::one_line(&task.title, 80)
            )),
            body: format!(
                "{note}\n\n(The user sent {title} back to in progress: it's still yours. task_get {id} has it.)",
                title = super::words::quoted(&task.title),
                id = task.id
            ),
            deliver: true,
        },
    );
}
