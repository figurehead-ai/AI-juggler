//! AJ-E1540: a session that knows its conversation is switched by it.
//!
//! A conversation started outside AI Juggler and picked up later ("Continue
//! in AI Juggler", or a session recorded from outside) has its id on the
//! session record, but its file is older than the record. Finding the
//! source by "a file created after the launch" then found nothing, and
//! every such switch failed with "Couldn't tell which file is this
//! session" (found by the live lab, AJ-E1539, on Claude Code 2.1.302).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "support/set_up.rs"]
mod set_up;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::{
    ConversationId, InstanceKind, InstanceRecord, SessionId, SessionStatus, Timestamp,
    TranscriptRef,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    use std::os::unix::fs::PermissionsExt;
    let stub = dir.join("claude-stub");
    std::fs::write(&stub, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// A small, real-shaped Claude transcript for conversation `id` in `cwd`.
fn seed(path: &Path, id: &str, cwd: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let line = |uuid: &str, kind: &str, text: &str, at: &str| {
        format!(
            "{{\"type\":\"{kind}\",\"uuid\":\"{uuid}\",\"sessionId\":\"{id}\",\"cwd\":\"{cwd}\",\
             \"timestamp\":\"{at}\",\"message\":{{\"role\":\"{kind}\",\"content\":\"{text}\"}}}}\n"
        )
    };
    let mut body = line(
        "u1",
        "user",
        "add a calc module",
        "2026-10-22T08:00:00.000Z",
    );
    body.push_str(&line(
        "u2",
        "assistant",
        "calc.py is written",
        "2026-10-22T08:00:05.000Z",
    ));
    std::fs::write(path, body).unwrap();
}

#[test]
fn a_session_that_knows_its_conversation_switches_by_it() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project = work.path().join("lab");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();

    let account_a = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Work", None).unwrap(),
    );
    let account_b = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap(),
    );
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();

    // The conversation was started outside AI Juggler, before the record.
    let native = "0b5e0e5c-6c0b-4c55-9a6b-2a7d3c9f1e11";
    let a_dir = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(&account_a.id);
    let transcript: PathBuf = a_dir
        .join("projects")
        .join(juggler_adapter::handover::mangled_dir(&cwd))
        .join(format!("{native}.jsonl"));
    seed(&transcript, native, &cwd);

    let session = SessionId::new("picked-up");
    let machine = commands::local_machine_id(&state);
    {
        let storage = state.storage.lock().unwrap();
        let project_row = storage
            .projects()
            .touch(&juggler_core::storage::project_root(&project), None)
            .unwrap();
        let profile = storage
            .profiles()
            .list_by_account(&juggler_core::model::AccountId::new(account_a.id.clone()))
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(profile.id.as_str(), profile_a.id);
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: session.clone(),
                conversation: Some(ConversationId::new(native)),
                profile: profile.id,
                machine,
                kind: InstanceKind::Terminal,
                project: Some(project_row.id),
                cwd: project.clone(),
                status: SessionStatus::Finished,
                // Picked up a minute after the conversation began.
                started_at: Timestamp::from_millis(Timestamp::now().millis() + 60_000),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                window: None,
                transcript_ref: None,
            })
            .unwrap();
    }

    let switched = commands::switch_execute_impl(
        &state,
        session.as_str(),
        &profile_b.id,
        "stop-now",
        None,
        &[],
    )
    .expect("the switch finds the conversation by its id");

    let installed: TranscriptRef = {
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .get(&SessionId::new(switched.session_id.clone()))
            .unwrap()
            .transcript_ref
            .expect("the continuation is linked to its transcript")
    };
    assert_eq!(installed.native_id.as_deref(), Some(native));
    assert!(
        installed.path.starts_with(
            state
                .accounts_dir()
                .join(CLAUDE_CODE_ID)
                .join(&account_b.id)
        )
    );
    assert_eq!(
        std::fs::read(&installed.path).unwrap(),
        std::fs::read(&transcript).unwrap(),
        "B continues the same conversation, byte for byte"
    );
}

