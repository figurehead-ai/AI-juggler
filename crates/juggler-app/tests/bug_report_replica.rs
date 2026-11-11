//! AJ-E2911 and AJ-E2912 evidence: with debug mode on, a launch and a
//! switch through the app's own commands, then a switch that fails (the
//! target's folder refuses the write), and Report a bug from that failure.
//!
//! It holds that the debug log has the launch's and both switches' steps
//! and never the conversation's text or folder; that the report's parts
//! come in their order, scrubbed, with the failed switch's steps; and that
//! the bug form's link fits. With `AIJ_EVIDENCE=1` it writes what it saw
//! to `docs/qa/evidence/AJ-E2911/` and `AJ-E2912/`.
//!
//! Its own test binary: debug mode is process-wide (the layer, the flag).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "support/set_up.rs"]
mod set_up;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapter::handover::mangled_dir;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::features::app::diagnostics::{debug_log, report};
use juggler_app::state::AppState;

const SESSION: &str = "7c1d2e3f-0000-4000-8000-00000000beef";
const MARK: &str = "MARKED-WORDS-NEVER-LOGGED";

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = dir.join("claude-stub");
    stand_in::write(&stub, "#!/bin/sh\nexit 0\n");
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn config_dir_of(state: &Arc<AppState>, account_id: &str) -> PathBuf {
    state.accounts_dir().join(CLAUDE_CODE_ID).join(account_id)
}

/// Four turns, every one carrying the mark.
fn seed(config: &Path, cwd: &str) -> PathBuf {
    let dir = config.join("projects").join(mangled_dir(cwd));
    std::fs::create_dir_all(&dir).unwrap();
    let mut text = String::new();
    for turn in 0..4 {
        let at = format!("2026-11-0{}T10:00:00Z", turn + 1);
        let ask = serde_json::json!({
            "uuid": format!("q{turn}"), "parentUuid": if turn == 0 { None } else { Some(format!("a{}", turn - 1)) },
            "sessionId": SESSION, "cwd": cwd, "timestamp": at, "type": "user",
            "message": {"role": "user", "content": format!("{MARK} question {turn}")},
        });
        let answer = serde_json::json!({
            "uuid": format!("a{turn}"), "parentUuid": format!("q{turn}"), "sessionId": SESSION,
            "cwd": cwd, "timestamp": at, "type": "assistant",
            "message": {"role": "assistant", "content": [{"type": "text", "text": format!("{MARK} answer {turn}")}]},
        });
        text.push_str(&format!("{ask}\n{answer}\n"));
    }
    let main = dir.join(format!("{SESSION}.jsonl"));
    std::fs::write(&main, text).unwrap();
    main
}

fn attach(state: &Arc<AppState>, session_id: &str, transcript: &Path) {
    let storage = state.storage.lock().unwrap();
    let id = juggler_core::model::SessionId::new(session_id.to_owned());
    let mut record = storage.sessions().get(&id).unwrap();
    record.transcript_ref = Some(juggler_core::model::TranscriptRef {
        path: transcript.to_path_buf(),
        native_id: Some(SESSION.to_owned()),
    });
    storage.sessions().update(&record).unwrap();
}

