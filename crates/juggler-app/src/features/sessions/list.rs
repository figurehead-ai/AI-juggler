//! The board's session list and stopping a session.

use std::sync::Arc;

use juggler_core::model::SessionRecord;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::sessions::dto::InstanceInfo;
use crate::features::sessions::read_model::{
    LiveStatsMap, desktop_isolation_labels, session_infos_for, working_records,
};
use crate::state::AppState;

/// The read model with the LAST sampled live stats (no extra collector
/// refresh, so the 2 s sampler's CPU deltas stay honest) — what this
/// Mac shares with paired consoles over the Fleet link (AJ-E705).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on storage failures.
pub fn instances_shared_snapshot(state: &Arc<AppState>) -> AppResult<Vec<InstanceInfo>> {
    let records = working_records(&*lock_storage_arc(state)?)?;
    instances_of(state, records)
}

/// The rows of `records` as [`sessions_list`] gives them (from the last
/// resource sample, never a new one): a page of sessions.
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub(crate) fn instances_of(
    state: &Arc<AppState>,
    records: Vec<SessionRecord>,
) -> AppResult<Vec<InstanceInfo>> {
    let live: LiveStatsMap = state
        .resources
        .lock()
        .map(|collector| {
            collector
                .stats_snapshot()
                .into_iter()
                .map(|(session, stats)| (session.to_string(), stats))
                .collect()
        })
        .unwrap_or_default();
    rows_with(state, records, &live)
}

/// The rows of `records` with `live` stats: counted subagents, the
/// runtime's words, the conversation's title and its resume refusal
/// (rc21) on each.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on
/// storage failures.
pub(crate) fn rows_with(
    state: &Arc<AppState>,
    records: Vec<SessionRecord>,
    live: &LiveStatsMap,
) -> AppResult<Vec<InstanceInfo>> {
    joined(state, stored_rows(state, records, live)?)
}

/// The rows of `records` as storage makes them (their Accounts, launch
/// settings, lifecycle, marks and `live` stats), before [`joined`].
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on
/// storage failures.
pub(crate) fn stored_rows(
    state: &Arc<AppState>,
    records: Vec<SessionRecord>,
    live: &LiveStatsMap,
) -> AppResult<Vec<InstanceInfo>> {
    let labels = desktop_isolation_labels(&state.registry);
    let adopted = state.supervisor.alive_adopted_sessions();
    let storage = lock_storage_arc(state)?;
    let mut rows = session_infos_for(&storage, records, live, &labels, &adopted)?;
    with_task_titles(&storage, &mut rows);
    Ok(rows)
}

/// A session's title before its conversation's is read (D-118, the 0.5
/// review of Home: "Session started yesterday 11:13 PM" read badly): the
/// task it works (the newest it was linked to: started for, claimed,
/// assigned or carried by a switch), so a session started for a task is
/// named by it, never by when it started. [`crate::features::conversations::titled`]
/// then puts the conversation's own title (the tool's summary, else the
/// first thing the user asked) over it when the index has one; only with
/// neither does the window fall back to a branch, then its folder and
/// start time (`shortName`, `naming::short_name`). A failed read leaves
/// the rows as they were.
fn with_task_titles(storage: &juggler_core::storage::Storage, rows: &mut [InstanceInfo]) {
    let untitled: Vec<String> = rows
        .iter()
        .filter(|row| row.title.is_none())
        .map(|row| row.id.clone())
        .collect();
    if untitled.is_empty() {
        return;
    }
    let Ok(links) = storage.tasks().of_sessions(&untitled) else {
        return;
    };
    let mut titles: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // Newest link first: the first seen for a session is its task now.
    for (session, _, title, _) in links {
        titles.entry(session).or_insert(title);
    }
    for row in rows.iter_mut().filter(|row| row.title.is_none()) {
        row.title = titles
            .get(&row.id)
            .filter(|title| !title.trim().is_empty())
            .cloned();
    }
}

/// What moves on its own beside storage, joined onto `rows`: counted
/// subagents, the runtime's words, the resume refusal (rc21) and the
/// conversation's title.
///
/// # Errors
///
/// A poisoned lock.
pub(crate) fn joined(
    state: &Arc<AppState>,
    rows: Vec<InstanceInfo>,
) -> AppResult<Vec<InstanceInfo>> {
    let rows = crate::features::subagents::count::counted(state, rows);
    let rows = crate::features::runtime::described(state, rows);
    let rows = crate::features::launch::resume_gate::with_refusals(state, rows);
    crate::features::conversations::titled(state, rows)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use juggler_adapter::AdapterRegistry;
    use juggler_core::model::{
        InstanceKind, InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
    };
    use juggler_core::storage::{NewTask, TASK_USER};
    use juggler_core::tasks::{Priority, TaskKind, TaskStatus};

    use super::*;

    fn session(id: &str) -> InstanceRecord {
        InstanceRecord {
            id: SessionId::new(id),
            conversation: None,
            profile: ProfileId::new("rec"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: PathBuf::from("/nowhere/app"),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        }
    }

    /// The 0.5 review of Home: a session with no conversation title yet
    /// read "Session started yesterday 11:13 PM". One started for a task
    /// (or that claimed one) is named by its task before any date; one
    /// with neither keeps no title (the window's facts name it).
    #[test]
    fn a_session_with_no_conversation_title_is_named_by_its_task_before_any_date() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(
            AppState::init_with_registry(dir.path().join("data"), AdapterRegistry::new()).unwrap(),
        );
        {
            let storage = state.storage.lock().unwrap();
            storage
                .accounts()
                .insert(&crate::test_support::account(
                    "acc",
                    "fake",
                    "Work",
                    dir.path().join("acc"),
                ))
                .unwrap();
            storage
                .profiles()
                .insert(&crate::test_support::launch_settings("rec", "Main", "acc"))
                .unwrap();
            for id in ["s-task", "s-none"] {
                storage.sessions().insert(&session(id)).unwrap();
            }
            let pool = storage.pools().create("Team", None).unwrap().id;
            let repo = storage.tasks();
            for (id, title) in [
                ("aj-0001", "Wire the export button"),
                ("aj-0002", "Add the CSV export"),
            ] {
                repo.create(&NewTask {
                    id,
                    pool_id: &pool,
                    kind: TaskKind::Task,
                    parent_id: None,
                    title,
                    body: "",
                    acceptance: "",
                    priority: Priority::Normal,
                    status: TaskStatus::Ready,
                    assignee: None,
                    created_by: TASK_USER,
                    created_by_name: "You",
                    external_url: None,
                    project: None,
                    blocked_by: &[],
                })
                .unwrap();
            }
            repo.link_session("aj-0001", "s-task", None, "started_for")
                .unwrap();
            // A later link is its task now.
            std::thread::sleep(std::time::Duration::from_millis(2));
            repo.link_session("aj-0002", "s-task", None, "claimed")
                .unwrap();
        }
        let records = {
            let storage = state.storage.lock().unwrap();
            ["s-task", "s-none"]
                .into_iter()
                .map(|id| storage.sessions().get(&SessionId::new(id)).unwrap())
                .collect::<Vec<_>>()
        };
        let rows = instances_of(&state, records).unwrap();
        let title = |id: &str| rows.iter().find(|row| row.id == id).unwrap().title.clone();
        assert_eq!(title("s-task").as_deref(), Some("Add the CSV export"));
        assert_eq!(title("s-none"), None, "nothing to name it by: its facts do");
    }
}
