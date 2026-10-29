//! AJ-E1903 (D-44): the user sees and tracks the agents' work — a pool's
//! list (ready work first, a page at a time), the epic and story tree
//! behind a fold, a session's tasks, and one task in full with its
//! holder, sessions, messages, history and the actions it offers now.
//! Reads only: agents write the tasks; the user's four actions are
//! [`super::oversight`]'s.

use std::collections::HashMap;
use std::sync::Arc;

use juggler_core::model::SessionId;
use juggler_core::storage::{Storage, TASK_USER, Task, TaskKey, TaskQuery};
use juggler_core::tasks::{TaskKind, TaskStatus, UserAction, may_user};

use super::dto::{
    TaskActions, TaskCounts, TaskDetail, TaskEventInfo, TaskInfo, TaskIssueInfo, TaskLinkInfo,
    TaskSessionInfo, TaskThreadInfo,
};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::paging::{Page, PageRequest, SortKey, page, page_of_fetched};
use crate::state::AppState;

/// The items a task's page shows under it.
pub const PAGE: u32 = 40;
/// A page of a task's history.
pub const HISTORY: u32 = 20;

/// A task as a row: names as the pool shows them, the session its holder
/// runs in now.
pub(crate) fn info(storage: &Storage, task: &Task) -> TaskInfo {
    let holder_session_id = task
        .assignee_ref
        .as_deref()
        .and_then(|holder| crate::features::messages::live_session_of(storage, holder))
        .map(|session| session.to_string());
    let you = |who: &str, name: &str| {
        if who == TASK_USER {
            super::YOU.to_owned()
        } else {
            name.to_owned()
        }
    };
    TaskInfo {
        id: task.id.clone(),
        pool_id: task.pool_id.clone(),
        kind: task.kind.as_str().to_owned(),
        parent_id: task.parent_id.clone(),
        title: task.title.clone(),
        status: task.status.as_str().to_owned(),
        stored_status: (task.stored != task.status).then(|| task.stored.as_str().to_owned()),
        priority: task.priority.as_str().to_owned(),
        holder: task
            .assignee_ref
            .as_deref()
            .map(|who| you(who, task.assignee_name.as_deref().unwrap_or("someone"))),
        holder_session_id,
        blocked_by: task.blocked_by.clone(),
        items: task.children.0,
        items_done: task.children.1,
        written_by: you(&task.created_by, &task.created_by_name),
        by_agent: task.created_by != TASK_USER,
        updated_at_ms: task.updated_at,
        issue_label: task
            .external
            .as_ref()
            .and_then(|x| juggler_core::tasks::external_link(&x.url).ok())
            .and_then(|link| link.label),
    }
}

/// A task's place as the IPC's keyset cursor names it (`paging.rs`).
pub(crate) fn sort_key(task: &Task) -> SortKey {
    let key = task.key();
    SortKey::new(
        [i64::from(key.rank), i64::from(key.priority), -key.seq],
        task.id.clone(),
    )
}

/// The task a cursor starts after.
pub(crate) fn after_of(request: &PageRequest) -> AppResult<Option<TaskKey>> {
    Ok(request
        .start_after()?
        .and_then(|key| match key.ranks.as_slice() {
            [rank, priority, seq] => Some(TaskKey {
                rank: u8::try_from(*rank).ok()?,
                priority: u8::try_from(*priority).ok()?,
                seq: -seq,
            }),
            _ => None,
        }))
}

/// One page of `query` (after the request's cursor), as rows.
fn page_of(
    state: &Arc<AppState>,
    query: TaskQuery<'_>,
    request: &PageRequest,
) -> AppResult<Page<TaskInfo>> {
    let limit = request.limit();
    let storage = lock_storage_arc(state)?;
    let rows = storage.tasks().list(
        &TaskQuery {
            after: after_of(request)?,
            ..query
        },
        u32::try_from(limit + 1).unwrap_or(u32::MAX),
    )?;
    let fetched = rows
        .iter()
        .map(|task| (sort_key(task), info(&storage, task)))
        .collect();
    Ok(page_of_fetched(fetched, limit, None))
}

