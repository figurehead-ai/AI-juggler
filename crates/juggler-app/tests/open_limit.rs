//! 0.3: a session that hits its usage limit and stays open says so.
//!
//! An interactive tool shows its limit and keeps running, so the exit
//! classifier never marked it. Here a real `fake-harness` process stops at
//! a limit (its own `limit` line, as a tool records one) and stays open:
//! the session is at its limit while its process runs, Needs you and its
//! row say so, it carries on when the model replies, a hit whose reset
//! passed marks nothing, and quitting the tool at its limit keeps it there.
//! A driven session's runtime word marks it too, and its next turn clears
//! it. Assertion messages print no data.

// Unix only: the fixture is stopped by a signal.
#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapter::runtime::{ErrorCategory, RuntimeEvent};
use juggler_app::features::sessions::{lifecycle, limits};
use juggler_app::state::AppState;
use juggler_core::model::{SessionId, SessionStatus, Timestamp, TranscriptRef};

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::FakeHarnessAdapter::new(
        common::fake_harness::bin(),
    )));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn record(state: &Arc<AppState>, id: &SessionId) -> juggler_core::model::InstanceRecord {
    state.storage.lock().unwrap().sessions().get(id).unwrap()
}

fn append(path: &Path, line: &str) {
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(file, "{line}").unwrap();
}

/// Launch settings in `folder` whose session stops at a limit resetting
/// in an hour, and stays open.
fn launch_at_limit(state: &Arc<AppState>, folder: &Path) -> (SessionId, PathBuf) {
    let account = juggler_app::commands::create_account(state, "fake", "Work Max", None).unwrap();
    let profile = juggler_app::commands::create_profile(
        state,
        &account.id,
        "Main",
        None,
        Some(folder.to_string_lossy().into_owned()),
    )
    .unwrap();
    {
        let storage = state.storage.lock().unwrap();
        let mut recipe = storage
            .profiles()
            .get(&juggler_core::model::ProfileId::new(profile.id.as_str()))
            .unwrap();
        recipe
            .settings_overlay
            .env
            .insert("FAKE_HARNESS_LIMIT_HIT_MS".into(), "3600000".into());
        storage.profiles().update(&recipe).unwrap();
    }
    let session = juggler_app::commands::launch_profile(state, &profile.id, None)
        .unwrap()
        .session_id;
    let id = SessionId::new(session.as_str());
    let transcripts = state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .get(&juggler_core::model::AccountId::new(account.id.as_str()))
        .unwrap()
        .state_dir
        .join("transcripts");
    let mut transcript = None;
    wait_for("the limit line", || {
        transcript = std::fs::read_dir(&transcripts)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                std::fs::read_to_string(path).is_ok_and(|text| text.contains("\"limit\""))
            });
        transcript.is_some()
    });
    let transcript = transcript.unwrap();
    // Linked as the watcher or the tool's hooks link it.
    {
        let storage = state.storage.lock().unwrap();
        let mut row = storage.sessions().get(&id).unwrap();
        if row.transcript_ref.is_none() {
            row.transcript_ref = Some(TranscriptRef {
                path: transcript.clone(),
                native_id: None,
            });
            storage.sessions().update(&row).unwrap();
        }
    }
    (id, transcript)
}

