//! AJ-E206 integration: the hybrid door (pass2/02 §8) — preflight
//! chooses 1-click only for proven pairs, a lossy pair is NEVER
//! reachable in 1-click (the P0 threshold), and undo compensates a
//! completed switch (stop target + resume source + parent-linked
//! edge), all proven with the stub harness.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::SessionStatus;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// A claude stub that records its argv under the isolated config dir and
/// exits — enough to prove the switch (and undo) launched with
/// `--resume <id>`.
fn claude_stub(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("claude-stub");
    stand_in::write(
        &path,
        // Written whole, then moved into place: a reader never sees it
        // half-written (the same fix as parked_reset's stub).
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/last-args.tmp\" && mv \"$CLAUDE_CONFIG_DIR/last-args.tmp\" \"$CLAUDE_CONFIG_DIR/last-args\"\n",
    );
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = claude_stub(dir);
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    // AJ-E204: the same adapter behind the handover sub-trait —
    // without it the pair is replay/pack only.
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn seed_transcript(config_dir: &Path, cwd: &str, session_id: &str) {
    let encoded = cwd.replace('/', "-");
    let dir = config_dir.join("projects").join(encoded);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{session_id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"message\":{{}}}}\n{{\"type\":\"assistant\",\"cwd\":\"{cwd}\"}}\n"
        ),
    )
    .unwrap();
}

fn config_dir_of(state: &Arc<AppState>, account_id: &str) -> std::path::PathBuf {
    state.accounts_dir().join(CLAUDE_CODE_ID).join(account_id)
}

/// Launches one session on `profile` inside `cwd` and pins its
/// transcript (the watcher's job in production — AJ-E204).
fn launch_with_transcript(
    state: &Arc<AppState>,
    profile_id: &str,
    account_id: &str,
    cwd: &str,
    native_id: &str,
) -> String {
    let launched = commands::launch_profile(state, profile_id, Some(cwd.to_owned())).unwrap();
    // The setup launch's stub records its (empty) argv asynchronously —
    // wait for THAT write to land so it can never race a later resume's
    // argv record (a late truncate clobbers its first byte).
    {
        let setup_args = config_dir_of(state, account_id).join("last-args");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !setup_args.is_file() {
            assert!(std::time::Instant::now() < deadline, "setup stub never ran");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    seed_transcript(&config_dir_of(state, account_id), cwd, native_id);
    let transcript = config_dir_of(state, account_id)
        .join("projects")
        .join(cwd.replace('/', "-"))
        .join(format!("{native_id}.jsonl"));
    let storage = state.storage.lock().unwrap();
    let mut record = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(
            launched.session_id.clone(),
        ))
        .unwrap();
    record.transcript_ref = Some(juggler_core::model::TranscriptRef {
        path: transcript,
        native_id: Some(native_id.to_owned()),
    });
    storage.sessions().update(&record).unwrap();
    drop(storage);
    launched.session_id
}

fn door_counters(state: &Arc<AppState>) -> std::collections::BTreeMap<String, u64> {
    let storage = state.storage.lock().unwrap();
    storage
        .app_settings()
        .get("switch_door_counters")
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default()
}

#[test]
fn preflight_opens_the_wizard_until_the_pair_is_proven() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project_dir = work.path().join("storefront");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();

    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();
    let source_id =
        launch_with_transcript(&state, &profile_a.id, &account_a.id, &cwd, "native-sess-7");

    let info = commands::switch_preflight_impl(&state, &source_id, &profile_b.id).unwrap();

    // §8: an unproven pair opens the wizard — exactly the
    // non-trivial steps, never phantom ones.
    assert_eq!(info.door, "wizard");
    assert_eq!(
        info.wizard_steps,
        vec![
            "Target".to_owned(),
            "What's carried".to_owned(),
            "Confirm".to_owned()
        ]
    );
    assert!(
        !info.usual_account,
        "no history for B on this project ⇒ not the usual account"
    );
    // QA P23: the screen says what a transplant does, as the list does;
    // "not proven" is its own reason, never an F-number.
    assert_eq!(info.mechanism, "transplant");
    assert_eq!(info.fidelity_words, "Same conversation, new account");
    assert!(
        info.wizard_reasons
            .iter()
            .all(|reason| !reason.contains("Fidelity") && !reason.contains("F1")),
        "{:?}",
        info.wizard_reasons
    );
    if !info.validated {
        assert_eq!(info.fidelity, "F1", "the door keeps the honesty cap");
        assert!(info.experimental_copy.starts_with("Beta, not proven yet:"));
        assert!(
            info.wizard_reasons
                .iter()
                .any(|reason| reason == "This move hasn't been proven on a real setup yet."),
            "{:?}",
            info.wizard_reasons
        );
    }
    // AJ-E1541: the task list is rebuilt from the conversation (proven by
    // the lab and the handover tests), and the carry-list says so, never
    // "some context" and never a byte copy.
    assert!(info.carry.regenerated.contains(&"The task list".to_owned()));
    assert!(!info.carry.dropped.contains(&"The task list".to_owned()));
    // D-03 amendment (2026-10-27): another account is the point of a
    // switch — never a reason, never a warning.
    assert!(
        !info
            .wizard_reasons
            .iter()
            .any(|reason| reason.contains("usual account")),
        "another account is not a risk"
    );
    // A same-tool switch on this Mac copies the session as it is: the
    // keys question doesn't apply, and the Target step says what moves.
    assert!(!info.secrets.asks, "a transplant never asks about keys");
    assert!(info.move_line.starts_with("The whole conversation moves"));
    // The preflight only reads: nothing is counted until the switch
    // screen counts the door it shows (§5 telemetry, KV only).
    assert!(
        door_counters(&state).is_empty(),
        "the preflight wrote a counter"
    );
    commands::switch_door_count_impl(&state, &info.door, &info.wizard_steps).unwrap();
    assert_eq!(
        door_counters(&state)
            .get("switch_wizard[Target,What's carried,Confirm]")
            .copied(),
        Some(1),
        "local door counter recorded"
    );
    // Anything that isn't a door the preflight can choose is refused,
    // and counts nothing.
    for (door, steps) in [
        ("wizard", vec!["Target".to_owned(), "Anything".to_owned()]),
        ("one-click", vec!["Target".to_owned()]),
        ("teleport", vec![]),
    ] {
        assert!(
            commands::switch_door_count_impl(&state, door, &steps).is_err(),
            "{door} {steps:?}"
        );
    }
    assert_eq!(door_counters(&state).values().sum::<u64>(), 1);
}

