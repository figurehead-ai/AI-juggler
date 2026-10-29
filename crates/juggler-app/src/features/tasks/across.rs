//! 0.3 — the Tasks screen's reads: every pool's tasks at once, by state
//! (the board's columns) or as the plan (epics and stories), filtered by
//! project, pool, holder and words, a page at a time. Reads only: agents
//! write the tasks (D-44); the user's actions stay [`super::oversight`]'s.

use std::collections::HashMap;
use std::sync::Arc;

use juggler_core::storage::{Storage, TASK_USER, TaskQuery};
use juggler_core::tasks::{TaskKind, TaskStatus};

use super::dto::{ProjectTaskCounts, TaskBoardRow, TaskCounts, TasksFilterInput};
use super::views::{after_of, info, sort_key};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::paging::{Page, PageRequest, SortKey, page, page_of_fetched};
use crate::state::AppState;

/// The filter, as the store reads it: its pools (by id), with each pool's
/// name and project for the rows.
struct Scope {
    pools: Vec<String>,
    names: HashMap<String, (String, Option<String>)>,
}

/// The pools the filter names: one pool, a project's pools, or every pool
/// that isn't deleted.
fn scope(storage: &Storage, filter: &TasksFilterInput) -> AppResult<Scope> {
    let shown = crate::features::pools::shown_pools(storage)?;
    let names = shown
        .iter()
        .map(|(pool, name)| {
            let project = pool.project.as_ref().map(|p| p.as_str().to_owned());
            (pool.id.clone(), (name.clone(), project))
        })
        .collect::<HashMap<_, _>>();
    let pools = shown
        .iter()
        .filter(|(pool, _)| filter.pool_id.as_deref().is_none_or(|id| pool.id == id))
        .filter(|(pool, _)| {
            filter
                .project_id
                .as_deref()
                .is_none_or(|id| pool.project.as_ref().is_some_and(|p| p.as_str() == id))
        })
        .map(|(pool, _)| pool.id.clone())
        .collect();
    Ok(Scope { pools, names })
}

/// The statuses a filter's word names: `open` (the default), `closed`, or
/// one status.
fn statuses(word: Option<&str>) -> AppResult<Vec<TaskStatus>> {
    match word {
        None | Some("open") => Ok(TaskStatus::ALL
            .into_iter()
            .filter(|s| s.is_open())
            .collect()),
        Some("closed") => Ok(vec![TaskStatus::Done, TaskStatus::Cancelled]),
        Some(word) => TaskStatus::parse(word)
            .map(|status| vec![status])
            .ok_or_else(|| AppError::Validation {
                subject: "status",
                message: format!("{word} isn't a status"),
            }),
    }
}

/// The kinds a filter names: the work (`task`, the default) or the plan
/// (`plan`: epics and stories).
fn kinds(word: Option<&str>) -> AppResult<&'static [TaskKind]> {
    match word {
        None | Some("task") => Ok(&[TaskKind::Task]),
        Some("plan") => Ok(&[TaskKind::Epic, TaskKind::Story]),
        Some(word) => Err(AppError::Validation {
            subject: "kind",
            message: format!("{word} isn't a kind of item"),
        }),
    }
}

/// The holders a filter names: anyone (the default), `you`, or `nobody`
/// (no one has it yet). `(assignees, unheld)`.
fn holders(word: Option<&str>) -> AppResult<(Option<Vec<String>>, bool)> {
    match word {
        None | Some("anyone") => Ok((None, false)),
        Some("you") => Ok((Some(vec![TASK_USER.to_owned()]), false)),
        Some("nobody") => Ok((None, true)),
        Some(word) => Err(AppError::Validation {
            subject: "holder",
            message: format!("{word} isn't a holder filter"),
        }),
    }
}

/// An item's epic and story, outermost first ("Checkout v2", "Totals"),
/// read once per page.
fn path_of(
    storage: &Storage,
    parent: Option<&str>,
    seen: &mut HashMap<String, Option<(String, Option<String>)>>,
) -> Vec<String> {
    let mut path = Vec::new();
    let mut next = parent.map(str::to_owned);
    // An epic › a story › its task: two steps at most.
    for _ in 0..2 {
        let Some(id) = next.take() else { break };
        let entry = seen
            .entry(id.clone())
            .or_insert_with(|| {
                storage
                    .tasks()
                    .get(&id)
                    .ok()
                    .flatten()
                    .map(|t| (t.title, t.parent_id))
            })
            .clone();
        let Some((title, up)) = entry else { break };
        path.push(title);
        next = up;
    }
    path.reverse();
    path
}

