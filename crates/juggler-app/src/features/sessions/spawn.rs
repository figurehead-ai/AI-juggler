//! Intent before spawn, for the session row (ADR-0010 §1).
//!
//! The spawn ledger writes its intent before a process starts; the
//! session row does the same. Every path that starts a session (a
//! launch, a desktop launch, a resume, a switch or hand-over, its undo,
//! a switch back, a crash restart) writes its row as `Starting` first and
//! spawns second, so whatever the process reports, an exit within
//! milliseconds included, finds its row. A spawn that fails ends the row
//! as `Failed`, with the reason: no row is left looking live.

use std::path::PathBuf;
use std::sync::Arc;

use juggler_core::model::{
    Account, AttentionReason, ConversationId, InstanceKind, InstanceRecord, ProfileId, ProjectId,
    SessionId, SessionStatus, Timestamp, TranscriptRef, WindowIdentity,
};

use crate::commands::{engine_usage_snapshot, local_machine_id, lock_storage_arc};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The attention rule of a session whose process never started (the
/// reason, in words, is its one input).
pub const LAUNCH_FAILED_RULE: &str = "launch_failed";

/// What a new session's row says before its process starts.
pub(crate) struct NewSession<'a> {
    /// The id the supervisor will run it under.
    pub id: &'a SessionId,
    /// The account it runs on (its tool, for usage statistics).
    pub account: &'a Account,
    /// Its launch settings.
    pub profile: &'a ProfileId,
    /// Terminal or desktop.
    pub kind: InstanceKind,
    /// The tracked project it belongs to.
    pub project: Option<ProjectId>,
    /// The folder it runs in.
    pub cwd: PathBuf,
    /// The conversation it resumes, when it resumes one.
    pub conversation: Option<ConversationId>,
    /// Where that conversation's file is, when known.
    pub transcript_ref: Option<TranscriptRef>,
    /// How it came to be, for usage statistics (`new`, `resume`, …).
    pub origin: &'static str,
}

/// Writes `row` as a `Starting` session, then runs `spawn` (which starts
/// its process under the supervisor). On success the start is counted
/// and `spawn`'s result returned; on failure the row ends as `Failed`
/// with the reason and the error is returned.
///
/// Storage is not held while `spawn` runs: the lifecycle forwarder may
/// apply the process's events to the row at any moment after it starts.
///
/// # Errors
///
/// Storage errors writing the row (nothing is spawned then), and
/// `spawn`'s own error.
pub(crate) fn spawn_tracked<T>(
    state: &Arc<AppState>,
    row: &NewSession<'_>,
    spawn: impl FnOnce() -> AppResult<T>,
) -> AppResult<T> {
    // The machine identity locks storage itself: resolved first.
    let machine = local_machine_id(state);
    {
        let storage = lock_storage_arc(state)?;
        let usage = engine_usage_snapshot(&storage, row.profile);
        storage.sessions().insert(&InstanceRecord {
            id: row.id.clone(),
            conversation: row.conversation.clone(),
            profile: row.profile.clone(),
            machine,
            kind: row.kind,
            project: row.project.clone(),
            cwd: row.cwd.clone(),
            status: SessionStatus::Starting,
            started_at: Timestamp::now(),
            last_activity: None,
            usage,
            attention: None,
            running: None,
            // A desktop window is known once its process is: see
            // `record_window`. Terminal sessions own none (AJ-E414).
            window: None,
            transcript_ref: row.transcript_ref.clone(),
        })?;
    }
    match spawn() {
        Ok(spawned) => {
            crate::usage_stats::instance_started(
                row.account.harness.as_str(),
                row.kind == InstanceKind::Desktop,
                row.origin,
            );
            Ok(spawned)
        }
        Err(error) => {
            launch_failed(state, row.id, &error);
            Err(error)
        }
    }
}

/// Ends a session whose process never started: `Failed`, the reason
/// its one input. Only while it is still `Starting` (nothing else has
/// said anything about it); a storage failure here leaves it to the
/// next start's reconciliation, which ends every row it can't adopt.
fn launch_failed(state: &Arc<AppState>, id: &SessionId, error: &AppError) {
    let Ok(storage) = lock_storage_arc(state) else {
        return;
    };
    let Ok(mut record) = storage.sessions().get(id) else {
        return;
    };
    if record.status != SessionStatus::Starting {
        return;
    }
    let now = Timestamp::now();
    record.status = SessionStatus::Failed;
    record.attention = Some(AttentionReason {
        rule: LAUNCH_FAILED_RULE.to_owned(),
        inputs: vec![error.to_string()],
        since_ms: now.millis(),
    });
    record.last_activity = Some(now);
    let _ = storage.sessions().update(&record);
}

