//! AJ-E1520 app-level: the status row's subagent count (I-08) and the
//! inspector's list, from a live session's transcript as it grows, on
//! the tick's incremental read — and from the file once the session ended.
//! The app reads Claude's and Codex's transcripts the same way.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::commands;
use juggler_app::features::subtasks;
use juggler_app::features::subtasks::dto::SubtaskStateInfo;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId, ProfileId,
    SessionId, SessionStatus, SettingsOverlay, Timestamp, TranscriptRef,
};

fn seeded_state(dir: &Path, launch_model: Option<&str>) -> Arc<AppState> {
    let state = Arc::new(
        AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-sub"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Work Max".into(),
                plan: None,
                state_dir: dir.join("state"),
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
                id: ProfileId::new("rec-sub"),
                name: "Main".into(),
                account: AccountId::new("acc-sub"),
                settings_overlay: SettingsOverlay {
                    model: launch_model.map(str::to_string),
                    env: Default::default(),
                    restart_on_crash: None,
                },
                default_cwd: None,
            })
            .unwrap();
    }
    state
}

fn seed_instance(state: &Arc<AppState>, dir: &Path, id: &str, transcript: &Path) -> SessionId {
    let session = SessionId::new(id);
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: ProfileId::new("rec-sub"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            transcript_ref: Some(TranscriptRef {
                native_id: Some("native-1".into()),
                path: transcript.to_path_buf(),
            }),
            attention: None,
            running: None,
            window: None,
        })
        .unwrap();
    session
}

fn task_call(id: &str, description: &str) -> String {
    format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"{id}\",\"name\":\"Task\",\"input\":{{\"description\":\"{description}\",\"subagent_type\":\"Explore\"}}}}]}}}}\n"
    )
}

fn task_answer(id: &str) -> String {
    format!(
        "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"{id}\",\"content\":\"done\"}}]}}}}\n"
    )
}

fn append(path: &Path, text: &str) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

/// The count on the session's row, as the UI receives it.
fn count(state: &Arc<AppState>, session: &SessionId) -> Option<u64> {
    let rows = {
        let storage = state.storage.lock().unwrap();
        commands::session_infos_snapshot(&storage).unwrap()
    };
    subtasks::counted(state, rows)
        .into_iter()
        .find(|row| row.id == session.as_str())
        .unwrap()
        .subtask_count
}

fn states(state: &Arc<AppState>, session: &SessionId) -> Vec<(String, SubtaskStateInfo)> {
    subtasks::list(state, session.as_str())
        .unwrap()
        .items
        .into_iter()
        .map(|item| (item.label, item.state))
        .collect()
}

#[test]
fn the_count_follows_a_live_transcript_and_the_list_outlives_the_session() {
    let dir = tempfile::tempdir().unwrap();
    let state = seeded_state(dir.path(), None);
    let transcript = dir.path().join("t.jsonl");
    let session = seed_instance(&state, dir.path(), "sess-sub", &transcript);

    // Not read yet (no transcript): unknown, never a guessed 0.
    assert_eq!(count(&state, &session), None);
    assert!(!subtasks::tick(&state), "nothing to read yet");
    assert_eq!(count(&state, &session), None);

    append(&transcript, &task_call("t1", "Map the parsers"));
    append(&transcript, &task_call("t2", "Review errors"));
    assert!(subtasks::tick(&state), "two started");
    assert_eq!(count(&state, &session), Some(2));

    append(&transcript, &task_answer("t1"));
    assert!(subtasks::tick(&state));
    assert_eq!(count(&state, &session), Some(1));
    assert!(
        !subtasks::tick(&state),
        "nothing appended: no change, no emit"
    );
    assert_eq!(
        states(&state, &session),
        vec![
            ("Map the parsers".to_owned(), SubtaskStateInfo::Done),
            ("Review errors".to_owned(), SubtaskStateInfo::Running),
        ]
    );

    // The session ends with t2 unanswered: the row stops counting, and
    // the list (read from the file now) says it never finished.
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage.sessions().get(&session).unwrap();
        record.status = SessionStatus::Finished;
        storage.sessions().update(&record).unwrap();
    }
    assert!(subtasks::tick(&state), "a running subagent left the count");
    assert_eq!(count(&state, &session), None);
    assert_eq!(
        states(&state, &session),
        vec![
            ("Map the parsers".to_owned(), SubtaskStateInfo::Done),
            ("Review errors".to_owned(), SubtaskStateInfo::Unfinished),
        ]
    );
}

#[test]
fn a_codex_session_is_counted_the_same_way() {
    let dir = tempfile::tempdir().unwrap();
    let state = seeded_state(dir.path(), None);
    let transcript = dir.path().join("rollout.jsonl");
    let t = "\"timestamp\":\"2026-10-23T09:00:00.000Z\"";
    std::fs::write(
        &transcript,
        format!(
            "{{{t},\"type\":\"session_meta\",\"payload\":{{\"id\":\"thread-1\"}}}}\n\
             {{{t},\"type\":\"response_item\",\"payload\":{{\"type\":\"function_call\",\"name\":\"spawn_agent\",\"arguments\":\"{{\\\"message\\\":\\\"Map the parsers\\\"}}\",\"call_id\":\"c1\"}}}}\n\
             {{{t},\"type\":\"response_item\",\"payload\":{{\"type\":\"function_call_output\",\"call_id\":\"c1\",\"output\":\"{{\\\"agent_id\\\":\\\"a1\\\"}}\"}}}}\n"
        ),
    )
    .unwrap();
    let session = seed_instance(&state, dir.path(), "sess-codex", &transcript);
    assert!(subtasks::tick(&state));
    assert_eq!(count(&state, &session), Some(1));
    assert_eq!(
        states(&state, &session),
        vec![("Map the parsers".to_owned(), SubtaskStateInfo::Running)]
    );
}

/// A session that moves to a new transcript (a resume, a `/clear`) is read
/// from the new file, and a count that drops is reported as a change.
#[test]
fn a_new_transcript_path_is_read_afresh_and_its_drop_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let state = seeded_state(dir.path(), None);
    let first = dir.path().join("first.jsonl");
    append(&first, &task_call("t1", "Map the parsers"));
    append(&first, &task_call("t2", "Review errors"));
    let session = seed_instance(&state, dir.path(), "sess-moved", &first);
    assert!(subtasks::tick(&state));
    assert_eq!(count(&state, &session), Some(2));

    let second = dir.path().join("second.jsonl");
    append(&second, &task_call("t9", "Write tests"));
    append(&second, &task_answer("t9"));
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage.sessions().get(&session).unwrap();
        record.transcript_ref = Some(TranscriptRef {
            native_id: Some("native-2".into()),
            path: second.clone(),
        });
        storage.sessions().update(&record).unwrap();
    }
    assert!(subtasks::tick(&state), "2 running → 0 is a change");
    assert_eq!(count(&state, &session), Some(0));
    assert_eq!(
        states(&state, &session),
        vec![("Write tests".to_owned(), SubtaskStateInfo::Done)]
    );
}