#[test]
fn an_open_session_at_its_limit_says_so_until_it_carries_on() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = dir.path().canonicalize().unwrap();
    let state = app_state(&root);
    let folder = root.join("shop");
    std::fs::create_dir_all(&folder).unwrap();
    let (id, transcript) = launch_at_limit(&state, &folder);

    // The hit marks it while its process still runs.
    wait_for("the session at its limit", || {
        limits::check(&state, &id);
        record(&state, &id).status == SessionStatus::QuotaBlocked
    });
    assert!(!state.supervisor.is_finished(&id), "the tool is still open");
    let marked = record(&state, &id);
    let reason = marked.attention.as_ref().unwrap();
    assert_eq!(reason.rule, limits::RULE);
    // Its row and Needs you say it hit its limit.
    let row = juggler_app::commands::instances_shared_snapshot(&state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == id.as_str())
        .unwrap();
    assert_eq!(row.status, "quota_blocked");
    assert!(
        row.needs_attention.is_some(),
        "an open session at its limit needs the user"
    );
    // Looked at again with nothing new: nothing changes.
    assert!(!limits::check(&state, &id));
    // Parked open, "Resume" takes it off the queue and carries on in the
    // same session: never a second copy writing the same conversation.
    juggler_app::commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();
    juggler_app::parked::instance_park_impl(&state, id.as_str()).unwrap();
    let resumed = juggler_app::parked::parked_resume_impl(&state, id.as_str()).unwrap();
    assert_eq!(resumed.session_id, id.as_str());
    assert!(!state.supervisor.is_finished(&id));
    assert!(juggler_app::parked::parked_list_impl(&state).is_empty());
    assert_eq!(record(&state, &id).status, SessionStatus::QuotaBlocked);

    // The model replied: the session carried on.
    append(
        &transcript,
        r#"{"type":"assistant","turn":3,"usage":{"input_tokens":1,"output_tokens":1}}"#,
    );
    assert!(limits::check(&state, &id), "the reply clears it");
    let carried = record(&state, &id);
    assert_eq!(carried.status, SessionStatus::Running);
    assert!(carried.attention.is_none());

    // A hit whose reset already passed marks nothing: the account has room.
    let now = Timestamp::now().millis();
    append(
        &transcript,
        &format!(
            r#"{{"type":"limit","at_ms":{now},"resets_at_ms":{}}}"#,
            now - 1
        ),
    );
    assert!(!limits::check(&state, &id));
    assert_eq!(record(&state, &id).status, SessionStatus::Running);

    // A new hit marks it again; the tool quits there (not stopped from AI
    // Juggler): it ended at its limit, for the switch and the reset queue.
    append(
        &transcript,
        &format!(
            r#"{{"type":"limit","at_ms":{},"resets_at_ms":{}}}"#,
            now + 1,
            now + 3_600_000
        ),
    );
    assert!(limits::check(&state, &id));
    assert_eq!(record(&state, &id).status, SessionStatus::QuotaBlocked);
    let pid = state.supervisor.root_identity(&id).unwrap().pid;
    let killed = std::process::Command::new("kill")
        .arg("-INT")
        .arg(pid.to_string())
        .status()
        .unwrap();
    assert!(killed.success());
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(Instant::now() < deadline, "no exit event");
        if let Some(event) = state.supervisor.recv_timeout(Duration::from_millis(100)) {
            let exited = matches!(event, juggler_supervisor::LifecycleEvent::Exited { .. });
            lifecycle::forward(&state, event);
            if exited {
                break;
            }
        }
    }
    let ended = record(&state, &id);
    assert_eq!(ended.status, SessionStatus::QuotaBlocked);
    assert_eq!(ended.attention.unwrap().rule, limits::EXIT_RULE);
    // Ended, its mark is the exit's: never cleared as an open one.
    assert!(!limits::check(&state, &id));
}

#[test]
fn a_driven_session_is_marked_by_its_runtime_and_cleared_by_its_next_turn() {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = dir.path().canonicalize().unwrap();
    let state = app_state(&root);
    let folder = root.join("shop");
    std::fs::create_dir_all(&folder).unwrap();
    let account = juggler_app::commands::create_account(&state, "fake", "Work Max", None).unwrap();
    let profile = juggler_app::commands::create_profile(
        &state,
        &account.id,
        "Main",
        None,
        Some(folder.to_string_lossy().into_owned()),
    )
    .unwrap();
    let id = SessionId::new("s-driven");
    {
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .insert(&juggler_core::model::InstanceRecord {
                id: id.clone(),
                conversation: None,
                profile: juggler_core::model::ProfileId::new(profile.id.as_str()),
                machine: juggler_core::model::MachineId::new("local"),
                kind: juggler_core::model::InstanceKind::Terminal,
                project: None,
                cwd: folder.clone(),
                status: SessionStatus::Running,
                started_at: Timestamp::now(),
                last_activity: None,
                usage: Default::default(),
                transcript_ref: None,
                attention: None,
                running: None,
                window: None,
            })
            .unwrap();
    }
    // A limit the tool retries past is no hit; another error neither.
    let error = |category, will_retry| RuntimeEvent::Error {
        category,
        message: "words".into(),
        will_retry,
    };
    assert!(!limits::driven(
        &state,
        &id,
        &error(ErrorCategory::RateLimit, true),
        None
    ));
    assert!(!limits::driven(
        &state,
        &id,
        &error(ErrorCategory::Other, false),
        None
    ));
    assert_eq!(record(&state, &id).status, SessionStatus::Running);
    // One it won't retry past marks it.
    assert!(limits::driven(
        &state,
        &id,
        &error(ErrorCategory::RateLimit, false),
        None
    ));
    let marked = record(&state, &id);
    assert_eq!(marked.status, SessionStatus::QuotaBlocked);
    assert_eq!(marked.attention.unwrap().rule, limits::RULE);
    // Its next turn clears it.
    let turn = RuntimeEvent::TurnStarted {
        turn: None,
        origin: None,
    };
    assert!(limits::driven(&state, &id, &turn, None));
    assert_eq!(record(&state, &id).status, SessionStatus::Running);
    assert!(!limits::driven(&state, &id, &turn, None), "once");
}