/// Records a desktop session's window identity once its process is
/// known, read and written under one lock, so a status its events set
/// meanwhile stays as they set it.
///
/// # Errors
///
/// Storage errors.
pub(crate) fn record_window(
    state: &Arc<AppState>,
    id: &SessionId,
    window: WindowIdentity,
) -> AppResult<()> {
    let storage = lock_storage_arc(state)?;
    let mut record = storage.sessions().get(id)?;
    record.window = Some(window);
    storage.sessions().update(&record)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_supervisor::LifecycleEvent;

    /// A state with one Claude Code account (a stand-in binary: nothing
    /// here runs it) and its launch settings.
    fn fixture(dir: &std::path::Path) -> (Arc<AppState>, Account, ProfileId) {
        let mut registry = juggler_adapter::AdapterRegistry::new();
        registry.register(Arc::new(juggler_adapters::ClaudeCodeAdapter::with_bin(
            dir.join("claude-not-run"),
        )));
        let state = Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap());
        let account =
            crate::commands::create_account(&state, juggler_adapters::CLAUDE_CODE_ID, "Work", None)
                .unwrap();
        let profile =
            crate::commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
        let account = crate::commands::find_account_arc(&state, &account.id).unwrap();
        (state, account, ProfileId::new(profile.id))
    }

    fn row<'a>(id: &'a SessionId, account: &'a Account, profile: &'a ProfileId) -> NewSession<'a> {
        NewSession {
            id,
            account,
            profile,
            kind: InstanceKind::Terminal,
            project: None,
            cwd: std::env::temp_dir(),
            conversation: None,
            transcript_ref: None,
            origin: "new",
        }
    }

    fn status(state: &Arc<AppState>, id: &SessionId) -> InstanceRecord {
        lock_storage_arc(state).unwrap().sessions().get(id).unwrap()
    }

    /// The race, forced: the process exits at once and its exit is
    /// applied (as the lifecycle forwarder would) before `spawn_tracked`
    /// returns, which is where the row used to be written. The exit must
    /// land on the row, and the start that comes after it must not bring
    /// the row back to life.
    #[test]
    fn an_exit_before_the_launch_returns_lands_on_the_row() {
        let dir = tempfile::tempdir().unwrap();
        let (state, account, profile) = fixture(dir.path());
        let id = SessionId::new("sess-exits-at-once");
        let spec = juggler_adapter::LaunchSpec {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), "exit 0".into()],
            env: juggler_adapter::EnvMap::default(),
            cwd: std::env::temp_dir(),
            needs_pty: false,
        };
        let ctx = crate::commands::ledger_ctx(&state, &account, None);
        let mut started = None;
        spawn_tracked(&state, &row(&id, &account, &profile), || {
            let spawned = state.supervisor.spawn(&id, &spec, &ctx)?;
            // Drain until the exit, applying every event in the order
            // the supervisor sent it; keep a Started to replay after.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                assert!(
                    std::time::Instant::now() < deadline,
                    "the stub never exited"
                );
                let Some(event) = state
                    .supervisor
                    .recv_timeout(std::time::Duration::from_millis(100))
                else {
                    continue;
                };
                if matches!(event, LifecycleEvent::Started { .. }) {
                    started = Some(event.clone());
                }
                let exited = matches!(event, LifecycleEvent::Exited { .. });
                let storage = lock_storage_arc(&state).unwrap();
                crate::commands::apply_lifecycle_event(&storage, event, false, None);
                if exited {
                    break;
                }
            }
            Ok(spawned)
        })
        .unwrap();
        assert_eq!(status(&state, &id).status, SessionStatus::Finished);
        // A Started read after the exit (two threads send them) is stale.
        let late = started.unwrap_or(LifecycleEvent::Started {
            session: id.clone(),
            pid: 1,
        });
        let storage = lock_storage_arc(&state).unwrap();
        assert!(crate::commands::apply_lifecycle_event(&storage, late, false, None).is_none());
        drop(storage);
        assert_eq!(status(&state, &id).status, SessionStatus::Finished);
    }

    #[test]
    fn a_spawn_that_fails_ends_its_row_failed_with_the_reason() {
        let dir = tempfile::tempdir().unwrap();
        let (state, account, profile) = fixture(dir.path());
        let id = SessionId::new("sess-never-starts");
        let result: AppResult<()> = spawn_tracked(&state, &row(&id, &account, &profile), || {
            Err(AppError::Refused("no such program".to_owned()))
        });
        assert!(result.is_err());
        let record = status(&state, &id);
        assert_eq!(record.status, SessionStatus::Failed);
        let reason = record.attention.unwrap();
        assert_eq!(reason.rule, LAUNCH_FAILED_RULE);
        assert!(
            reason.inputs[0].contains("no such program"),
            "{:?}",
            reason.inputs
        );
    }

    #[test]
    fn the_row_exists_as_starting_while_the_process_starts() {
        let dir = tempfile::tempdir().unwrap();
        let (state, account, profile) = fixture(dir.path());
        let id = SessionId::new("sess-seen-starting");
        let seen = spawn_tracked(&state, &row(&id, &account, &profile), || {
            Ok(status(&state, &id).status)
        })
        .unwrap();
        assert_eq!(seen, SessionStatus::Starting);
    }
}