/// One page of a pool's tasks (the work; epics and stories are the plan):
/// `filter` is `open` (the default: every open one, ready work first),
/// `closed`, `mine` (yours), `all`, or one status.
///
/// # Errors
///
/// Validation for an unknown filter or cursor; storage failures.
pub fn list(
    state: &Arc<AppState>,
    pool_id: &str,
    filter: Option<&str>,
    words: Option<&str>,
    request: &PageRequest,
) -> AppResult<Page<TaskInfo>> {
    let open: Vec<TaskStatus> = TaskStatus::ALL
        .into_iter()
        .filter(|s| s.is_open())
        .collect();
    let closed = [TaskStatus::Done, TaskStatus::Cancelled];
    let you = [TASK_USER.to_owned()];
    let (statuses, assignees): (Option<Vec<TaskStatus>>, Option<&[String]>) = match filter {
        None | Some("open") => (Some(open), None),
        Some("closed") => (Some(closed.to_vec()), None),
        Some("mine") => (Some(open), Some(&you)),
        Some("all") => (None, None),
        Some(word) => (
            Some(vec![TaskStatus::parse(word).ok_or_else(|| {
                AppError::Validation {
                    subject: "filter",
                    message: format!("{word} isn't a filter"),
                }
            })?]),
            None,
        ),
    };
    page_of(
        state,
        TaskQuery {
            pool_id: Some(pool_id),
            assignees,
            statuses: statuses.as_deref(),
            // AJ-E1903: the list is the work; epics and stories are the
            // plan, behind its own fold.
            kinds: Some(&[TaskKind::Task]),
            words: words.filter(|w| !w.trim().is_empty()),
            ..TaskQuery::default()
        },
        request,
    )
}

/// How many of a pool's tasks (the work) are in each honest status: the
/// filter's numbers.
///
/// # Errors
///
/// Storage failures.
pub fn counts(state: &Arc<AppState>, pool_id: &str) -> AppResult<TaskCounts> {
    let mut out = TaskCounts::default();
    for (status, count) in lock_storage_arc(state)?
        .tasks()
        .counts(pool_id, Some(&[TaskKind::Task]))?
    {
        *slot(&mut out, status) = count;
    }
    Ok(out)
}

/// The count `status` goes in.
pub(crate) fn slot(out: &mut TaskCounts, status: TaskStatus) -> &mut u32 {
    match status {
        TaskStatus::Ready => &mut out.ready,
        TaskStatus::InProgress => &mut out.in_progress,
        TaskStatus::InReview => &mut out.in_review,
        TaskStatus::Blocked => &mut out.blocked,
        TaskStatus::Todo => &mut out.todo,
        TaskStatus::Done => &mut out.done,
        TaskStatus::Cancelled => &mut out.cancelled,
    }
}

/// One page of the pool's plan: its epics and stories (the tree's folds).
///
/// # Errors
///
/// Validation for a cursor this app didn't make; storage failures.
pub fn plan(
    state: &Arc<AppState>,
    pool_id: &str,
    request: &PageRequest,
) -> AppResult<Page<TaskInfo>> {
    page_of(
        state,
        TaskQuery {
            pool_id: Some(pool_id),
            kinds: Some(&[TaskKind::Epic, TaskKind::Story]),
            ..TaskQuery::default()
        },
        request,
    )
}

/// One page of the items under an epic or story (`""`: tasks with none).
///
/// # Errors
///
/// Validation for a cursor this app didn't make; storage failures.
pub fn children(
    state: &Arc<AppState>,
    pool_id: &str,
    parent: &str,
    request: &PageRequest,
) -> AppResult<Page<TaskInfo>> {
    page_of(
        state,
        TaskQuery {
            pool_id: Some(pool_id),
            parent: Some(parent),
            kinds: (parent.is_empty()).then_some(&[TaskKind::Task][..]),
            ..TaskQuery::default()
        },
        request,
    )
}

/// One page of a session's tasks: what it holds (any session of its Work
/// item) and what it worked, open ones first.
///
/// # Errors
///
/// Validation for a cursor this app didn't make; storage failures.
pub fn of_session(
    state: &Arc<AppState>,
    session_id: &str,
    request: &PageRequest,
) -> AppResult<Page<TaskInfo>> {
    let storage = lock_storage_arc(state)?;
    let session = SessionId::new(session_id);
    let refs = crate::features::messages::refs_of(&storage, &session);
    let repo = storage.tasks();
    let mut rows = repo.list(
        &TaskQuery {
            assignees: Some(&refs),
            ..TaskQuery::default()
        },
        200,
    )?;
    let sessions: Vec<String> = refs
        .iter()
        .filter_map(|r| r.strip_prefix("session:").map(str::to_owned))
        .collect();
    for (_, id, _, _) in repo.of_sessions(&sessions)? {
        if !rows.iter().any(|t| t.id == id)
            && let Some(task) = repo.get(&id)?
        {
            rows.push(task);
        }
    }
    let keyed: Vec<(SortKey, TaskInfo)> = rows
        .iter()
        .map(|task| (sort_key(task), info(&storage, task)))
        .collect();
    page(keyed, |(key, _)| key.clone(), request).map(|page| page.map(|(_, row)| row))
}

