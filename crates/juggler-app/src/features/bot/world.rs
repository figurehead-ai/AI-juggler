//! What Bot looks things up in (ADR-0034 §6.2): AI Juggler's own read
//! models, read once per question into rows: sessions (their state from
//! the sessions read model), tasks (the tasks store, with their pools)
//! and Accounts (the room the usage pipeline shows). Read-only: nothing
//! here writes, sends, switches or stops anything, and no transcript is
//! read. Agent-written names pass the guard ([`super::guard`]).

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use juggler_core::storage::TaskQuery;
use juggler_core::tasks::TaskStatus;

use super::dto::{BotRowInfo, BotRowKind, BotState};
use super::guard::guard_name;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The records, read now.
#[derive(Clone, Debug, Default)]
pub struct World {
    /// Now, epoch ms.
    pub now_ms: i64,
    /// Local time minus UTC, hours.
    pub offset_hours: i64,
    /// Every session the sessions read model holds.
    pub sessions: Vec<BotRowInfo>,
    /// The tasks, newest first.
    pub tasks: Vec<BotRowInfo>,
    /// Every Account, with its room.
    pub accounts: Vec<BotRowInfo>,
    /// What the guard removed from the records' names (for the echo check).
    pub removed: Vec<String>,
}

/// The longest name a row keeps, in characters.
pub const NAME_MAX: usize = 300;

/// The most tasks read for one question.
const TASKS_READ: u32 = 400;

impl World {
    /// Every row, by kind and id.
    #[must_use]
    pub fn find(&self, kind: BotRowKind, id: &str) -> Option<&BotRowInfo> {
        let rows = match kind {
            BotRowKind::Session => &self.sessions,
            BotRowKind::Task => &self.tasks,
            BotRowKind::Account => &self.accounts,
        };
        rows.iter().find(|row| row.id == id)
    }
}

/// A session's state from its status word.
#[must_use]
pub fn session_state(status: &str) -> Option<BotState> {
    Some(match status {
        "starting" | "running" => BotState::Working,
        "waiting_user" => BotState::AsksYou,
        "stuck" => BotState::Stuck,
        "quota_blocked" => BotState::Limit,
        "finished" => BotState::Finished,
        "stopped" | "handed_over" => BotState::Ended,
        "failed" => BotState::Failed,
        _ => return None,
    })
}

/// A task's state from its status.
#[must_use]
pub const fn task_state(status: TaskStatus) -> BotState {
    match status {
        TaskStatus::Ready => BotState::Ready,
        TaskStatus::InProgress => BotState::Working,
        TaskStatus::InReview => BotState::InReview,
        TaskStatus::Blocked => BotState::Blocked,
        TaskStatus::Todo => BotState::Todo,
        TaskStatus::Done => BotState::Done,
        TaskStatus::Cancelled => BotState::Cancelled,
    }
}

fn empty_row(kind: BotRowKind, id: String, title: String) -> BotRowInfo {
    BotRowInfo {
        kind,
        id,
        title,
        name_hidden: false,
        state: None,
        at_ms: None,
        started_ms: None,
        provider: None,
        app_id: None,
        app_label: None,
        desktop: false,
        project: None,
        project_id: None,
        pool: None,
        pool_id: None,
        holder: None,
        files_changed: None,
        branch: None,
        used_percent: None,
        window: None,
        resets_at_ms: None,
        blocked_by: None,
        quote: None,
        then_state: None,
        gone: false,
    }
}

/// A name through the guard: the cleaned words, and whether it was all
/// an order; what was removed goes to `removed`.
fn guarded_into(name: &str, removed: &mut Vec<String>) -> (String, bool) {
    // A name is cut to what a row shows before the guard reads it (the Bot
    // review's I3: no long text is read twice under the storage lock).
    let name: String = name.chars().take(NAME_MAX).collect();
    let guarded = guard_name(&name);
    let hidden = !guarded.removed.is_empty() && guarded.cleaned.is_empty();
    removed.extend(guarded.removed);
    (guarded.cleaned, hidden)
}

/// A session row from the read model's row.
#[must_use]
pub fn session_row<S: BuildHasher>(
    row: &crate::features::sessions::dto::InstanceInfo,
    projects: &HashMap<String, String, S>,
    removed: &mut Vec<String>,
) -> BotRowInfo {
    let folder = std::path::Path::new(&row.cwd)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty());
    let raw = row
        .label
        .clone()
        .filter(|l| !l.trim().is_empty())
        .or_else(|| row.title.clone().filter(|t| !t.trim().is_empty()))
        .or(folder)
        .unwrap_or_default();
    let (title, name_hidden) = guarded_into(&raw, removed);
    let state = session_state(&row.status);
    let at_ms = match state {
        Some(BotState::AsksYou | BotState::Stuck) => row
            .attention
            .as_ref()
            .map(|a| a.since_ms)
            .or(row.last_activity_ms),
        Some(BotState::Working) => Some(row.started_at_ms),
        _ => row
            .lifecycle_at_ms
            .or(row.last_activity_ms)
            .or(Some(row.started_at_ms)),
    };
    BotRowInfo {
        name_hidden,
        state,
        at_ms,
        started_ms: Some(row.started_at_ms),
        provider: Some(row.provider.clone()),
        app_id: row.app_id.clone(),
        app_label: Some(guarded_into(&row.account_label, removed).0).filter(|l| !l.is_empty()),
        desktop: row.kind == "desktop",
        project: row
            .project_id
            .as_ref()
            .and_then(|id| projects.get(id).cloned()),
        project_id: row.project_id.clone(),
        files_changed: row
            .git_dirty_count
            .and_then(|n| u32::try_from(n).ok())
            .filter(|&n| n > 0),
        // Every string from a record passes the guard, names included: an
        // agent can name a branch (the Bot review's M1).
        branch: row
            .git_branch
            .as_deref()
            .map(|b| guarded_into(b, removed).0)
            .filter(|b| !b.is_empty()),
        ..empty_row(BotRowKind::Session, row.id.clone(), title)
    }
}

