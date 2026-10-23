//! QA of the switch screen (AJ-E1543 Req.4) on a real setup, read-only.
//! Both tests are ignored, and do nothing unless their variables are set.
//!
//! The screen opens from one of AI Juggler's own sessions. On a real setup
//! walked read-only there are none (every session there was started
//! outside it), so:
//!
//! - `switch_session_add` puts one FINISHED session record in AI Juggler
//!   Dev's own data (`AIJ_QA_REAL_DATA`) for a real conversation of the
//!   account named `AIJ_QA_SWITCH_ACCOUNT`: its newest top-level transcript
//!   last written more than two hours ago, in a project that's still there.
//!   No process, nothing launched, nothing written outside the Dev data
//!   folder. The record's id goes to `AIJ_QA_SWITCH_OUT`.
//! - `switch_session_remove` deletes that record, and every row that names
//!   it, from the same data folder.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use juggler_app::state::AppState;
use juggler_core::model::{
    ConversationId, InstanceKind, InstanceRecord, MachineId, SessionId, SessionStatus, Timestamp,
    TranscriptRef,
};

/// The `cwd` and the number of user or assistant lines of a transcript.
fn shape(path: &Path) -> (Option<String>, usize) {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut cwd = None;
    let mut messages = 0;
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if cwd.is_none() {
            cwd = value.get("cwd").and_then(|c| c.as_str()).map(str::to_owned);
        }
        if matches!(
            value.get("type").and_then(|t| t.as_str()),
            Some("user" | "assistant")
        ) {
            messages += 1;
        }
    }
    (cwd, messages)
}

#[test]
#[ignore = "QA on a real setup; needs AIJ_QA_REAL_DATA, AIJ_QA_SWITCH_ACCOUNT, AIJ_QA_SWITCH_OUT"]
fn switch_session_add() {
    let (Some(data), Ok(label), Some(out)) = (
        std::env::var_os("AIJ_QA_REAL_DATA").map(PathBuf::from),
        std::env::var("AIJ_QA_SWITCH_ACCOUNT"),
        std::env::var_os("AIJ_QA_SWITCH_OUT").map(PathBuf::from),
    ) else {
        eprintln!("AIJ_QA_REAL_DATA / AIJ_QA_SWITCH_ACCOUNT / AIJ_QA_SWITCH_OUT not set: nothing to do");
        return;
    };
    let state = AppState::init(data).unwrap();
    let storage = state.storage.lock().unwrap();
    let account = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .find(|account| account.label == label)
        .expect("that account was brought in");
    let profile = storage
        .profiles()
        .list_by_account(&account.id)
        .unwrap()
        .into_iter()
        .next()
        .expect("the account has a launch setting");
    // Its newest finished conversation: top level, quiet for two hours,
    // with messages, in a folder that's still there and a known project.
    let quiet = SystemTime::now() - Duration::from_secs(2 * 3600);
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    for dir in std::fs::read_dir(account.state_dir.join("projects")).unwrap().flatten() {
        for file in std::fs::read_dir(dir.path()).into_iter().flatten().flatten() {
            let path = file.path();
            let modified = file.metadata().and_then(|m| m.modified()).ok();
            if path.extension().is_some_and(|e| e == "jsonl")
                && let Some(modified) = modified.filter(|m| *m < quiet)
            {
                found.push((modified, path));
            }
        }
    }
    found.sort();
    let (path, cwd, project) = found
        .into_iter()
        .rev()
        .find_map(|(_, path)| {
            let (cwd, messages) = shape(&path);
            let cwd = PathBuf::from(cwd?);
            let root = juggler_core::storage::project_root(&cwd);
            let project = storage.projects().by_root(&root).ok().flatten()?;
            (messages > 0 && cwd.is_dir()).then_some((path, cwd, project))
        })
        .expect("a finished conversation in a project");
    let native = path.file_stem().unwrap().to_string_lossy().into_owned();
    let id = SessionId::new(format!("qa-switch-{}", &native[..8]));
    storage
        .sessions()
        .insert(&InstanceRecord {
            id: id.clone(),
            conversation: Some(ConversationId::new(native.clone())),
            profile: profile.id.clone(),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd,
            status: SessionStatus::Finished,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            transcript_ref: Some(TranscriptRef {
                native_id: Some(native),
                path,
            }),
            attention: None,
            running: None,
            window: None,
        })
        .unwrap();
    std::fs::write(&out, id.as_str()).unwrap();
    eprintln!("added {}", id.as_str());
}

#[test]
#[ignore = "QA on a real setup; needs AIJ_QA_REAL_DATA and AIJ_QA_SWITCH_OUT"]
fn switch_session_remove() {
    let (Some(data), Some(out)) = (
        std::env::var_os("AIJ_QA_REAL_DATA").map(PathBuf::from),
        std::env::var_os("AIJ_QA_SWITCH_OUT").map(PathBuf::from),
    ) else {
        eprintln!("AIJ_QA_REAL_DATA / AIJ_QA_SWITCH_OUT not set: nothing to do");
        return;
    };
    let id = std::fs::read_to_string(&out).unwrap();
    assert!(id.starts_with("qa-switch-"), "only a record this test added");
    let state = AppState::init(data).unwrap();
    let storage = state.storage.lock().unwrap();
    let conn = storage.raw_conn();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut removed = 0;
    for table in tables {
        let columns: Vec<String> = conn
            .prepare(&format!("PRAGMA table_info(\"{table}\")"))
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for column in columns
            .iter()
            .filter(|c| matches!(c.as_str(), "session_id" | "session" | "instance_id"))
        {
            removed += conn
                .execute(&format!("DELETE FROM \"{table}\" WHERE \"{column}\" = ?1"), [&id])
                .unwrap();
        }
        if table == "instances" {
            removed += conn
                .execute("DELETE FROM instances WHERE id = ?1", [&id])
                .unwrap();
        }
    }
    assert!(
        storage.sessions().get(&SessionId::new(id.as_str())).is_err(),
        "the record is gone"
    );
    eprintln!("removed {id}: {removed} rows");
}