/// AJ-E1904: a switch history node's task, as `aj-7f3c · title`: the
/// task its session worked (the newest), else — for the line of work's
/// newest session — the one its Work item holds now (a switch keeps it).
#[must_use]
pub fn epoch_task(storage: &Storage, session: &str, newest: bool) -> Option<String> {
    let repo = storage.tasks();
    if let Some((_, id, title, _)) = repo
        .of_sessions(&[session.to_owned()])
        .ok()?
        .into_iter()
        .next()
    {
        return Some(format!("{id} · {title}"));
    }
    if !newest {
        return None;
    }
    let refs = crate::features::messages::refs_of(storage, &SessionId::new(session));
    repo.held_by(&refs)
        .ok()
        .flatten()
        .map(|task| format!("{} · {}", task.id, task.title))
}

/// An event in words.
fn what(verb: &str, detail: Option<&str>) -> String {
    let detail = detail.unwrap_or_default();
    match verb {
        "created" => "added it".to_owned(),
        "claimed" => "took it".to_owned(),
        "released" if detail.is_empty() => "gave it back".to_owned(),
        "released" => format!("gave it back: {detail}"),
        "assigned" => format!("gave it to {detail}"),
        "unassigned" => "gave it to no one".to_owned(),
        "status" => format!("moved it: {detail}"),
        "edited" => format!("changed its {detail}"),
        "noted" => format!("noted: {detail}"),
        "linked" => format!("linked {detail}"),
        "unlinked" => format!("unlinked {detail}"),
        "parent" => format!("moved it under {detail}"),
        "linked_issue" => format!("linked the issue {detail}"),
        "held" => format!("asked to {detail} (waits on your OK)"),
        "allowed" => format!("— you allowed: {detail}"),
        "refused" => format!("— you said no: {detail}"),
        "asked_for_session" if detail.is_empty() => "asked you for a new session for it".to_owned(),
        "asked_for_session" => format!("asked you for a new session for it: {detail}"),
        "session_started" => format!("started a session for it: {detail}"),
        "approved" => "approved it: done".to_owned(),
        "sent_back" => format!("sent it back: {detail}"),
        "freed" if detail.is_empty() => "freed it: its session had ended".to_owned(),
        "freed" => format!("freed it from {detail}, whose session had ended"),
        "cancelled" if detail.is_empty() => "cancelled it".to_owned(),
        "cancelled" => format!("cancelled it: {detail}"),
        "issue_looked_up" => format!("looked up the issue: {detail}"),
        // AJ-E1904: "Planner continued on Personal (from Work Max)".
        "switched" => format!("continued on {detail}"),
        other => other.replace('_', " "),
    }
}

/// The issue a task comes from, as the user sees it.
fn issue_of(state: &Arc<AppState>, task: &Task) -> Option<TaskIssueInfo> {
    task.external.as_ref().map(|x| {
        let link = juggler_core::tasks::external_link(&x.url).ok();
        let gh = link
            .as_ref()
            .is_some_and(|l| l.github.is_some())
            .then(|| super::issues::gh_or_why(state));
        TaskIssueInfo {
            url: x.url.clone(),
            service: link.as_ref().map_or("web", |l| l.service).to_owned(),
            label: link.as_ref().and_then(|l| l.label.clone()),
            title: x.title.clone(),
            state: x.state.clone(),
            checked_at_ms: x.checked_at,
            can_look_up: gh.as_ref().is_some_and(Result::is_ok),
            look_up_note: gh.and_then(Result::err),
        }
    })
}

/// The sessions that worked a task, by name, and whether each runs.
fn sessions_of(
    storage: &Storage,
    task_id: &str,
    names: &HashMap<String, String>,
) -> AppResult<Vec<TaskSessionInfo>> {
    let live: std::collections::HashSet<String> = storage
        .sessions()
        .live()
        .unwrap_or_default()
        .into_iter()
        .filter(|r| crate::features::messages::is_live(r.status))
        .map(|r| r.id.to_string())
        .collect();
    Ok(storage
        .tasks()
        .sessions_of(task_id)?
        .into_iter()
        .map(|s| TaskSessionInfo {
            name: names
                .get(&s.session_id)
                .cloned()
                .unwrap_or_else(|| "A session that ended".to_owned()),
            live: live.contains(&s.session_id),
            session_id: s.session_id,
            how: s.how,
            at_ms: s.linked_at,
        })
        .collect())
}

