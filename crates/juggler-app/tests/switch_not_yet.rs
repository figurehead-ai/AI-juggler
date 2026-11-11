//! The rc33 M4 walk: a session with no conversation yet had live doors
//! ("Continue this session in another app…" on its out-of-room notice,
//! "Continue on another account…" in its header) while its switch list
//! greyed every app and the dialog refused. The switch's targets now say
//! why the session itself can't switch yet (`not_yet`, the same on every
//! row and its `filtered_reason`), which the page's other doors grey with;
//! once it has a turn, none.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::sync::Arc;

use juggler_app::state::{AppState, default_registry};
use juggler_core::model::{
    ConversationId, InstanceKind, InstanceRecord, ProfileId, SessionId, SessionStatus, Timestamp,
    TranscriptRef,
};

#[test]
fn a_session_with_nothing_to_carry_says_why_on_every_target() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("data"), default_registry()).unwrap(),
    );
    let made = |label: &str| {
        let account =
            juggler_app::features::account::create_account(&state, "claude-code", label, None)
                .unwrap();
        let profile =
            juggler_app::features::profile::create_profile(&state, &account.id, "Main", None, None)
                .unwrap();
        (account, profile)
    };
    let (work, work_profile) = made("Work · work");
    made("Personal · personal");
    made("Spare · spare");
    let cwd = dir.path().join("web");
    std::fs::create_dir_all(&cwd).unwrap();
    let mut writer = InstanceRecord {
        id: SessionId::new("writer"),
        conversation: None,
        profile: ProfileId::new(work_profile.id.as_str()),
        machine: juggler_app::commands::local_machine_id(&state),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: cwd.clone(),
        status: SessionStatus::Finished,
        started_at: Timestamp::now(),
        last_activity: Some(Timestamp::now()),
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    };
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&writer)
        .unwrap();

    let targets =
        juggler_app::features::switch::targets::switch_targets_impl(&state, "writer").unwrap();
    assert_eq!(targets.len(), 2, "{targets:?}");
    for target in &targets {
        let why = target.not_yet.as_deref().expect("why it can't switch yet");
        assert!(
            why.starts_with("This session hasn't started a conversation yet"),
            "{why}"
        );
        assert_eq!(
            target.filtered_reason, target.not_yet,
            "the list says the same"
        );
    }

    // It talks: a turn in Claude Code's own file, and nothing holds it back
    // for that.
    let native = "0b5e7a52-1c2d-4e3f-8a9b-0c1d2e3f4a5b";
    let file = state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .get(&juggler_core::model::AccountId::new(work.id.clone()))
        .unwrap()
        .state_dir
        .join("projects")
        .join(juggler_adapter::handover::mangled_dir(
            &cwd.to_string_lossy(),
        ))
        .join(format!("{native}.jsonl"));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let line = |kind: &str, parent: Option<&str>, uuid: &str, message: serde_json::Value| {
        serde_json::json!({
            "parentUuid": parent, "isSidechain": false, "userType": "external",
            "cwd": cwd, "sessionId": native, "version": "2.1.296", "gitBranch": "main",
            "uuid": uuid, "timestamp": "2026-11-11T09:00:00.000Z", "type": kind, "message": message,
        })
        .to_string()
    };
    std::fs::write(
        &file,
        [
            line("user", None, "u-1", serde_json::json!({"role": "user", "content": "Fix the flaky login test"})),
            line(
                "assistant",
                Some("u-1"),
                "a-1",
                serde_json::json!({"id": "msg_1", "type": "message", "role": "assistant", "model": "claude-opus-4-6",
                    "content": [{"type": "text", "text": "Done."}], "stop_reason": "end_turn",
                    "usage": {"input_tokens": 10, "output_tokens": 5}}),
            ),
        ]
        .join("\n")
            + "\n",
    )
    .unwrap();
    writer.conversation = Some(ConversationId::new(native));
    writer.transcript_ref = Some(TranscriptRef {
        path: file,
        native_id: Some(native.to_owned()),
    });
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .update(&writer)
        .unwrap();
    let targets =
        juggler_app::features::switch::targets::switch_targets_impl(&state, "writer").unwrap();
    assert_eq!(targets.len(), 2);
    assert!(
        targets.iter().all(|target| target.not_yet.is_none()),
        "{targets:?}"
    );
}
