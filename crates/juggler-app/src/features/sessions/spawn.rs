//! The one way a session starts. Every launch, resume, switch or
//! hand-over, its undo, a switch back, a continue here, a crash restart
//! and a resume after AI Juggler restarted goes through
//! `start_session`, which, in this order:
//!
//! 1. writes the session's row as `Starting` (intent before spawn,
//!    ADR-0010 §1), so whatever its process reports, an exit within
//!    milliseconds included, finds its row;
//! 2. chooses its runtime (ADR-0022): the terminal, or driven when its
//!    plan says so and the tool's protocol answers
//!    ([`crate::features::runtime::launch`], which spawns each attempt
//!    through `spawn_process`);
//! 3. adds the agent bridge to the launch that runs (AJ-E1704);
//! 4. adds the account's key to that launch only, for an account through
//!    another service (AJ-E1601, D-34): read from the Keychain into
//!    memory and put in the variable its tool reads. Never in its
//!    arguments (the spawn ledger records those), never in the launch
//!    spec the caller keeps, and never for any other account's session;
//! 5. spawns it under the app's supervisor, which sends its `Started`
//!    before the thread that waits for its exit exists;
//! 6. counts the start, or ends the row as `Failed` with the reason: no
//!    row is left looking live.
//!
//! A desktop app (`start_desktop`) has no runtime, bridge or key: its
//! row is written first and ended the same way. Nothing else in the app
//! calls the supervisor's spawns for a session (the guardrail
//! `every_session_starts_through_the_one_spawn_path` holds it).

use std::path::PathBuf;
use std::sync::Arc;

use juggler_adapter::{HarnessAdapter, LaunchSpec, Runtime};
use juggler_core::model::{
    Account, AttentionReason, ConversationId, InstanceKind, InstanceRecord, Profile, ProfileId,
    ProjectId, SessionId, SessionStatus, Timestamp, TranscriptRef, WindowIdentity,
};
use juggler_supervisor::{DriverPipes, SpawnedProcess};

use crate::commands::{engine_usage_snapshot, local_machine_id, lock_storage_arc};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The attention rule of a session whose process never started (the
/// reason, in words, is its one input).
pub const LAUNCH_FAILED_RULE: &str = "launch_failed";

/// What a new session's row says before its process starts.
#[derive(Clone)]
pub(crate) struct NewSession<'a> {
    /// The id the supervisor will run it under.
    pub id: &'a SessionId,
    /// The account it runs on (its tool, its key, its ledger rows).
    pub account: &'a Account,
    /// Its launch settings.
    pub profile: &'a ProfileId,
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

impl NewSession<'_> {
    /// Its project as the ledger and the agent bridge name it.
    fn project_name(&self) -> Option<String> {
        self.project.as_ref().map(ToString::to_string)
    }
}

/// How a session runs.
pub(crate) enum Runs<'a> {
    /// In its terminal, as this launch says (a resume, a switch, a
    /// continue: the tool's own terminal UI).
    Terminal(LaunchSpec),
    /// In the runtime its plan picks (ADR-0022), this terminal launch the
    /// fallback (and what runs for a terminal plan).
    Planned {
        /// Its launch settings, for the driven launch.
        profile: &'a Profile,
        /// The tool's conversation to continue, when resuming.
        resume: Option<String>,
        /// The user's choice for this session, when they made one.
        requested: Option<Runtime>,
        /// The terminal launch.
        terminal: LaunchSpec,
    },
}

/// What started.
pub struct Started {
    /// The session's id: the row's own, or a new one when a driven start
    /// failed after its process was spawned (that id is spent).
    pub session: SessionId,
    /// The process.
    pub spawned: SpawnedProcess,
    /// Its runtime.
    pub runtime: Runtime,
}

/// A start that failed, and the session whose row it ends.
pub(crate) struct Unstarted {
    /// The row to end `Failed`.
    pub session: SessionId,
    /// Why.
    pub error: AppError,
}

impl Unstarted {
    /// `error` ends `session`'s row.
    pub(crate) fn of(session: &SessionId, error: impl Into<AppError>) -> Self {
        Unstarted {
            session: session.clone(),
            error: error.into(),
        }
    }
}