/// AJ-E1541: A → B → C, then back to where it started. Every switch after
/// the first starts from the epoch the previous switch made, so the chain is
/// one path: the plan offers B and A, and returning to A carries what was
/// new on B and on C (found by the live lab: each switch started a new
/// chain, and "back" could only reach the last hop).
#[test]
fn a_chain_of_switches_returns_to_where_it_started_with_every_hop() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project = work.path().join("lab");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let account = |label: &str| {
        let account = set_up::set_up(
            &state,
            commands::create_account(&state, CLAUDE_CODE_ID, label, None).unwrap(),
        );
        let profile = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
        (account.id, profile.id)
    };
    let (a, profile_a) = account("Alpha");
    let (b, profile_b) = account("Bravo");
    let (_c, profile_c) = account("Charlie");
    let file_of = |account: &str, native: &str| {
        state
            .accounts_dir()
            .join(CLAUDE_CODE_ID)
            .join(account)
            .join("projects")
            .join(juggler_adapter::handover::mangled_dir(&cwd))
            .join(format!("{native}.jsonl"))
    };
    let append = |path: &std::path::Path, uuid: &str, kind: &str, text: &str| {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        writeln!(
            file,
            "{{\"type\":\"{kind}\",\"uuid\":\"{uuid}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"{kind}\",\"content\":\"{text}\"}}}}"
        )
        .unwrap();
    };
    let native = "0b5e0e5c-6c0b-4c55-9a6b-2a7d3c9f1e22";
    let a_file = file_of(&a, native);
    seed(&a_file, native, &cwd);

    let machine = commands::local_machine_id(&state);
    let session = SessionId::new("chain-start");
    {
        let storage = state.storage.lock().unwrap();
        let project_row = storage
            .projects()
            .touch(&juggler_core::storage::project_root(&project), None)
            .unwrap();
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: session.clone(),
                conversation: Some(ConversationId::new(native)),
                profile: juggler_core::model::ProfileId::new(profile_a.clone()),
                machine,
                kind: InstanceKind::Terminal,
                project: Some(project_row.id),
                cwd: project.clone(),
                status: SessionStatus::Finished,
                started_at: Timestamp::from_millis(Timestamp::now().millis() + 60_000),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                window: None,
                transcript_ref: None,
            })
            .unwrap();
    }

    let on_b =
        commands::switch_execute_impl(&state, session.as_str(), &profile_b, "stop-now", None, &[])
            .unwrap();
    append(
        &file_of(&b, native),
        "b1",
        "user",
        "name the multiply function times",
    );
    let on_c =
        commands::switch_execute_impl(&state, &on_b.session_id, &profile_c, "stop-now", None, &[])
            .unwrap();
    let c_file = {
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .get(&SessionId::new(on_c.session_id.clone()))
            .unwrap()
            .transcript_ref
            .unwrap()
            .path
    };
    append(
        &c_file,
        "c1",
        "assistant",
        "the release name is Copper Kettle",
    );

    let plan = commands::switch_back_plan_impl(&state, &on_c.session_id).unwrap();
    let offered: Vec<(&str, bool)> = plan
        .returns
        .iter()
        .map(|r| (r.account.as_str(), r.start))
        .collect();
    assert_eq!(offered, vec![("Bravo", false), ("Alpha", true)]);
    let start = plan
        .returns
        .iter()
        .find(|r| r.start)
        .unwrap()
        .epoch_id
        .clone();

    let back = commands::switch_back_plan_to_impl(&state, &on_c.session_id, Some(&start)).unwrap();
    assert_eq!(back.original_account, "Alpha");
    let delta = &back.delta_preview;
    assert!(
        delta.starts_with("While you were away, this work continued on Bravo, then Charlie"),
        "{delta}"
    );
    assert!(
        delta.contains("name the multiply function times"),
        "{delta}"
    );
    assert!(delta.contains("Copper Kettle"), "{delta}");
    assert!(
        !delta.contains("calc.py is written"),
        "A's own turns aren't news to A: {delta}"
    );
}

