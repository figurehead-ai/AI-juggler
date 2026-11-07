//! AJ-E1903 Req.3 (D-31): "Start a session for this task" — only ever the
//! user's click, on the launch settings the user picks.
//!
//! The session starts the way every session the user starts does (the
//! project's folder, its runtime, its agent tools). D-72: its first prompt
//! is a pointer to the task, never the brief (a terminal launch takes its
//! first prompt on the command line, where other programs can read it):
//! with the agent tools it reads the brief through them; without, from a
//! private copy only this user can open. A session run in AI Juggler still
//! gets the whole brief as the user's message, delivered between turns
//! (E18), which no command line carries. Then the task is the new session's, in
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
    Ok(folder_in(&*lock_storage_arc(state)?, pool_id, writer))
}

/// The folder task `task_id`'s session would start in ([`start`]'s
/// choice), `None` for a task that isn't there or has no folder. The lab
/// write mode holds a task's start to a folder inside the lab.
pub(crate) fn task_folder(
    storage: &juggler_core::storage::Storage,
    task_id: &str,
) -> Option<String> {
    let task = storage.tasks().get(task_id).ok().flatten()?;
    folder_in(storage, &task.pool_id, &task.created_by)
}

fn folder_in(
    storage: &juggler_core::storage::Storage,
    pool_id: &str,
    writer: &str,
) -> Option<String> {
    let project = storage
        .pools()
        .get(pool_id)
        .ok()
        .and_then(|pool| pool.project);
    if let Some(project) = project
        && let Ok(record) = storage.projects().get(&project)
    {
        return Some(record.root.to_string_lossy().into_owned());
    }
    writer
        .strip_prefix("session:")
        .and_then(|id| storage.sessions().get(&SessionId::new(id)).ok())
        .map(|record| record.cwd.to_string_lossy().into_owned())
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

/// The folder a task's private brief is kept in when its session has no
/// agent tools (D-72).
const BRIEFS: &str = "task-briefs";

/// AJ-E1907 R2: how long a private brief stays after it's written. Its
/// session reads it at its first turn (seconds; minutes while a tool's own
/// permission prompt waits), so an hour is long past that.
const BRIEF_KEEP_MS: i64 = 60 * 60 * 1000;

/// AJ-E1907 R2: removes the private briefs read long enough ago, and
/// those whose task is closed or gone. At start, on the quota tick and
/// before each new brief is written.
pub fn sweep_briefs(state: &AppState, now_ms: i64) -> usize {
    let open = |id: &str| {
        state.storage.lock().is_ok_and(|storage| {
            storage
                .tasks()
                .get(id)
                .ok()
                .flatten()
                .is_some_and(|task| task.status.is_open())
        })
    };
    sweep_dir(&state.data_dir.join(BRIEFS), now_ms, &open)
}

/// [`sweep_briefs`] over `dir`, with `open` telling an open task's id.
fn sweep_dir(dir: &std::path::Path, now_ms: i64, open: &dyn Fn(&str) -> bool) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(id) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".md"))
        else {
            continue;
        };
        let written_ms = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|since| i64::try_from(since.as_millis()).ok())
            .unwrap_or(0);
        let read_long_ago = now_ms.saturating_sub(written_ms) >= BRIEF_KEEP_MS;
        if (read_long_ago || !open(id)) && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// D-72: the first prompt of a session started for `task` on `profile_id`:
/// a pointer to the task. With the agent tools (on for its project and
/// account), it reads the brief through them; without, the brief is
/// written to a private file (this user only) and the pointer names it.
fn first_prompt(
    state: &Arc<AppState>,
    task: &juggler_core::storage::Task,
    pool: &str,
    profile_id: &str,
    brief: &str,
) -> AppResult<String> {
    let (project, account) = {
        let storage = lock_storage_arc(state)?;
        let project = storage
            .pools()
            .get(&task.pool_id)
            .ok()
            .and_then(|pool| pool.project)
            .map(|id| id.to_string());
        let account = storage
            .profiles()
            .get(&juggler_core::model::ProfileId::new(profile_id))
            .map(|recipe| recipe.account.to_string())?;
        (project, account)
    };
    if crate::features::agent_bridge::tools_offered(state, project.as_deref(), &account) {
        return Ok(tasks::pointer(&task.id, pool, &task.title, None));
    }
    sweep_briefs(state, crate::commands::now_millis());
    let dir = state.data_dir.join(BRIEFS);
    crate::features::agent_bridge::own_dir(&dir)
        .map_err(|error| AppError::io("the task's brief", &error))?;
    let path = dir.join(format!("{}.md", task.id));
    crate::features::agent_bridge::write_private(&path, brief)
        .map_err(|error| AppError::io("the task's brief", &error))?;
    Ok(tasks::pointer(
        &task.id,
        pool,
        &task.title,
        Some(&path.to_string_lossy()),
    ))
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
    let first = first_prompt(state, &task, &pool_name, profile_id, &brief)?;
    let launched =
        crate::features::launch::launch_profile_primed(state, profile_id, cwd, None, Some(first))?;
    let session = SessionId::new(launched.session_id.as_str());
    // The new session joins the task's pool (the user's click names it
    // there; D-62: its other pools stay).
    let in_pool = crate::features::pools::members::role_in(
        &*lock_storage_arc(state)?,
        &session,
        &task.pool_id,
    )?
    .is_some();
    if !in_pool {
        crate::features::pools::model::join_impl(
            state,
            &task.pool_id,
            &format!("session:{}", session.as_str()),
            "member",
            None,
            &[],
        )?;
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
                // D-25: its title on screen; the brief names its id for the agent.
                subject: Some(juggler_core::messaging::one_line(&task.title, 80)),
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

#[cfg(test)]
mod brief_tests {
    use super::*;

    /// AJ-E1907 R2: a brief goes once it's been read (an hour on), or at
    /// once when its task closed; an open task's fresh brief stays.
    #[test]
    fn a_brief_is_removed_after_its_session_read_it_or_its_task_closed() {
        let dir = tempfile::tempdir().unwrap();
        for id in ["t-open", "t-closed"] {
            std::fs::write(dir.path().join(format!("{id}.md")), "brief").unwrap();
        }
        std::fs::write(dir.path().join("notes.txt"), "not a brief").unwrap();
        let now = crate::commands::now_millis();
        let open = |id: &str| id == "t-open";
        assert_eq!(sweep_dir(dir.path(), now, &open), 1, "the closed task's");
        assert!(dir.path().join("t-open.md").exists());
        assert!(!dir.path().join("t-closed.md").exists());
        assert_eq!(
            sweep_dir(dir.path(), now + BRIEF_KEEP_MS, &open),
            1,
            "read by now"
        );
        assert!(!dir.path().join("t-open.md").exists());
        assert!(dir.path().join("notes.txt").exists(), "only briefs");
        assert_eq!(sweep_dir(&dir.path().join("missing"), now, &open), 0);
    }
}
