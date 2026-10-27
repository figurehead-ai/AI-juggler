//! A session's History tab (wf09, AJ-E1533, AJ-E2108 Req.4): its status
//! changes, its lifecycle steps (reviewed, archived, restarted after a
//! crash) and the switches it took part in, as one timeline, oldest
//! first, a page at a time. Works for a live session as for an ended one
//! (Replay is the ended one's read-only transcript).

use std::sync::Arc;

use juggler_core::model::SessionId;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::history::LineageStepInfo;
use crate::paging::{Page, PageRequest, SortKey, page};
use crate::state::AppState;

/// One event in a session's history.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SessionEventInfo {
    /// `status` | `lifecycle` | `switch`.
    pub kind: String,
    /// `status`: the status it moved to (a wire value the UI says in
    /// words); `lifecycle`: the step; `switch`: empty.
    pub what: String,
    /// `switch`: where the work went or came from.
    pub switch: Option<LineageStepInfo>,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// Session `session_id`'s history, oldest first.
///
/// # Errors
///
/// `NotFound` for an unknown session; storage failures; a cursor this
/// app didn't make.
pub fn history_page_impl(
    state: &Arc<AppState>,
    session_id: &str,
    request: &PageRequest,
) -> AppResult<Page<SessionEventInfo>> {
    let storage = lock_storage_arc(state)?;
    let id = SessionId::new(session_id);
    storage.sessions().get(&id)?;
    let mut rows: Vec<(String, SessionEventInfo)> = Vec::new();
    let mut push = |kind: &str, n: usize, event: SessionEventInfo| {
        rows.push((format!("{kind}:{n:08}"), event));
    };
    for (n, (_, to, at_ms)) in storage.sessions().transitions(&id)?.into_iter().enumerate() {
        push(
            "a-status",
            n,
            SessionEventInfo {
                kind: "status".into(),
                what: to,
                switch: None,
                at_ms,
            },
        );
    }
    for (n, (what, at_ms)) in storage
        .sessions()
        .lifecycle_events(&id)?
        .into_iter()
        .enumerate()
    {
        push(
            "b-lifecycle",
            n,
            SessionEventInfo {
                kind: "lifecycle".into(),
                what,
                switch: None,
                at_ms,
            },
        );
    }
    for (n, (at_ms, step)) in crate::history::lineage_steps(&storage, session_id)?
        .into_iter()
        .enumerate()
    {
        push(
            "c-switch",
            n,
            SessionEventInfo {
                kind: "switch".into(),
                what: String::new(),
                switch: Some(step),
                at_ms,
            },
        );
    }
    drop(storage);
    Ok(page(
        rows,
        |(key, event)| SortKey::new([event.at_ms], key.clone()),
        request,
    )?
    .map(|(_, event)| event))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use juggler_adapter::AdapterRegistry;
    use juggler_core::model::{
        Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
        ProfileId, SessionStatus, SettingsOverlay, Timestamp,
    };

    use super::*;

    fn state_with_a_session(dir: &std::path::Path) -> Arc<AppState> {
        let state = Arc::new(
            AppState::init_with_registry(dir.join("data"), AdapterRegistry::new()).unwrap(),
        );
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc"),
                harness: HarnessId::new("fake"),
                label: "Work".into(),
                plan: None,
                state_dir: dir.join("acc"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
                in_place: false,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec"),
                name: "Main".into(),
                account: AccountId::new("acc"),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            })
            .unwrap();
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: SessionId::new("s-1"),
                conversation: None,
                profile: ProfileId::new("rec"),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: PathBuf::from("/nowhere/app"),
                status: SessionStatus::Finished,
                started_at: Timestamp::now(),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
        storage
            .sessions()
            .record_lifecycle(&SessionId::new("s-1"), "archived")
            .unwrap();
        drop(storage);
        state
    }

    #[test]
    fn a_session_s_history_is_its_steps_oldest_first_a_page_at_a_time() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_a_session(dir.path());
        let all = history_page_impl(&state, "s-1", &PageRequest::first(50)).unwrap();
        let kinds: Vec<&str> = all.items.iter().map(|event| event.kind.as_str()).collect();
        assert!(
            kinds.contains(&"lifecycle"),
            "a lifecycle step is in its history"
        );
        assert!(
            all.items
                .windows(2)
                .all(|pair| pair[0].at_ms <= pair[1].at_ms),
            "oldest first"
        );
        let first = history_page_impl(&state, "s-1", &PageRequest::first(1)).unwrap();
        assert_eq!(first.items.len(), 1, "a page holds its limit");
        assert_eq!(first.total, all.total, "the total says how much there is");
        assert!(
            history_page_impl(&state, "missing", &PageRequest::first(1)).is_err(),
            "an unknown session is refused"
        );
    }
}