/// Starts `row`'s session as `runs` says: its row first, then its
/// runtime, the bridge and its key, the supervisor; the start counted, or
/// its row ended `Failed` with the reason.
///
/// # Errors
///
/// Storage errors writing the row (nothing is spawned then), the
/// account's key missing from this Mac's Keychain, and the supervisor's
/// spawn error.
pub(crate) fn start_session(
    state: &Arc<AppState>,
    row: &NewSession<'_>,
    runs: Runs<'_>,
) -> AppResult<Started> {
    let adapter = state
        .registry
        .get(&row.account.harness)
        .ok_or_else(|| AppError::UnknownHarness(row.account.harness.to_string()))?;
    write_starting(state, row, InstanceKind::Terminal)?;
    let started = match runs {
        Runs::Terminal(mut spec) => spawn_process(
            state,
            adapter.as_ref(),
            row,
            row.id,
            &mut spec,
            Io::Owned,
            false,
        )
        .map(|(spawned, _)| Started {
            session: row.id.clone(),
            spawned,
            runtime: Runtime::Terminal,
        })
        .map_err(|error| Unstarted::of(row.id, error)),
        Runs::Planned {
            profile,
            resume,
            requested,
            terminal,
        } => crate::features::runtime::launch::start(
            state,
            adapter.as_ref(),
            row,
            &crate::features::runtime::launch::Plan {
                profile,
                resume,
                requested,
            },
            terminal,
        ),
    };
    match started {
        Ok(started) => {
            counted(row.account, InstanceKind::Terminal, row.origin);
            Ok(started)
        }
        Err(unstarted) => {
            launch_failed(state, &unstarted.session, &unstarted.error);
            Err(unstarted.error)
        }
    }
}

/// Starts the account's desktop app as `row`'s session (AJ-E413): its
/// row first, then the supervisor; its window recorded once its process
/// is known.
///
/// # Errors
///
/// As [`spawn_tracked`].
pub(crate) fn start_desktop(
    state: &Arc<AppState>,
    row: &NewSession<'_>,
    launch: &juggler_adapter::DesktopLaunch,
) -> AppResult<SpawnedProcess> {
    let spawned = spawn_tracked(state, row, InstanceKind::Desktop, || {
        let ctx = crate::commands::ledger_ctx(state, row.account, row.project_name());
        Ok(state.supervisor.spawn_desktop(row.id, launch, &ctx)?)
    })?;
    // Window identity recorded at launch (AJ-E414 Req.2): the owning PID
    // + bundle id; the title is adopted at first sight (first focus
    // attempt) and re-discovered thereafter.
    record_window(
        state,
        row.id,
        WindowIdentity {
            pid: spawned.pid,
            bundle_id: launch.bundle_id.clone(),
            title: None,
            cgwindow_id: None,
            start_time: None,
        },
    )?;
    Ok(spawned)
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
    kind: InstanceKind,
    spawn: impl FnOnce() -> AppResult<T>,
) -> AppResult<T> {
    write_starting(state, row, kind)?;
    match spawn() {
        Ok(spawned) => {
            counted(row.account, kind, row.origin);
            Ok(spawned)
        }
        Err(error) => {
            launch_failed(state, row.id, &error);
            Err(error)
        }
    }
}

/// Whose output a spawned process's pipes go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Io {
    /// The supervisor's: its terminal, or its output ring.
    Owned,
    /// The caller's: a driven session's stdin and stdout, for its
    /// runtime's codec (AJ-E2302).
    Driver,
}

