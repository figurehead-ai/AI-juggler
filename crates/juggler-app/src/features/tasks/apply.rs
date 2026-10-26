//! The task operations, one place for all of them: an agent's call (after
//! `allowed`), the user's answer to a held ask, and the user's own edits.
//! Each checks the task rules for its actor and writes through the store,
//! which logs it. None of them decides whether an agent may: that is
//! `allowed(pool, action)`'s, in [`super::agent`].

use std::sync::Arc;

use juggler_agentbridge::wire::{TaskCreateArgs, TaskUpdateArgs};
use juggler_core::storage::{Claim, NewTask, Task, TaskChange, TaskNeed};
use juggler_core::tasks::{
    self, ACCEPTANCE_MAX, BODY_MAX, Change, LinkKind, NOTE_MAX, Priority, TaskKind, TaskStatus,
    TextError,
};

use super::Actor;
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

fn refused(words: impl Into<String>) -> AppError {
    AppError::Refused(words.into())
}

fn text(what: &str, value: &str, max: usize) -> AppResult<String> {
    tasks::clean_text(value, max)
        .map_err(|_| refused(format!("{what} is at most {max} characters")))
}

fn title(value: &str) -> AppResult<String> {
    tasks::clean_title(value).map_err(|error| match error {
        TextError::Empty => refused("a task needs a title"),
        TextError::TooLong => refused(format!(
            "a title is one line of at most {} characters",
            tasks::TITLE_MAX
        )),
    })
}

fn priority(word: Option<&str>) -> AppResult<Option<Priority>> {
    word.map(|word| {
        Priority::parse(word).ok_or_else(|| {
            refused(format!(
                "{word} isn't a priority (urgent, high, normal or low)"
            ))
        })
    })
    .transpose()
}

fn status(word: Option<&str>) -> AppResult<Option<TaskStatus>> {
    word.map(|word| {
        TaskStatus::parse(word).ok_or_else(|| {
            refused(format!(
                "{word} isn't a status (todo, ready, in_progress, blocked, in_review, done or cancelled)"
            ))
        })
    })
    .transpose()
}

/// A link as a change: `None` keeps it, `Some(None)` removes it (as
/// `TaskChange::external_url` takes it).
#[allow(clippy::option_option)]
fn url(value: Option<&str>) -> AppResult<Option<Option<String>>> {
    match value.map(str::trim) {
        None => Ok(None),
        Some("") => Ok(Some(None)),
        Some(link) => tasks::external_link(link)
            .map(|link| Some(Some(link.url)))
            .map_err(refused),
    }
}

/// A task of `pool`, or why not.
pub(crate) fn task_in(state: &Arc<AppState>, pool_id: &str, id: &str) -> AppResult<Task> {
    lock_storage_arc(state)?
        .tasks()
        .get(id.trim())?
        .filter(|task| task.pool_id == pool_id)
        .ok_or_else(|| refused(format!("there's no task {} in this pool", id.trim())))
}

fn blockers(list: &[String]) -> AppResult<Vec<String>> {
    if list.len() > tasks::LINKS_MAX {
        return Err(refused(format!(
            "at most {} blockers at once",
            tasks::LINKS_MAX
        )));
    }
    Ok(list.iter().map(|id| id.trim().to_owned()).collect())
}

/// Adds a task to `pool`, made by `actor` (ready unless it says to do,
/// and no one's).
///
/// # Errors
///
/// [`AppError::Refused`] in words: a rule (an epic or a story is the
/// lead's), a field that isn't taken, a parent or blocker not in the pool.
pub fn create(
    state: &Arc<AppState>,
    actor: &Actor,
    pool_id: &str,
    args: &TaskCreateArgs,
) -> AppResult<Task> {
    let kind = match args.kind.as_deref() {
        None => TaskKind::Task,
        Some(word) => TaskKind::parse(word)
            .ok_or_else(|| refused(format!("{word} isn't a kind (task, story or epic)")))?,
    };
    tasks::may_plan(actor.role, kind).map_err(refused)?;
    let start = status(args.status.as_deref())?.unwrap_or(TaskStatus::Ready);
    if !matches!(start, TaskStatus::Ready | TaskStatus::Todo) {
        return Err(refused("a new task starts ready or to do"));
    }
    let title = title(&args.title)?;
    let body = text("a body", args.body.as_deref().unwrap_or_default(), BODY_MAX)?;
    let acceptance = text(
        "acceptance criteria",
        args.acceptance.as_deref().unwrap_or_default(),
        ACCEPTANCE_MAX,
    )?;
    let priority = priority(args.priority.as_deref())?.unwrap_or(Priority::Normal);
    let link = url(args.url.as_deref())?.flatten();
    let blocked_by = blockers(&args.blocked_by)?;
    let parent = args
        .parent
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty());
    let task = {
        let storage = lock_storage_arc(state)?;
        let id = super::new_short_id(&storage)?;
        storage
            .tasks()
            .create(&NewTask {
                id: &id,
                pool_id,
                kind,
                parent_id: parent,
                title: &title,
                body: &body,
                acceptance: &acceptance,
                priority,
                status: start,
                assignee: None,
                created_by: &actor.member_ref,
                created_by_name: &actor.name,
                external_url: link.as_deref(),
                blocked_by: &blocked_by,
            })
            .map_err(|error| match error {
                juggler_core::CoreError::Validation { message, .. } => refused(message),
                other => other.into(),
            })?
    };
    state.tasks.notify(pool_id);
    Ok(task)
}