/// AJ-E1540: the freeze choice is real. A source session AI Juggler started
/// and that is still running stops before its conversation is copied: at
/// once for "Stop now", after its turn for "Finish this turn". Before, the
/// choice was dropped and the source kept running beside its continuation.
fn freezing(freeze: &str) {
    use std::os::unix::fs::PermissionsExt;
    let work = tempfile::tempdir().unwrap();
    // A tool that keeps running and says nothing: a session between turns.
    let stub = work.path().join("claude-busy");
    std::fs::write(&stub, "#!/bin/sh\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());
    let project = work.path().join("lab");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let account_a = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Work", None).unwrap(),
    );
    let account_b = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap(),
    );
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();
    let source = SessionId::new(launched.session_id.clone());
    let native = "0b5e0e5c-6c0b-4c55-9a6b-2a7d3c9f1e33";
    let transcript = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(&account_a.id)
        .join("projects")
        .join(juggler_adapter::handover::mangled_dir(&cwd))
        .join(format!("{native}.jsonl"));
    seed(&transcript, native, &cwd);
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage.sessions().get(&source).unwrap();
        record.transcript_ref = Some(TranscriptRef {
            path: transcript.clone(),
            native_id: Some(native.to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }
    assert!(
        !state.supervisor.is_finished(&source),
        "the source is running"
    );

    commands::switch_execute_impl(&state, source.as_str(), &profile_b.id, freeze, None, &[])
        .expect("the switch goes ahead once the source has stopped");
    assert!(
        state.supervisor.is_finished(&source),
        "{freeze}: the source stopped before its conversation was copied"
    );
}

#[test]
fn stop_now_stops_the_source_before_the_copy() {
    freezing("stop-now");
}

#[test]
fn finish_this_turn_stops_the_source_once_it_is_quiet() {
    freezing("finish-turn");
}

/// The live lab of 2026-10-28 (Claude Code 2.1.310): a headless Claude
/// turn that edits a file leaves file history in the account's
/// `file-history/<id>/`. The switch copied it, then, checking the source
/// was untouched, looked for it beside the transcript, found nothing and
/// failed every such Claude → Claude switch: "source changed during the
/// switch: ~file-history/…". It now checks each file where it was read.
#[test]
fn a_session_with_file_history_switches_and_its_history_travels() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project = work.path().join("lab");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let account_a = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Work", None).unwrap(),
    );
    let account_b = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap(),
    );
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();
    let native = "0b5e0e5c-6c0b-4c55-9a6b-2a7d3c9f1e12";
    let a_dir = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(&account_a.id);
    let transcript = a_dir
        .join("projects")
        .join(juggler_adapter::handover::mangled_dir(&cwd))
        .join(format!("{native}.jsonl"));
    seed(&transcript, native, &cwd);
    let history = a_dir.join("file-history").join(native);
    std::fs::create_dir_all(&history).unwrap();
    std::fs::write(
        history.join("6757c7b1fa2c80a2@v3"),
        "calc.py before the edit",
    )
    .unwrap();

    let session = SessionId::new("with-history");
    let machine = commands::local_machine_id(&state);
    {
        let storage = state.storage.lock().unwrap();
        let project_row = storage
            .projects()
            .touch(&juggler_core::storage::project_root(&project), None)
            .unwrap();
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: session.clone(),
                conversation: Some(ConversationId::new(native)),
                profile: juggler_core::model::ProfileId::new(profile_a.id.clone()),
                machine,
                kind: InstanceKind::Terminal,
                project: Some(project_row.id),
                cwd: project.clone(),
                status: SessionStatus::Finished,
                started_at: Timestamp::from_millis(Timestamp::now().millis() + 60_000),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                window: None,
                transcript_ref: None,
            })
            .unwrap();
    }

    commands::switch_execute_impl(
        &state,
        session.as_str(),
        &profile_b.id,
        "stop-now",
        None,
        &[],
    )
    .expect("a session with file history switches");
    let b_history = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(&account_b.id)
        .join("file-history")
        .join(native)
        .join("6757c7b1fa2c80a2@v3");
    assert_eq!(
        std::fs::read_to_string(b_history).unwrap(),
        "calc.py before the edit"
    );
    // Copied, never moved.
    assert!(history.join("6757c7b1fa2c80a2@v3").is_file());
}