/// Steps 3 to 5 for one process of `row`'s session, run as `id`: the
/// agent bridge into `spec` (so the caller's spec is what ran), with
/// `state_reports` the tool's own hooks after it (AJ-E2306, a terminal
/// the runtime started), the account's key into this launch only, then
/// the supervisor.
///
/// # Errors
///
/// The account's key missing from this Mac's Keychain, and the
/// supervisor's spawn error.
pub(crate) fn spawn_process(
    state: &Arc<AppState>,
    adapter: &dyn HarnessAdapter,
    row: &NewSession<'_>,
    id: &SessionId,
    spec: &mut LaunchSpec,
    io: Io,
    state_reports: bool,
) -> AppResult<(SpawnedProcess, Option<DriverPipes>)> {
    let project = row.project_name();
    crate::features::agent_bridge::prepare(
        state,
        adapter,
        spec,
        id,
        project.as_deref(),
        row.account.id.as_str(),
    );
    if state_reports {
        crate::features::runtime::reports::prepare(state, adapter, spec, id);
    }
    let launched = launched_spec(state, row.account, spec)?;
    let ctx = crate::commands::ledger_ctx(state, row.account, project);
    match io {
        Io::Owned => Ok((state.supervisor.spawn(id, &launched, &ctx)?, None)),
        Io::Driver => {
            let (spawned, pipes) = state.supervisor.spawn_driven(id, &launched, &ctx)?;
            Ok((spawned, Some(pipes)))
        }
    }
}

/// AJ-E2304: the tool's own terminal UI on a driven session's
/// conversation, as `id`: an owned child like a sign-in terminal (no
/// session row), under the session's account, with its key.
///
/// # Errors
///
/// As [`spawn_process`].
pub(crate) fn spawn_view(
    state: &Arc<AppState>,
    id: &SessionId,
    account: &Account,
    spec: &LaunchSpec,
) -> AppResult<SpawnedProcess> {
    let launched = launched_spec(state, account, spec)?;
    let mut ctx = crate::commands::ledger_ctx(state, account, None);
    ctx.role = juggler_core::storage::ROLE_LOGIN;
    Ok(state.supervisor.spawn(id, &launched, &ctx)?)
}

/// `spec` as `account`'s session starts it: with the account's key in its
/// environment when it runs through another service, else as it is.
///
/// # Errors
///
/// As `crate::features::service_accounts::session_env`: an account
/// whose key isn't in this Mac's Keychain starts nothing.
pub fn launched_spec(
    state: &Arc<AppState>,
    account: &Account,
    spec: &LaunchSpec,
) -> AppResult<LaunchSpec> {
    let mut launched = spec.clone();
    if let Some((name, key)) = crate::features::service_accounts::session_env(state, account)? {
        launched.env.insert(name, key);
    }
    Ok(launched)
}