/// Takes a ready task for `actor` (an agent: the user assigns instead).
///
/// # Errors
///
/// Storage failures.
pub fn claim(state: &Arc<AppState>, actor: &Actor, pool_id: &str, id: &str) -> AppResult<Claim> {
    let claim = {
        let storage = lock_storage_arc(state)?;
        let works = storage.switch_graph().works_by_session()?;
        let session = actor.session.as_ref().map(|session| {
            (
                session.as_str().to_owned(),
                works.get(session).map(|w| w.as_str().to_owned()),
            )
        });
        storage.tasks().claim(
            pool_id,
            id.trim(),
            &actor.refs,
            actor.who(),
            session
                .as_ref()
                .map(|(session, work)| (session.as_str(), work.as_deref())),
        )?
    };
    if matches!(claim, Claim::Won(_)) {
        state.tasks.notify(pool_id);
    }
    Ok(claim)
}

/// What a change asks, for the rules.
fn changes_of(args: &TaskUpdateArgs) -> AppResult<Vec<Change>> {
    let mut changes = Vec::new();
    if args.title.is_some()
        || args.body.is_some()
        || args.acceptance.is_some()
        || args.priority.is_some()
        || !args.blocked_by.is_empty()
        || args.url.is_some()
        || (args.note.is_some() && !args.release && args.status.is_none())
    {
        changes.push(Change::Edit);
    }
    if let Some(to) = status(args.status.as_deref())? {
        changes.push(Change::Status(to));
    }
    if args.release {
        changes.push(Change::Release);
    }
    if changes.is_empty() {
        return Err(refused(
            "say what changes: a status, release, a note or a field",
        ));
    }
    Ok(changes)
}

/// Whether `actor` may make the changes `args` asks to `task`.
///
/// # Errors
///
/// [`AppError::Refused`] with the rule, in words.
pub fn may_update(actor: &Actor, task: &Task, args: &TaskUpdateArgs) -> AppResult<()> {
    let holds = task
        .assignee_ref
        .as_ref()
        .is_some_and(|holder| actor.refs.contains(holder) || *holder == actor.member_ref);
    let made_it = actor.refs.contains(&task.created_by) || task.created_by == actor.member_ref;
    for change in changes_of(args)? {
        tasks::may_change(
            actor.role,
            holds,
            made_it,
            task.assignee_ref.is_some(),
            change,
        )
        .map_err(refused)?;
    }
    Ok(())
}

/// Changes a task: its status, its words, its priority, blockers or
/// link; or gives it back (with a note for whoever takes it next).
///
/// # Errors
///
/// [`AppError::Refused`] in words: a rule, a field that isn't taken.
pub fn update(
    state: &Arc<AppState>,
    actor: &Actor,
    pool_id: &str,
    args: &TaskUpdateArgs,
) -> AppResult<Task> {
    let task = task_in(state, pool_id, &args.task)?;
    may_update(actor, &task, args)?;
    let title = args.title.as_deref().map(title).transpose()?;
    let body = args
        .body
        .as_deref()
        .map(|b| text("a body", b, BODY_MAX))
        .transpose()?;
    let acceptance = args
        .acceptance
        .as_deref()
        .map(|a| text("acceptance criteria", a, ACCEPTANCE_MAX))
        .transpose()?;
    let note = args
        .note
        .as_deref()
        .map(|n| text("a note", n, NOTE_MAX))
        .transpose()?
        .filter(|n| !n.is_empty());
    let link = url(args.url.as_deref())?;
    let blocked_by = blockers(&args.blocked_by)?;
    let to = status(args.status.as_deref())?;
    let storage = lock_storage_arc(state)?;
    let repo = storage.tasks();
    for blocker in &blocked_by {
        repo.link(blocker, &task.id, LinkKind::Blocks, actor.who())
            .map_err(|error| match error {
                juggler_core::CoreError::Validation { message, .. } => refused(message),
                other => other.into(),
            })?;
    }
    let changed = repo
        .update(
            &task.id,
            &TaskChange {
                title: title.as_deref(),
                body: body.as_deref(),
                acceptance: acceptance.as_deref(),
                priority: priority(args.priority.as_deref())?,
                status: to,
                parent: None,
                external_url: link.as_ref().map(Option::as_deref),
                release: args.release,
                note: note.as_deref(),
            },
            actor.who(),
        )
        .map_err(|error| match error {
            juggler_core::CoreError::Validation { message, .. } => refused(message),
            other => other.into(),
        })?;
    if let Some(session) = &actor.session {
        let works = storage.switch_graph().works_by_session()?;
        let _ = repo.link_session(
            &task.id,
            session.as_str(),
            works
                .get(session)
                .map(juggler_core::model::WorkItemId::as_str),
            "worked",
        );
    }
    drop(storage);
    state.tasks.notify(pool_id);
    Ok(changed)
}

