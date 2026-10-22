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

    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work", None).unwrap();
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
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
        .join(cwd.replace('/', "-"))
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
