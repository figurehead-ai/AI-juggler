//! D-76 (AJ-E211): a switch that leaves out the start of a conversation,
//! through the app's own switch command: what lands starts at a turn with
//! its chain whole, the turns before it are gone, the source keeps every
//! byte, and the screen's preview said so first.

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

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapter::handover::{Cut, mangled_dir, sha256_file};
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::switch_cut::{SwitchCutArgs, preview_impl};
use juggler_app::state::AppState;

const SESSION: &str = "7c1d2e3f-0000-4000-8000-00000000cafe";

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

/// Ten turns, a day apart: the person asks, Claude answers.
fn seed(config: &Path, cwd: &str) -> PathBuf {
    let dir = config.join("projects").join(mangled_dir(cwd));
    std::fs::create_dir_all(&dir).unwrap();
    let mut lines = Vec::new();
    for day in 0..10 {
        let at = format!("2026-10-{:02}T10:00:00Z", day + 1);
        let ask = format!("q{day}");
        let answer = format!("a{day}");
        lines.push(serde_json::json!({
            "uuid": ask, "parentUuid": if day == 0 { None } else { Some(format!("a{}", day - 1)) },
            "sessionId": SESSION, "cwd": cwd, "timestamp": at, "type": "user",
            "message": {"role": "user", "content": format!("question number {day}")},
        }));
        lines.push(serde_json::json!({
            "uuid": answer, "parentUuid": ask, "sessionId": SESSION, "cwd": cwd,
            "timestamp": at, "type": "assistant",
            "message": {"role": "assistant", "content": [{"type": "text", "text": format!("answer {day}")}]},
        }));
    }
    let main = dir.join(format!("{SESSION}.jsonl"));
    let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
    std::fs::write(&main, text).unwrap();
    main
}

#[test]
fn the_last_three_turns_switch_and_the_rest_stays_behind() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project = work.path().join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Default", None).unwrap(),
    );
    let b = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap(),
    );
    let profile_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();
    let main = seed(&config_dir_of(&state, &a.id), &cwd);
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.clone(),
            ))
            .unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: main.clone(),
            native_id: Some(SESSION.to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }
    let source_before = sha256_file(&main, None).unwrap();

    // The screen says what it would leave out, first.
    let args = SwitchCutArgs {
        before_ms: None,
        keep_last: Some(3),
    };
    let shown = preview_impl(&state, &launched.session_id, &profile_b.id, args).unwrap();
    assert!(shown.supported);
    assert_eq!((shown.turns, shown.turns_left_out), (10, 7));
    assert!(
        shown.words.starts_with("Carries the last 3 turns"),
        "{}",
        shown.words
    );
    assert!(shown.loses.is_some());

    let switched = commands::switch_execute_cut(
        &state,
        &launched.session_id,
        &profile_b.id,
        "stop-now",
        Cut::KeepLast(3),
    )
    .unwrap_or_else(|error| panic!("the switch failed: {:?}", error.wire()));

    let landed = config_dir_of(&state, &b.id)
        .join("projects")
        .join(mangled_dir(&cwd))
        .join(format!("{SESSION}.jsonl"));
    let lines: Vec<serde_json::Value> = std::fs::read_to_string(&landed)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 6, "three turns, each asked and answered");
    assert_eq!(lines[0]["uuid"], "q7");
    assert!(lines[0]["parentUuid"].is_null(), "the chain starts here");
    let uuids: Vec<&str> = lines.iter().filter_map(|l| l["uuid"].as_str()).collect();
    for line in &lines[1..] {
        assert!(
            uuids.contains(&line["parentUuid"].as_str().unwrap()),
            "the chain is whole"
        );
    }
    assert!(
        !std::fs::read_to_string(&landed)
            .unwrap()
            .contains("question number 6")
    );
    assert_eq!(
        sha256_file(&main, None).unwrap(),
        source_before,
        "the source keeps every byte"
    );
    // The switch's record says what it reached and what it carried.
    let storage = state.storage.lock().unwrap();
    let graph = storage.switch_graph();
    let edge_id = juggler_core::model::SwitchEdgeId::new(switched.edge_id.clone());
    let edge = graph.edge(&edge_id).unwrap();
    assert_eq!(
        edge.fidelity_shown.as_deref(),
        Some("F2"),
        "lowered, as a compaction lowers it"
    );
    let manifest = graph.manifest_for(&edge_id).unwrap();
    assert!(
        manifest.iter().any(|row| row.disposition == "cut"),
        "{manifest:?}"
    );
}