/// A page of history in words, and the cursor for older events.
fn history_of(events: &[juggler_core::storage::TaskEvent]) -> (Vec<TaskEventInfo>, Option<String>) {
    let next = (events.len() > HISTORY as usize)
        .then(|| {
            events
                .get(HISTORY as usize - 1)
                .map(|e| format!("h{}", e.id))
        })
        .flatten();
    let page = events
        .iter()
        .take(HISTORY as usize)
        .map(|e| TaskEventInfo {
            id: e.id,
            at_ms: e.at,
            who: if e.actor_ref == TASK_USER {
                super::YOU.to_owned()
            } else {
                e.actor_name.clone()
            },
            by_agent: e.actor_ref != TASK_USER,
            what: what(&e.verb, e.detail.as_deref()),
        })
        .collect();
    (page, next)
}

/// What the user may do to `task` now (D-44): the same rules the actions
/// check when they're done. `holder_running`: a session of its holder
/// runs.
pub(crate) fn actions_of(task: &Task, holder_running: bool) -> TaskActions {
    let may = |action| {
        may_user(
            action,
            task.stored,
            task.assignee_ref.is_some(),
            holder_running,
        )
        .is_ok()
    };
    TaskActions {
        review: may(UserAction::Approve),
        free: may(UserAction::Free),
        cancel: may(UserAction::Cancel),
    }
}

/// One task in full, with a page of its history (`cursor`: older).
///
/// # Errors
///
/// [`AppError::Refused`] for a task that isn't there; storage failures.
pub fn detail(state: &Arc<AppState>, task_id: &str, cursor: Option<&str>) -> AppResult<TaskDetail> {
    let names: HashMap<String, String> = {
        let pool = lock_storage_arc(state)?
            .tasks()
            .get(task_id)?
            .map(|t| t.pool_id)
            .ok_or_else(|| AppError::Refused("That task isn't there any more".to_owned()))?;
        crate::features::messages::roster(state, &pool)
            .unwrap_or_default()
            .into_iter()
            .map(|m| (m.session_id, m.name))
            .collect()
    };
    let storage = lock_storage_arc(state)?;
    let repo = storage.tasks();
    let task = repo
        .get(task_id)?
        .ok_or_else(|| AppError::Refused("That task isn't there any more".to_owned()))?;
    let row = info(&storage, &task);
    let before = cursor
        .and_then(|c| c.strip_prefix('h'))
        .and_then(|n| n.parse().ok());
    let (history, history_next) = history_of(&repo.events(&task.id, before, HISTORY + 1)?);
    let children = repo.list(
        &TaskQuery {
            pool_id: Some(&task.pool_id),
            parent: Some(&task.id),
            ..TaskQuery::default()
        },
        PAGE,
    )?;
    let session_asked = repo
        .open_needs(0, 200)?
        .into_iter()
        .find(|n| n.kind == "launch" && n.task_id.as_deref() == Some(task.id.as_str()))
        .map(|n| n.id);
    Ok(TaskDetail {
        pool_name: super::pool_name(&storage, &task.pool_id).unwrap_or_default(),
        body: task.body.clone(),
        acceptance: task.acceptance.clone(),
        parent_title: task
            .parent_id
            .as_deref()
            .and_then(|p| repo.get(p).ok().flatten())
            .map(|p| p.title),
        children: children.iter().map(|t| info(&storage, t)).collect(),
        links: repo
            .links(&task.id)?
            .into_iter()
            .map(|l| TaskLinkInfo {
                kind: super::agent::link_word(l.kind, l.outgoing).to_owned(),
                task_id: l.other_id,
                title: l.other_title,
                status: l.other_status.as_str().to_owned(),
            })
            .collect(),
        sessions: sessions_of(&storage, &task.id, &names)?,
        threads: storage
            .messages()
            .threads_naming(&task.pool_id, &task.id, 10)?
            .into_iter()
            .map(|t| TaskThreadInfo {
                thread_id: t.id,
                subject: t.subject,
                count: t.count,
                at_ms: t.updated_at,
            })
            .collect(),
        history_next,
        history,
        issue: issue_of(state, &task),
        session_asked,
        actions: actions_of(&task, row.holder_session_id.is_some()),
        task: row,
    })
}