#[test]
fn lossy_pair_is_never_reachable_in_one_click() {
    // The P0 threshold: even when EVERY other §8 condition holds
    // (usual account, transplant, headroom), a listed Drop keeps the
    // door at the wizard until E210's fixtures flip the cell.
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project_dir = work.path().join("storefront");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();

    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();
    let source_id =
        launch_with_transcript(&state, &profile_a.id, &account_a.id, &cwd, "native-sess-8");
    // Make B the project's dominant account: two live sessions to
    // A's one — the usual-account test passes on its own.
    for id in ["b-1", "b-2"] {
        launch_with_transcript(&state, &profile_b.id, &account_b.id, &cwd, id);
    }

    let info = commands::switch_preflight_impl(&state, &source_id, &profile_b.id).unwrap();
    assert!(info.usual_account, "B dominates the project history");
    assert_eq!(
        info.door, "wizard",
        "a listed Drop (todos) must never ride 1-click — P0 threshold"
    );
}

#[test]
fn undo_stops_target_resumes_source_and_compensates_the_edge() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project_dir = work.path().join("storefront");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();

    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();
    let source_id =
        launch_with_transcript(&state, &profile_a.id, &account_a.id, &cwd, "native-sess-9");

    // Guardrail: undo of an unknown edge refuses (nothing to undo).
    let refused = commands::switch_undo_impl(&state, "edge-nope");
    assert!(refused.is_err(), "unknown edge refused");

    // The switch (user click path): B's continuation instance.
    let switched =
        commands::switch_execute_impl(&state, &source_id, &profile_b.id, "stop-now", None, &[])
            .unwrap();
    assert_eq!(switched.source_session_id, source_id);
    assert!(!switched.edge_id.is_empty(), "edge id is the undo handle");

    // Undo: stop B's instance, resume A's native session, compensate.
    let undone = commands::switch_undo_impl(&state, &switched.edge_id).unwrap();
    assert_ne!(undone.session_id, switched.session_id);
    assert_ne!(undone.session_id, source_id);

    // The graph keeps BOTH edges (append-only): the switch and its
    // parent-linked compensation — T1 native resume, F4, confirmed.
    let (edges, epochs) = {
        let storage = state.storage.lock().unwrap();
        let graph = storage.switch_graph();
        let works = graph
            .works_for_project(
                &storage
                    .projects()
                    .by_root(&std::fs::canonicalize(&cwd).unwrap())
                    .unwrap()
                    .expect("project recorded")
                    .id,
            )
            .unwrap();
        assert_eq!(works.len(), 1);
        (
            graph.edges_for_work(&works[0].id).unwrap(),
            graph.epochs_for_work(&works[0].id).unwrap(),
        )
    };
    assert_eq!(edges.len(), 2, "switch + compensating edge");
    let (original, undo_edge) = (&edges[0], &edges[1]);
    use juggler_core::model::{EdgeState, SwitchMechanism};
    assert!(matches!(original.state, EdgeState::Confirmed));
    assert!(matches!(undo_edge.state, EdgeState::Confirmed));
    assert_eq!(
        undo_edge.parent_edge_a.as_ref().map(|id| id.as_str()),
        Some(original.id.as_str())
    );
    assert!(matches!(undo_edge.mechanism, SwitchMechanism::NativeResume));
    assert_eq!(undo_edge.fidelity_shown.as_deref(), Some("F4"));
    assert_eq!(undo_edge.from_epoch, original.to_epoch.clone().unwrap());
    // Three epochs: source, target, resumed source.
    assert_eq!(epochs.len(), 3);

    // A's account resumed its own native session (the stub's argv is
    // the proof the user sees).
    let resumed = {
        let storage = state.storage.lock().unwrap();
        let record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                undone.session_id.clone(),
            ))
            .unwrap();
        (record.profile, record.status)
    };
    assert_eq!(
        resumed,
        (
            juggler_core::model::LaunchRecipeId::new(profile_a.id.clone()),
            SessionStatus::Starting
        )
    );
    let args_path = config_dir_of(&state, &account_a.id).join("last-args");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let args = loop {
        if let Ok(args) = std::fs::read_to_string(&args_path) {
            // The shell truncates before printf writes — wait for
            // content, not just existence.
            if !args.trim().is_empty() {
                break args;
            }
        }
        if std::time::Instant::now() > deadline {
            panic!("stub never recorded its resume args");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    assert!(
        args.contains("--resume") && args.contains("native-sess-9"),
        "source resumed natively: {args}"
    );
}

/// QA P23: a Claude → Codex move replays, and the screen says so: "Full
/// story, fresh start", and the Beta line is the replay's, never the
/// transplant's "the whole session moves".
#[test]
fn a_cross_tool_move_says_it_starts_fresh() {
    let work = tempfile::tempdir().unwrap();
    let stub = claude_stub(work.path());
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    registry.register(Arc::new(juggler_adapters::CodexAdapter::with_bin(
        "/bin/true",
    )));
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());
    let project_dir = work.path().join("storefront");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();
    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let account_b =
        commands::create_account(&state, juggler_adapters::CODEX_ID, "Codex", None).unwrap();
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();
    let source_id =
        launch_with_transcript(&state, &profile_a.id, &account_a.id, &cwd, "native-sess-8");

    let info = commands::switch_preflight_impl(&state, &source_id, &profile_b.id).unwrap();
    assert_eq!(info.mechanism, "replay");
    assert_eq!(info.fidelity_words, "Full story, fresh start");
    assert!(
        info.experimental_copy.contains("a fresh start on Codex"),
        "{}",
        info.experimental_copy
    );
    assert!(!info.experimental_copy.contains("the whole session moves"));
}
