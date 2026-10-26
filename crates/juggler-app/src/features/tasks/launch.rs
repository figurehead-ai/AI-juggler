//! AJ-E1903 Req.3 (D-31): "Start a session for this task" — only ever the
//! user's click, on the launch settings the user picks.
//!
//! The session starts the way every session the user starts does (the
//! project's folder, its runtime, its agent tools), with the task's brief
//! as its first prompt: a terminal launch takes it as the tool's own first
//! prompt; a session run in AI Juggler gets it as the user's message,
//! delivered between turns (E18). Then the task is the new session's, in
//! progress, and the session is linked to it ("started for"). A request
//! from an agent that waited in Needs you is closed with it.
//!
//! Nothing here picks the account or reads quota: `profile_id` is the
//! user's choice from the app's own command.

use std::sync::Arc;

use juggler_core::model::SessionId;
use juggler_core::storage::{TASK_USER, TaskChange};
use juggler_core::tasks::{self, TaskStatus};

use super::Actor;
use super::dto::TaskStartedInfo;
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The folder a session for a task works in: the pool's project, else
/// where the task's writer worked, else the launch settings' own.
fn folder_of(state: &Arc<AppState>, pool_id: &str, writer: &str) -> AppResult<Option<String>> {
    let storage = lock_storage_arc(state)?;
    let project = storage
        .pools()
        .get(pool_id)
        .ok()
        .and_then(|pool| pool.project);
    if let Some(project) = project
        && let Ok(record) = storage.projects().get(&project)
    {
        return Ok(Some(record.root.to_string_lossy().into_owned()));
    }
    Ok(writer
        .strip_prefix("session:")
        .and_then(|id| storage.sessions().get(&SessionId::new(id)).ok())
        .map(|record| record.cwd.to_string_lossy().into_owned()))
}

/// The task's brief, within what one message holds.
fn brief_of(task: &juggler_core::storage::Task, pool: &str) -> String {
    let agent = (task.created_by != TASK_USER).then_some(task.created_by_name.as_str());
    let full = tasks::brief(&tasks::Brief {
        id: &task.id,
        pool,
        title: &task.title,
        body: &task.body,
        acceptance: &task.acceptance,
        written_by_agent: agent,
    });
    if full.chars().count() <= tasks::BODY_MAX {
        return full;
    }
    let body: String = task.body.chars().take(tasks::BODY_MAX / 2).collect();
    let acceptance: String = task.acceptance.chars().take(tasks::BODY_MAX / 4).collect();
    tasks::brief(&tasks::Brief {
        id: &task.id,
        pool,
        title: &task.title,
        body: &format!("{body}… (the rest: the juggler tool task_get {})", task.id),
        acceptance: &acceptance,
        written_by_agent: agent,
    })
}

/// The user starts a session for a task on `profile_id` (their pick); it
/// closes `need` (an agent's request) when there was one.
///
/// # Errors
///
/// [`AppError::Refused`] for a task that's closed or gone, a need that
/// isn't open; the launch's own errors.
pub fn start(
    state: &Arc<AppState>,
    task_id: &str,
    profile_id: &str,
    need: Option<&str>,
) -> AppResult<TaskStartedInfo> {
    let task = lock_storage_arc(state)?
        .tasks()
        .get(task_id)?
        .ok_or_else(|| AppError::Refused("That task isn't there any more".to_owned()))?;
    if !task.status.is_open() {
        return Err(AppError::Refused(format!(
            "{} is {}: open it again first",
            task.id,
            task.status.words().to_lowercase()
        )));
    }
    if let Some(need) = need
        && lock_storage_arc(state)?.tasks().open_need(need)?.is_none()
    {
        return Err(AppError::Refused(
            "That request isn't waiting any more".to_owned(),
        ));
    }
    let pool_name = super::pool_name(&*lock_storage_arc(state)?, &task.pool_id)?;
    let brief = brief_of(&task, &pool_name);
    let cwd = folder_of(state, &task.pool_id, &task.created_by)?;
    let launched =
        crate::commands::launch_profile_primed(state, profile_id, cwd, None, Some(brief.clone()))?;
    let session = SessionId::new(launched.session_id.as_str());
    // The new session joins the task's pool (the user's click moves it).
    let in_pool = crate::features::pools::members::place_of(&*lock_storage_arc(state)?, &session)?
        .is_some_and(|(pool_id, _)| pool_id == task.pool_id);
    if !in_pool {
        crate::features::pools::pool_move_impl(state, session.as_str(), &task.pool_id)?;
    }
    if state.runtime.is_driven(&session) {
        // The user's first message, between turns (E18's delivery).
        let _ = crate::features::messages::views::compose(
            state,
            crate::features::messages::dto::ComposeInput {
                pool_id: task.pool_id.clone(),
                to: session.to_string(),
                thread_id: None,
                kind: Some("handoff".to_owned()),
                subject: Some(format!("{} · {}", task.id, task.title)),
                body: brief,
                deliver: true,
            },
        );
    }
    let name = crate::features::messages::roster(state, &task.pool_id)
        .ok()
        .and_then(|members| {
            members
                .into_iter()
                .find(|m| m.session_id == session.as_str())
        })
        .map_or_else(|| "The new session".to_owned(), |m| m.name);
    let member = format!("session:{}", session.as_str());
    let you = Actor::user();
    let profile = lock_storage_arc(state)?
        .profiles()
        .get(&juggler_core::model::ProfileId::new(profile_id))
        .map(|p| p.name)
        .unwrap_or_default();
    let task = {
        let storage = lock_storage_arc(state)?;
        let repo = storage.tasks();
        repo.assign(&task.id, Some((&member, &name)), you.who())?;
        let task = repo.update(
            &task.id,
            &TaskChange {
                status: Some(TaskStatus::InProgress),
                ..TaskChange::default()
            },
            you.who(),
        )?;
        repo.link_session(&task.id, session.as_str(), None, "started_for")?;
        repo.log(
            &task.pool_id,
            &task.id,
            you.who(),
            "session_started",
            Some(&format!("{name} on {profile}")),
        )?;
        if let Some(need) = need {
            repo.close_need(need, "started", Some(session.as_str()))?;
        }
        super::views::info(&storage, &task)
    };
    state.tasks.notify(&task.pool_id);
    Ok(TaskStartedInfo {
        session_id: session.to_string(),
        task,
    })
}