/// Reads the records now.
///
/// # Errors
///
/// The storage lock is poisoned.
pub fn gather(state: &Arc<AppState>) -> AppResult<World> {
    let clock = state.notifier.clock();
    let now_ms = clock.now;
    let rows = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        crate::features::sessions::read_model::session_infos_snapshot(&storage).unwrap_or_default()
    };
    let rows = crate::features::conversations::titled(state, rows).unwrap_or_default();
    // Read under the lock, guarded after it (I3).
    let (projects, pools, tasks, accounts) = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        (
            storage.projects().list().unwrap_or_default(),
            storage.pools().list().unwrap_or_default(),
            storage
                .tasks()
                .list(&TaskQuery::default(), TASKS_READ)
                .unwrap_or_default(),
            accounts(state, &storage, now_ms),
        )
    };
    let mut removed = Vec::new();
    let projects: HashMap<String, String> = projects
        .into_iter()
        .map(|p| (p.id.to_string(), guarded_into(&p.name, &mut removed).0))
        .collect();
    let pools: HashMap<String, (String, Option<String>)> = pools
        .into_iter()
        .map(|p| {
            (
                p.id,
                (
                    guarded_into(&p.name, &mut removed).0,
                    p.project.map(|id| id.to_string()),
                ),
            )
        })
        .collect();
    let sessions = rows
        .iter()
        .map(|row| session_row(row, &projects, &mut removed))
        .collect();
    let tasks = tasks
        .into_iter()
        .map(|task| {
            let (title, name_hidden) = guarded_into(&task.title, &mut removed);
            let (pool, pool_project) = pools.get(&task.pool_id).cloned().unwrap_or_default();
            let project_id = task.project.clone().or(pool_project);
            BotRowInfo {
                name_hidden,
                state: Some(task_state(task.status)),
                at_ms: Some(task.updated_at),
                project: project_id.as_ref().and_then(|id| projects.get(id).cloned()),
                project_id,
                pool: Some(pool).filter(|p| !p.is_empty()),
                pool_id: Some(task.pool_id.clone()),
                holder: task
                    .assignee_name
                    .clone()
                    .map(|h| guarded_into(&h, &mut removed).0)
                    .filter(|h| !h.is_empty()),
                blocked_by: u32::try_from(task.blocked_by.len()).ok().filter(|&n| n > 0),
                ..empty_row(BotRowKind::Task, task.id.clone(), title)
            }
        })
        .collect();
    let accounts = accounts
        .into_iter()
        .map(|row| BotRowInfo {
            title: guarded_into(&row.title, &mut removed).0,
            ..row
        })
        .collect();
    Ok(World {
        now_ms,
        offset_hours: clock.utc_offset_hours.unwrap_or(0),
        sessions,
        tasks,
        accounts,
        removed,
    })
}

/// Every Account, with the room the usage pipeline shows.
fn accounts(
    state: &AppState,
    storage: &juggler_core::storage::Storage,
    now_ms: i64,
) -> Vec<BotRowInfo> {
    storage
        .accounts()
        .list()
        .unwrap_or_default()
        .into_iter()
        .map(|account| {
            let display = crate::features::quota::shown_all(state, storage, &account, now_ms)
                .into_iter()
                .next();
            let (window, shown) = display.as_ref().map_or((None, None), |d| {
                let tracking = |w: &Option<crate::features::quota::WindowDisplay>| {
                    w.as_ref().filter(|w| w.percentage.is_some()).cloned()
                };
                tracking(&d.five_hour).map_or_else(
                    || {
                        (
                            tracking(&d.weekly).map(|_| "weekly".to_owned()),
                            tracking(&d.weekly),
                        )
                    },
                    |w| (Some("five_hour".to_owned()), Some(w)),
                )
            });
            BotRowInfo {
                used_percent: shown.as_ref().and_then(|w| w.percentage).map(|p| {
                    let p = p.clamp(0.0, 100.0).round();
                    // In range by the clamp.
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "clamped to 0..=100 first"
                    )]
                    let whole = p as u32;
                    whole
                }),
                resets_at_ms: shown.and_then(|w| w.resets_at_ms),
                window,
                ..empty_row(
                    BotRowKind::Account,
                    account.id.to_string(),
                    account.label.clone(),
                )
            }
        })
        .collect()
}

/// Rows for tests.
#[cfg(test)]
pub mod tests_support {
    use super::{BotRowInfo, BotRowKind, BotState, empty_row};

    /// A row of `kind` named `title`, its id the title.
    #[must_use]
    pub fn row(kind: BotRowKind, title: &str, state: Option<BotState>) -> BotRowInfo {
        BotRowInfo {
            state,
            ..empty_row(kind, title.to_owned(), title.to_owned())
        }
    }
}