/// Who a task goes to.
#[derive(Clone, Debug)]
pub enum Assignee {
    /// A member of the pool: its session (as the roster names it).
    Member(String),
    /// No one.
    Nobody,
}

/// Gives a task to a member of its pool, or to no one.
///
/// # Errors
///
/// [`AppError::Refused`] in words: not the lead's call, a session that
/// isn't in the pool.
pub fn assign(
    state: &Arc<AppState>,
    actor: &Actor,
    pool_id: &str,
    id: &str,
    to: &Assignee,
) -> AppResult<Task> {
    if actor.role != tasks::Role::Lead {
        return Err(refused(
            "only the pool's lead gives tasks to others: ask the lead (send_message), or give yours back with task_update release",
        ));
    }
    let task = task_in(state, pool_id, id)?;
    let member = match to {
        Assignee::Nobody => None,
        Assignee::Member(session) => Some(
            crate::features::messages::roster(state, pool_id)?
                .into_iter()
                .find(|m| m.session_id == session.trim())
                .ok_or_else(|| {
                    refused(format!(
                        "{} isn't in this pool: use a session_id from pool_roster",
                        session.trim()
                    ))
                })?,
        ),
    };
    let storage = lock_storage_arc(state)?;
    let repo = storage.tasks();
    let changed = repo.assign(
        &task.id,
        member
            .as_ref()
            .map(|m| (m.anchor.as_str(), m.name.as_str())),
        actor.who(),
    )?;
    if let Some(member) = &member {
        let works = storage.switch_graph().works_by_session()?;
        let session = juggler_core::model::SessionId::new(member.session_id.as_str());
        let _ = repo.link_session(
            &task.id,
            &member.session_id,
            works
                .get(&session)
                .map(juggler_core::model::WorkItemId::as_str),
            "assigned",
        );
    }
    drop(storage);
    state.tasks.notify(pool_id);
    Ok(changed)
}

/// An agent asks the user for a new session for a task (D-31: the user
/// decides, picks the launch settings and starts it). The same request
/// from the same member isn't asked twice. The need's id.
///
/// # Errors
///
/// [`AppError::Refused`] for a task that's closed or not in the pool.
pub fn ask_for_session(
    state: &Arc<AppState>,
    actor: &Actor,
    pool_id: &str,
    id: &str,
    note: Option<&str>,
) -> AppResult<String> {
    let task = task_in(state, pool_id, id)?;
    if !task.status.is_open() {
        return Err(refused(format!(
            "{} is {}: there's nothing to start a session for",
            task.id,
            task.status.words().to_lowercase()
        )));
    }
    let note = note
        .map(|n| text("a note", n, NOTE_MAX))
        .transpose()?
        .filter(|n| !n.is_empty());
    let action = juggler_core::agent_access::AgentAction::PrepareLaunch.as_str();
    let storage = lock_storage_arc(state)?;
    let repo = storage.tasks();
    if let Some(open) = repo.need_open_for(&actor.member_ref, action, Some(&task.id))? {
        return Ok(open);
    }
    let need = crate::features::messages::new_id("tneed");
    repo.insert_need(&TaskNeed {
        id: need.clone(),
        pool_id: pool_id.to_owned(),
        task_id: Some(task.id.clone()),
        kind: "launch".to_owned(),
        action: action.to_owned(),
        actor_ref: actor.member_ref.clone(),
        actor_name: actor.name.clone(),
        change: note.clone(),
        summary: format!(
            "«{}» asks for a new session for {} «{}»",
            actor.name, task.id, task.title
        ),
        created_at: juggler_core::model::Timestamp::now().millis(),
    })?;
    repo.log(
        pool_id,
        &task.id,
        actor.who(),
        "asked_for_session",
        note.as_deref(),
    )?;
    drop(storage);
    state.tasks.notify(pool_id);
    Ok(need)
}