/// The measured lab of 2026-10-28: A → B → Codex → back to A. B moved the
/// task list on (sub done); Codex, run as `codex exec`, keeps no list of
/// its own, so the update said nothing of it and A's own list was stale.
/// The update now gives the newest list a hop left, and whose it is. (Here
/// the last hop is a Claude account whose copy predates B's list: a list
/// the last hop doesn't have, as for Codex.)
#[test]
fn switching_back_gives_the_task_list_the_newest_hop_left() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project = work.path().join("lab");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let account = |label: &str| {
        let account = set_up::set_up(
            &state,
            commands::create_account(&state, CLAUDE_CODE_ID, label, None).unwrap(),
        );
        let profile = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
        (account.id, profile.id)
    };
    let (a, profile_a) = account("Alpha");
    let (b, profile_b) = account("Bravo");
    let (_c, profile_c) = account("Charlie");
    let native = "0b5e0e5c-6c0b-4c55-9a6b-2a7d3c9f1e33";
    let file_of = |account: &str| {
        state
            .accounts_dir()
            .join(CLAUDE_CODE_ID)
            .join(account)
            .join("projects")
            .join(juggler_adapter::handover::mangled_dir(&cwd))
            .join(format!("{native}.jsonl"))
    };
    seed(&file_of(&a), native, &cwd);
    let session = SessionId::new("chain-tasks");
    let machine = commands::local_machine_id(&state);
    {
        let storage = state.storage.lock().unwrap();
        let project_row = storage
            .projects()
            .touch(&juggler_core::storage::project_root(&project), None)
            .unwrap();
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: session.clone(),
                conversation: Some(ConversationId::new(native)),
                profile: juggler_core::model::ProfileId::new(profile_a.clone()),
                machine,
                kind: InstanceKind::Terminal,
                project: Some(project_row.id),
                cwd: project.clone(),
                status: SessionStatus::Finished,
                started_at: Timestamp::from_millis(Timestamp::now().millis() + 60_000),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                window: None,
                transcript_ref: None,
            })
            .unwrap();
    }
    let on_b =
        commands::switch_execute_impl(&state, session.as_str(), &profile_b, "stop-now", None, &[])
            .unwrap();
    let on_c =
        commands::switch_execute_impl(&state, &on_b.session_id, &profile_c, "stop-now", None, &[])
            .unwrap();
    {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(file_of(&b))
            .unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","uuid":"b-todo","cwd":"{cwd}","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"t1","name":"TodoWrite","input":{{"todos":[{{"content":"sub","status":"completed"}},{{"content":"mul","status":"pending"}}]}}}}]}}}}"#
        )
        .unwrap();
    }
    let plan = commands::switch_back_plan_impl(&state, &on_c.session_id).unwrap();
    let start = plan
        .returns
        .iter()
        .find(|r| r.start)
        .unwrap()
        .epoch_id
        .clone();
    let back = commands::switch_back_plan_to_impl(&state, &on_c.session_id, Some(&start)).unwrap();
    assert!(
        back.delta_preview
            .contains("Todos changed: [x] sub; [ ] mul (as Bravo left it)."),
        "{}",
        back.delta_preview
    );
}