#[test]
fn a_failed_switch_reports_what_it_did_and_never_the_conversation() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    {
        let storage = state.storage.lock().unwrap();
        debug_log::install(&state.data_dir, &storage);
        debug_log::set(&storage, true).unwrap();
    }
    let project = work.path().join(format!("shop-{MARK}"));
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let account = |label: &str| {
        set_up::set_up(
            &state,
            juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, label, None)
                .unwrap(),
        )
    };
    let (a, b, c) = (account("Default"), account("Personal"), account("Side"));
    let profile = |id: &str| {
        juggler_app::features::profile::create_profile(&state, id, "Main", None, None).unwrap()
    };
    let (pa, pb, pc) = (profile(&a.id), profile(&b.id), profile(&c.id));

    // A launch, and a switch that lands.
    let launched =
        juggler_app::features::launch::launch_profile(&state, &pa.id, Some(cwd.clone())).unwrap();
    attach(
        &state,
        &launched.session_id,
        &seed(&config_dir_of(&state, &a.id), &cwd),
    );
    let switched = juggler_app::features::switch::execute::switch_execute_impl(
        &state,
        &launched.session_id,
        &pb.id,
        "stop-now",
        None,
        &[],
    )
    .unwrap_or_else(|error| panic!("the first switch failed: {:?}", error.wire()));

    // A switch whose target folder refuses the write.
    let refusing = config_dir_of(&state, &c.id).join("projects");
    std::fs::create_dir_all(&refusing).unwrap();
    std::fs::set_permissions(&refusing, std::fs::Permissions::from_mode(0o500)).unwrap();
    let failed = juggler_app::features::switch::execute::switch_execute_impl(
        &state,
        &switched.session_id,
        &pc.id,
        "stop-now",
        None,
        &[],
    );
    std::fs::set_permissions(&refusing, std::fs::Permissions::from_mode(0o700)).unwrap();
    let wire = failed.expect_err("the target refused the write").wire();

    // The debug log: the launch's and both switches' steps, no conversation.
    // A launch's exit is told by its wait thread: give it a moment.
    let mut log = debug_log::text(&state.data_dir, 1024 * 1024);
    for _ in 0..50 {
        if log.contains(" launch exited ") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        log = debug_log::text(&state.data_dir, 1024 * 1024);
    }
    eprintln!("the failed switch: {wire:?}\n{log}");
    assert!(log.lines().any(|l| l.contains(" launch spawned ")), "{log}");
    // R2: a launch logs its exit code (or the signal that ended it).
    assert!(
        log.lines()
            .any(|l| l.contains(" launch exited ")
                && (l.contains(" code=") || l.contains(" signal="))),
        "{log}"
    );
    // R2: a switch logs the conversation's turns (four were seeded).
    assert!(
        log.lines()
            .any(|l| l.contains(" switch bundled ") && l.contains(" turns=4")),
        "{log}"
    );
    assert!(
        log.lines().any(|l| l.contains(" switch launched ")),
        "{log}"
    );
    assert!(
        !log.contains(MARK),
        "the log carries the conversation:\n{log}"
    );

    // Report a bug from the failed switch's step.
    let context = report::BugContextArgs {
        from: "switch".to_owned(),
        words: Some(wire.user_message.clone()),
        detail: wire.detail.clone(),
        code: Some(wire.code.clone()),
        at_ms: Some(juggler_core::model::Timestamp::now().millis()),
    };
    let shown = report::preview(&state, &context).unwrap();
    let keys: Vec<&str> = shown.parts.iter().map(|p| p.key.as_str()).collect();
    assert_eq!(&keys[..3], ["failure", "app", "switch"], "{keys:?}");
    assert!(keys.contains(&"debug"), "{keys:?}");
    for part in &shown.parts {
        assert!(
            !part.text.contains(MARK),
            "part {} carries the conversation",
            part.key
        );
        assert!(
            !part.text.contains(&*work.path().to_string_lossy()),
            "part {} carries a path outside home",
            part.key
        );
    }
    let all: Vec<String> = shown.parts.iter().map(|p| p.key.clone()).collect();
    let composed = report::compose(
        &shown.id,
        &shown.title,
        "I switched to Side and it stopped.",
        &all,
    )
    .unwrap();
    let url = report::issue_url(&composed.title, &composed.fields);
    assert!(url.len() <= 8_000, "the link is {} long", url.len());
    // Left out, a part is gone from what goes.
    let without = report::compose(
        &shown.id,
        &shown.title,
        "",
        &["failure".to_owned(), "app".to_owned()],
    )
    .unwrap();
    assert!(!without.body.contains("The last switch"));

    if std::env::var_os("AIJ_EVIDENCE").is_some() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/qa/evidence");
        for dir in ["AJ-E2911", "AJ-E2912"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(root.join("AJ-E2911/report.txt"), &composed.body).unwrap();
        let fields: Vec<String> = composed
            .fields
            .iter()
            .map(|f| format!("{} ({}): {} characters", f.id, f.label, f.value.len()))
            .collect();
        std::fs::write(
            root.join("AJ-E2911/link.txt"),
            format!(
                "The bug form's link: {} characters; logs cut to fit: {}.\nIts fields:\n{}\n",
                url.len(),
                composed.cut,
                fields.join("\n")
            ),
        )
        .unwrap();
        std::fs::write(root.join("AJ-E2912/debug-log.txt"), &log).unwrap();
    }
}