/// Step 1: `row` as a `Starting` session of `kind`. A session resumed
/// under its own id (its turn cut off when AI Juggler stopped, AJ-E2307)
/// has its row already: it is starting again.
///
/// # Errors
///
/// Storage errors.
pub(crate) fn write_starting(
    state: &Arc<AppState>,
    row: &NewSession<'_>,
    kind: InstanceKind,
) -> AppResult<()> {
    // The machine identity locks storage itself: resolved first.
    let machine = local_machine_id(state);
    let storage = lock_storage_arc(state)?;
    if let Ok(mut record) = storage.sessions().get(row.id) {
        record.status = SessionStatus::Starting;
        record.attention = None;
        record.last_activity = Some(Timestamp::now());
        storage.sessions().update(&record)?;
        return Ok(());
    }
    let usage = engine_usage_snapshot(&storage, row.profile);
    storage.sessions().insert(&InstanceRecord {
        id: row.id.clone(),
        conversation: row.conversation.clone(),
        profile: row.profile.clone(),
        machine,
        kind,
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
    Ok(())
}

/// A start, for usage statistics.
fn counted(account: &Account, kind: InstanceKind, origin: &'static str) {
    crate::usage_stats::instance_started(
        account.harness.as_str(),
        kind == InstanceKind::Desktop,
        origin,
    );
}

/// Ends a session whose process never started: `Failed`, the reason
/// its one input. Only while it is still `Starting` (nothing else has
/// said anything about it); a storage failure here leaves it to the
/// next start's reconciliation, which ends every row it can't adopt.
pub(crate) fn launch_failed(state: &Arc<AppState>, id: &SessionId, error: &AppError) {
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

    fn exits_at_once() -> LaunchSpec {
        LaunchSpec {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), "exit 0".into()],
            env: juggler_adapter::EnvMap::default(),
            cwd: std::env::temp_dir(),
            needs_pty: false,
        }
    }

    /// Applies `state`'s lifecycle events in the order the supervisor sent
    /// them (as the forwarder would) until `id`'s exit; returns its
    /// Started, to replay.
    fn drain_until_exit(state: &Arc<AppState>) -> Option<LifecycleEvent> {
        let mut started = None;
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
            let storage = lock_storage_arc(state).unwrap();
            crate::commands::apply_lifecycle_event(&storage, event, false, None);
            if exited {
                return started;
            }
        }
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
        let spec = exits_at_once();
        let ctx = crate::commands::ledger_ctx(&state, &account, None);
        let mut started = None;
        spawn_tracked(
            &state,
            &row(&id, &account, &profile),
            InstanceKind::Terminal,
            || {
                let spawned = state.supervisor.spawn(&id, &spec, &ctx)?;
                started = drain_until_exit(&state);
                Ok(spawned)
            },
        )
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

    /// The one path end to end: a session whose process exits at once
    /// starts through `start_session`, and its exit lands on the row its
    /// start wrote first.
    #[test]
    fn a_session_started_through_the_one_path_ends_on_its_own_row() {
        let dir = tempfile::tempdir().unwrap();
        let (state, account, profile) = fixture(dir.path());
        let id = SessionId::new("sess-one-path");
        let started = start_session(
            &state,
            &row(&id, &account, &profile),
            Runs::Terminal(exits_at_once()),
        )
        .unwrap();
        assert_eq!(started.session, id);
        assert_eq!(started.runtime, Runtime::Terminal);
        drain_until_exit(&state);
        assert_eq!(status(&state, &id).status, SessionStatus::Finished);
    }

    #[test]
    fn a_spawn_that_fails_ends_its_row_failed_with_the_reason() {
        let dir = tempfile::tempdir().unwrap();
        let (state, account, profile) = fixture(dir.path());
        let id = SessionId::new("sess-never-starts");
        let result: AppResult<()> = spawn_tracked(
            &state,
            &row(&id, &account, &profile),
            InstanceKind::Terminal,
            || Err(AppError::Refused("no such program".to_owned())),
        );
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

    /// Through the one path too: a program that isn't there ends the
    /// row the start wrote, `Failed`, with the supervisor's reason.
    #[test]
    fn a_session_whose_program_is_missing_ends_failed_through_the_one_path() {
        let dir = tempfile::tempdir().unwrap();
        let (state, account, profile) = fixture(dir.path());
        let id = SessionId::new("sess-no-program");
        let mut spec = exits_at_once();
        spec.program = dir.path().join("not-a-program");
        let result = start_session(&state, &row(&id, &account, &profile), Runs::Terminal(spec));
        assert!(result.is_err());
        let record = status(&state, &id);
        assert_eq!(record.status, SessionStatus::Failed);
        assert_eq!(record.attention.unwrap().rule, LAUNCH_FAILED_RULE);
    }

    #[test]
    fn the_row_exists_as_starting_while_the_process_starts() {
        let dir = tempfile::tempdir().unwrap();
        let (state, account, profile) = fixture(dir.path());
        let id = SessionId::new("sess-seen-starting");
        let seen = spawn_tracked(
            &state,
            &row(&id, &account, &profile),
            InstanceKind::Terminal,
            || Ok(status(&state, &id).status),
        )
        .unwrap();
        assert_eq!(seen, SessionStatus::Starting);
    }

    /// A session resumed under its own id keeps its row: it starts again.
    #[test]
    fn a_session_resumed_under_its_own_id_starts_again_on_its_row() {
        let dir = tempfile::tempdir().unwrap();
        let (state, account, profile) = fixture(dir.path());
        let id = SessionId::new("sess-comes-back");
        let mut first = row(&id, &account, &profile);
        first.origin = "new";
        write_starting(&state, &first, InstanceKind::Terminal).unwrap();
        {
            let storage = lock_storage_arc(&state).unwrap();
            let mut record = storage.sessions().get(&id).unwrap();
            record.status = SessionStatus::WaitingUser;
            storage.sessions().update(&record).unwrap();
        }
        let seen = spawn_tracked(&state, &first, InstanceKind::Terminal, || {
            Ok(status(&state, &id).status)
        })
        .unwrap();
        assert_eq!(seen, SessionStatus::Starting);
    }
}