/// One page of every pool's tasks the filter names, ready work first.
///
/// # Errors
///
/// Validation for an unknown status, kind, holder or cursor; storage
/// failures.
pub fn across(
    state: &Arc<AppState>,
    filter: &TasksFilterInput,
    request: &PageRequest,
) -> AppResult<Page<TaskBoardRow>> {
    let limit = request.limit();
    let statuses = statuses(filter.status.as_deref())?;
    let kinds = kinds(filter.kind.as_deref())?;
    let (assignees, unheld) = holders(filter.holder.as_deref())?;
    let storage = lock_storage_arc(state)?;
    let scope = scope(&storage, filter)?;
    if scope.pools.is_empty() {
        return Ok(Page::empty());
    }
    let rows = storage.tasks().list(
        &TaskQuery {
            pools: Some(&scope.pools),
            assignees: assignees.as_deref(),
            unheld,
            statuses: Some(&statuses),
            kinds: Some(kinds),
            words: filter.words.as_deref().filter(|w| !w.trim().is_empty()),
            after: after_of(request)?,
            ..TaskQuery::default()
        },
        u32::try_from(limit + 1).unwrap_or(u32::MAX),
    )?;
    let mut seen = HashMap::new();
    let fetched = rows
        .iter()
        .map(|task| {
            let (pool_name, project_id) =
                scope.names.get(&task.pool_id).cloned().unwrap_or_default();
            let row = TaskBoardRow {
                task: info(&storage, task),
                pool_name,
                project_id,
                path: path_of(&storage, task.parent_id.as_deref(), &mut seen),
            };
            (sort_key(task), row)
        })
        .collect();
    Ok(page_of_fetched(fetched, limit, None))
}

/// How many of the tasks the filter names (its status aside) are in each
/// honest status: the board's column counts.
///
/// # Errors
///
/// Validation for an unknown kind or holder; storage failures.
pub fn across_counts(state: &Arc<AppState>, filter: &TasksFilterInput) -> AppResult<TaskCounts> {
    let kinds = kinds(filter.kind.as_deref())?;
    let (assignees, unheld) = holders(filter.holder.as_deref())?;
    let storage = lock_storage_arc(state)?;
    let scope = scope(&storage, filter)?;
    let mut out = TaskCounts::default();
    if scope.pools.is_empty() {
        return Ok(out);
    }
    for (status, count) in storage.tasks().counts_where(&TaskQuery {
        pools: Some(&scope.pools),
        assignees: assignees.as_deref(),
        unheld,
        kinds: Some(kinds),
        words: filter.words.as_deref().filter(|w| !w.trim().is_empty()),
        ..TaskQuery::default()
    })? {
        *super::views::slot(&mut out, status) = count;
    }
    Ok(out)
}

/// The most projects one call counts (a page of the Projects screen).
pub const PROJECTS_MAX: usize = 200;

/// Each project's open work (its pools' tasks) by status, in the order
/// asked, a page at a time: the Projects screen's page of rows in one
/// call.
///
/// # Errors
///
/// Validation for more than [`PROJECTS_MAX`] projects or a cursor this
/// app didn't make; storage failures.
pub fn counts_by_project(
    state: &Arc<AppState>,
    project_ids: &[String],
    request: &PageRequest,
) -> AppResult<Page<ProjectTaskCounts>> {
    if project_ids.len() > PROJECTS_MAX {
        return Err(AppError::Validation {
            subject: "projects",
            message: format!("at most {PROJECTS_MAX} projects at once"),
        });
    }
    let storage = lock_storage_arc(state)?;
    let shown = crate::features::pools::shown_pools(&storage)?;
    let repo = storage.tasks();
    let mut rows = Vec::with_capacity(project_ids.len());
    for (at, project) in project_ids.iter().enumerate() {
        let pools: Vec<String> = shown
            .iter()
            .filter(|(pool, _)| pool.project.as_ref().is_some_and(|p| p.as_str() == project))
            .map(|(pool, _)| pool.id.clone())
            .collect();
        let mut counts = TaskCounts::default();
        if !pools.is_empty() {
            for (status, count) in repo.counts_where(&TaskQuery {
                pools: Some(&pools),
                kinds: Some(&[TaskKind::Task]),
                ..TaskQuery::default()
            })? {
                *super::views::slot(&mut counts, status) = count;
            }
        }
        rows.push((
            i64::try_from(at).unwrap_or(i64::MAX),
            ProjectTaskCounts {
                project_id: project.clone(),
                counts,
            },
        ));
    }
    page(
        rows,
        |(at, row)| SortKey::new([*at], row.project_id.clone()),
        request,
    )
    .map(|page| page.map(|(_, row)| row))
}
