//! AJ-E1520 (0.2) app-level: the rail's subagent count (I-08, a row's
//! `subtask_count`) comes from the session's tool's subagent reader, the
//! one its Subagents tab reads: it follows a live session's transcript as
//! it grows, says nothing until read (never a guessed 0), and leaves the
//! row once the session ends (D-06: a subagent its records never saw end
//! is unknown then). Over a synthetic Claude Code setup, read-only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::subagents::count;
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    AccountId, ConversationId, InstanceKind, InstanceRecord, MachineId, SessionId, SessionStatus,
    Timestamp,
};

const CONVERSATION: &str = "conv-sub-1";

struct Lab {
    state: Arc<AppState>,
    transcript: PathBuf,
    session: SessionId,
    account: String,
    _dir: tempfile::TempDir,
}

fn append(path: &Path, text: &str) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

fn line(cwd: &Path, content: &str, kind: &str) -> String {
    format!(
        "{{\"type\":\"{kind}\",\"sessionId\":\"{CONVERSATION}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-27T10:00:00.000Z\",\"message\":{{\"role\":\"{kind}\",\"content\":{content}}}}}\n",
        cwd.display()
    )
}

fn agent_call(cwd: &Path, id: &str, description: &str) -> String {
    line(
        cwd,
        &format!(
            "[{{\"type\":\"tool_use\",\"id\":\"{id}\",\"name\":\"Agent\",\"input\":{{\"description\":\"{description}\",\"prompt\":\"p\",\"subagent_type\":\"Explore\"}}}}]"
        ),
        "assistant",
    )
}

fn agent_answer(cwd: &Path, id: &str) -> String {
    line(
        cwd,
        &format!("[{{\"type\":\"tool_result\",\"tool_use_id\":\"{id}\",\"content\":\"done\"}}]"),
        "user",
    )
}

/// An in-place Claude Code account with one conversation, and a live AI
/// Juggler session on it.
fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let cwd = root.join("Dev/shop");
    std::fs::create_dir_all(&cwd).unwrap();
    let config = root.join("claude");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    let folder = config
        .join("projects")
        .join(cwd.to_string_lossy().replace('/', "-"));
    std::fs::create_dir_all(&folder).unwrap();
    let transcript = folder.join(format!("{CONVERSATION}.jsonl"));
    append(&transcript, &line(&cwd, "\"Fix the parser\"", "user"));

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap();
    juggler_app::features::conversations::tick(&state).unwrap();

    let session = SessionId::new("sess-sub");
    let account = {
        let storage = state.storage.lock().unwrap();
        let account = storage.accounts().list().unwrap().remove(0).id;
        let profile = storage
            .profiles()
            .list_by_account(&AccountId::new(account.as_str()))
            .unwrap()
            .remove(0)
            .id;
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: session.clone(),
                conversation: Some(ConversationId::new(CONVERSATION)),
                profile,
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: cwd.clone(),
                status: SessionStatus::Running,
                started_at: Timestamp::now(),
                last_activity: Some(Timestamp::now()),
                usage: Default::default(),
                transcript_ref: None,
                attention: None,
                running: None,
                window: None,
            })
            .unwrap();
        account.to_string()
    };
    Lab {
        state,
        transcript,
        session,
        account,
        _dir: dir,
    }
}

/// The count on the session's row, as the UI receives it.
fn row_count(lab: &Lab) -> Option<u64> {
    let rows = {
        let storage = lab.state.storage.lock().unwrap();
        commands::session_infos_snapshot(&storage).unwrap()
    };
    count::counted(&lab.state, rows)
        .into_iter()
        .find(|row| row.id == lab.session.as_str())
        .unwrap()
        .subtask_count
}

#[test]
fn the_rows_count_follows_the_tabs_reader_and_leaves_when_the_session_ends() {
    let lab = lab();
    let cwd = lab.transcript.parent().unwrap().to_path_buf();
    assert_eq!(row_count(&lab), None, "not read yet: never a guessed 0");
    assert!(count::tick(&lab.state), "read: none running is a count");
    assert_eq!(row_count(&lab), Some(0));

    append(&lab.transcript, &agent_call(&cwd, "t1", "Map the parsers"));
    append(&lab.transcript, &agent_call(&cwd, "t2", "Review errors"));
    assert!(count::tick(&lab.state), "two started");
    assert_eq!(row_count(&lab), Some(2));

    append(&lab.transcript, &agent_answer(&cwd, "t1"));
    assert!(count::tick(&lab.state));
    assert_eq!(row_count(&lab), Some(1));
    assert!(
        !count::tick(&lab.state),
        "nothing appended: no change, no emit"
    );

    // The tab lists the same: one running, from the same reader.
    let tab = juggler_app::features::subagents::list_impl(
        &lab.state,
        &lab.account,
        CONVERSATION,
        &PageRequest::first(10),
    )
    .unwrap();
    assert_eq!(tab.state, "listed");
    assert_eq!(tab.page.items.len(), 2);

    // The session ends with t2 unanswered: the row stops counting.
    {
        let storage = lab.state.storage.lock().unwrap();
        let mut record = storage.sessions().get(&lab.session).unwrap();
        record.status = SessionStatus::Finished;
        storage.sessions().update(&record).unwrap();
    }
    assert!(count::tick(&lab.state), "a running subagent left the count");
    assert_eq!(row_count(&lab), None, "ended: unknown, not 0");
}

#[test]
fn a_session_whose_conversation_isnt_indexed_is_not_counted() {
    let lab = lab();
    {
        let storage = lab.state.storage.lock().unwrap();
        let mut record = storage.sessions().get(&lab.session).unwrap();
        record.conversation = Some(ConversationId::new("not-on-this-mac"));
        storage.sessions().update(&record).unwrap();
    }
    assert!(!count::tick(&lab.state));
    assert_eq!(row_count(&lab), None);
}
