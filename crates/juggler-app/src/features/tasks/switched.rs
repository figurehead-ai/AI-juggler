//! AJ-E1904, the 2026-10-28 tasks lab (step 6): a switch is a line in the
//! history of each task it carries. A task's holder is its line of work
//! (the Work item), so the task already moves with the switch; this writes
//! it down where the user reads a task's story: its history says
//! "Planner continued on Personal (from Work Max)", and its sessions list
//! the new session, so the project's "How this work travelled" shows the
//! task under both sessions even after later switches.

use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_core::model::{AccountId, SessionId};
use juggler_core::storage::{Storage, TaskQuery};
use juggler_core::tasks::TaskStatus;

use crate::state::AppState;

/// The history's verb for a switch, and the sessions list's word.
pub const VERB: &str = "switched";

/// The open statuses a switch carries a task in.
const OPEN: [TaskStatus; 5] = [
    TaskStatus::Ready,
    TaskStatus::InProgress,
    TaskStatus::InReview,
    TaskStatus::Blocked,
    TaskStatus::Todo,
];

/// The most tasks one switch writes a line for.
const MOST: u32 = 50;

/// Where a session runs, in words: the account, and its tool when the
/// switch changed tools ("Personal in Codex").
fn place(
    registry: &AdapterRegistry,
    account: &juggler_core::model::Account,
    with_tool: bool,
) -> String {
    if with_tool {
        format!(
            "{} in {}",
            account.label,
            crate::features::tools::display_name(registry, account.harness.as_str())
        )
    } else {
        account.label.clone()
    }
}

/// After a switch from `from` to `to` landed in the new session
/// `session`: [`note`], then the window hears of the pools it changed.
/// Never fails the switch.
pub(crate) fn after_switch(
    state: &Arc<AppState>,
    session: &SessionId,
    from: &AccountId,
    to: &AccountId,
) {
    let Ok(storage) = crate::commands::lock_storage_arc(state) else {
        return;
    };
    let pools = note(&storage, &state.registry, session, from, to);
    drop(storage);
    for pool in pools {
        state.tasks.notify(&pool);
    }
}

/// A switch from `from` to `to` landed in the new session `session`: each
/// open task its line of work holds gets the switch in its history, and
/// the new session in its sessions. Returns the pools whose tasks it
/// noted. A storage error writes nothing more.
pub fn note(
    storage: &Storage,
    registry: &AdapterRegistry,
    session: &SessionId,
    from: &AccountId,
    to: &AccountId,
) -> Vec<String> {
    let accounts = storage.accounts();
    let (Ok(from), Ok(to)) = (accounts.get(from), accounts.get(to)) else {
        return Vec::new();
    };
    let tools_differ = from.harness != to.harness;
    let detail = format!(
        "{} (from {})",
        place(registry, &to, tools_differ),
        place(registry, &from, false)
    );
    // Every ref the new session's tasks may be held by: its Work item's,
    // and each earlier session of it (a task claimed before its first
    // switch).
    let refs = crate::features::messages::refs_of(storage, session);
    let work = storage
        .switch_graph()
        .works_by_session()
        .ok()
        .and_then(|works| works.get(session).cloned());
    let repo = storage.tasks();
    let Ok(held) = repo.list(
        &TaskQuery {
            assignees: Some(&refs),
            statuses: Some(&OPEN),
            ..TaskQuery::default()
        },
        MOST,
    ) else {
        return Vec::new();
    };
    let mut pools: Vec<String> = Vec::new();
    for task in &held {
        let holder = task.assignee_ref.as_deref().unwrap_or_default();
        let name = task.assignee_name.as_deref().unwrap_or("Its session");
        if repo
            .log(&task.pool_id, &task.id, (holder, name), VERB, Some(&detail))
            .is_ok()
            && !pools.contains(&task.pool_id)
        {
            pools.push(task.pool_id.clone());
        }
        let _ = repo.link_session(
            &task.id,
            session.as_str(),
            work.as_ref().map(juggler_core::model::WorkItemId::as_str),
            VERB,
        );
    }
    pools
}
