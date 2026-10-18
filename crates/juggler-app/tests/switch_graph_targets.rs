//! AJ-E209 integration: the badged ranked switch targets (§Q6 with
//! the switch extensions; badges never "lossless", cross-Provider
//! always Beta + Drop list), the Switch Graph A→B→A timeline
//! (switch-back ↺, resumable nodes, the S-F9 hop note, D-13-ready
//! ids), the graph's Export pack action through the AJ-E208 GLOBAL
//! redaction policy (unset blocks; Strip leaves zero canary bytes),
//! and ranking determinism (same inputs → same order + same
//! explanation).

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
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
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter, FAKE_HARNESS_ID, FakeHarnessAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

fn claude_stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/last-args\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Claude + the fake harness (a registered second Provider, so a
/// cross-Provider target can rank — no real vendor anything).
fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = claude_stub(dir);
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    registry.register(Arc::new(FakeHarnessAdapter::new(dir.join("fake-harness"))));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn config_dir_of(state: &Arc<AppState>, account_id: &str) -> PathBuf {
    state.accounts_dir().join(CLAUDE_CODE_ID).join(account_id)
}

/// A non-empty `.credentials.json` marks the account signed-in — a
/// test marker file under the temp state dir, never a real
/// credential.
fn sign_in(state: &Arc<AppState>, account_id: &str) {
    std::fs::write(
        config_dir_of(state, account_id).join(".credentials.json"),
        r#"{"stub":true}"#,
    )
    .unwrap();
}

fn transcript_of(state: &Arc<AppState>, account_id: &str, cwd: &str, native: &str) -> PathBuf {
    config_dir_of(state, account_id)
        .join("projects")
        .join(cwd.replace('/', "-"))
        .join(format!("{native}.jsonl"))
}

fn append_turn(path: &Path, uuid: &str, role: &str, text: &str, cwd: &str) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    writeln!(
        file,
        "{{\"type\":\"{role}\",\"uuid\":\"{uuid}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"{role}\",\"content\":\"{text}\"}}}}"
    )
    .unwrap();
}

fn seed_transcript(path: &Path, cwd: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, String::new()).unwrap();
    append_turn(path, "u1", "user", "start the booknyx work", cwd);
    append_turn(path, "u2", "assistant", " scaffold laid out", cwd);
}

/// Launches A (claude stub) with a seeded transcript and waits for
/// the setup stub write (the race guard from the E207 fixture).
struct LaunchedSource {
    state: Arc<AppState>,
    cwd: String,
    session_id: String,
    transcript: PathBuf,
}

fn launch_source(work: &Path) -> LaunchedSource {
    let state = app_state(work);
    let project_dir = work.join("booknyx");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();

    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();
    {
        let setup_args = config_dir_of(&state, &account_a.id).join("last-args");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !setup_args.is_file() {
            assert!(std::time::Instant::now() < deadline, "setup stub never ran");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    let transcript = transcript_of(&state, &account_a.id, &cwd, "native-5");
    seed_transcript(&transcript, &cwd);
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.clone(),
            ))
            .unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: transcript.clone(),
            native_id: Some("native-5".to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }
    LaunchedSource {
        state,
        cwd,
        session_id: launched.session_id,
        transcript,
    }
}

#[test]
fn switch_targets_rank_badge_and_explain_never_lossless() {
    let work = tempfile::tempdir().unwrap();
    let source = launch_source(work.path());
    let state = &source.state;

    // A second same-Provider account (signed in) and a cross-Provider
    // account on the fake harness (left logged out — the filtered-row
    // contract carries its badge + Drop list all the same).
    let account_b = commands::create_account(state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let account_c = commands::create_account(state, FAKE_HARNESS_ID, "Zed Lab", None).unwrap();
    sign_in(state, &account_b.id);
    commands::create_profile(state, &account_b.id, "Main", None, None).unwrap();
    commands::create_profile(state, &account_c.id, "Main", None, None).unwrap();

    let targets = commands::switch_targets_impl(state, &source.session_id).unwrap();
    assert!(
        !targets.is_empty(),
        "the ranked destinations exist: {targets:?}"
    );

    // The source's own launch settings are never a destination.
    let labels: Vec<&str> = targets
        .iter()
        .map(|target| target.account_label.as_str())
        .collect();
    assert_eq!(labels, ["Personal", "Zed Lab"], "ranked then cross");

    // Same-Provider survivor: scored, explained (top-2 factor words,
    // plus the honest headroom-unknown note when quota is missing),
    // exactly one Suggested overall.
    let row_b = targets
        .iter()
        .find(|t| t.account_label == "Personal")
        .unwrap();
    assert!(row_b.score.is_some(), "survivor carries its §Q6 score");
    assert!(!row_b.cross_provider);
    assert!(!row_b.top_factors.is_empty() && row_b.top_factors.len() <= 3);
    assert_eq!(
        targets.iter().filter(|target| target.suggested).count(),
        1,
        "suggest-only: exactly one top survivor"
    );

    // Cross-Provider: Beta ALWAYS (D-11), Drop list mandatory, honest
    // hard-filter reason visible — and never "lossless" anywhere.
    let row_c = targets
        .iter()
        .find(|t| t.account_label == "Zed Lab")
        .unwrap();
    assert!(row_c.cross_provider);
    assert!(
        row_c.badge.contains("Beta"),
        "cross-Provider badge carries Beta: {}",
        row_c.badge
    );
    assert!(!row_c.drops.is_empty(), "the badge's honest companion");
    assert_eq!(row_c.filtered_reason.as_deref(), Some("logged out"));
    for target in &targets {
        assert!(!target.validated, "E210 owns flipping cells green");
        assert!(
            !target.badge.to_lowercase().contains("lossless"),
            "badge never lossless: {}",
            target.badge
        );
    }

    // Determinism: same inputs → same order + same explanation.
    let again = commands::switch_targets_impl(state, &source.session_id).unwrap();
    assert_eq!(targets, again, "ranking is deterministic");
}

#[test]
fn graph_renders_the_round_trip_and_hop_note() {
    let work = tempfile::tempdir().unwrap();
    let source = launch_source(work.path());
    let state = &source.state;

    let account_b = commands::create_account(state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let account_c = commands::create_account(state, CLAUDE_CODE_ID, "Lab", None).unwrap();
    let profile_b = commands::create_profile(state, &account_b.id, "Main", None, None).unwrap();
    let profile_c = commands::create_profile(state, &account_c.id, "Main", None, None).unwrap();

    // A → B (transplant), B → A (switch-back), A → C: three hops.
    let switched = commands::switch_execute_impl(
        state,
        &source.session_id,
        &profile_b.id,
        "stop-now",
        None,
        &[],
    )
    .unwrap();
    let b_transcript = transcript_of(state, &account_b.id, &source.cwd, "native-5");
    append_turn(
        &b_transcript,
        "b1",
        "user",
        "fixed the login bug",
        &source.cwd,
    );
    let returned =
        commands::switch_back_execute_impl(state, &switched.session_id, &[], true).unwrap();
    commands::switch_execute_impl(
        state,
        &returned.session_id,
        &profile_c.id,
        "stop-now",
        None,
        &[],
    )
    .unwrap();

    let project_id = {
        let storage = state.storage.lock().unwrap();
        storage
            .projects()
            .by_root(&std::fs::canonicalize(&source.cwd).unwrap())
            .unwrap()
            .expect("project recorded")
            .id
            .to_string()
    };
    let works = commands::switch_graph_overview_impl(state, &project_id).unwrap();
    assert_eq!(works.len(), 1, "one work item");
    let timeline = &works[0];

    // The travel: transplant, switch-back ↺, transplant — labelled.
    assert_eq!(timeline.edges.len(), 3, "three hops");
    assert!(timeline.edges[0].label.contains("transplant"));
    assert!(
        timeline.edges[1].label.contains("switch-back ↺"),
        "the return is marked: {}",
        timeline.edges[1].label
    );
    assert!(timeline.edges[1].switch_back);
    assert!(timeline.edges[2].label.contains("transplant"));

    // S-F9: the cumulative hop line informs, never blocks.
    assert!(
        timeline
            .hop_note
            .as_deref()
            .is_some_and(|note| note.contains("3 switches")),
        "hop note present: {:?}",
        timeline.hop_note
    );

    // Nodes: resumable where the transcript survives, labelled with
    // real accounts; ids stay machine-qualified-ready (D-13).
    assert!(timeline.epochs.iter().any(|node| node.resumable));
    for node in &timeline.epochs {
        assert!(!node.id.is_empty() && !node.account_label.is_empty());
        assert!(!node.provider.is_empty() && !node.kind.is_empty());
    }
    for edge in &timeline.edges {
        assert!(!edge.from.is_empty() && !edge.label.is_empty());
    }
    // No fork in a linear A→B→A→C travel: nothing collapsed.
    assert_eq!(timeline.collapsed, 0);
}

#[test]
fn pack_export_blocks_until_policy_then_strips_the_canary() {
    let work = tempfile::tempdir().unwrap();
    let source = launch_source(work.path());
    let state = &source.state;

    // The canary rides a real turn (never a live key; the E202
    // guard's own canary shape).
    append_turn(
        &source.transcript,
        "k1",
        "user",
        "rotate the key sk-live-DO-NOT-SHIP-9876543210 now",
        &source.cwd,
    );

    let account_b = commands::create_account(state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let profile_b = commands::create_profile(state, &account_b.id, "Main", None, None).unwrap();
    commands::switch_execute_impl(
        state,
        &source.session_id,
        &profile_b.id,
        "stop-now",
        None,
        &[],
    )
    .unwrap();

    // The A epoch (the source side of the recorded edge).
    let epoch_a = {
        let storage = state.storage.lock().unwrap();
        let graph = storage.switch_graph();
        let project = storage
            .projects()
            .by_root(&std::fs::canonicalize(&source.cwd).unwrap())
            .unwrap()
            .expect("project recorded");
        let work_row = graph.works_for_project(&project.id).unwrap()[0].id.clone();
        graph.edges_for_work(&work_row).unwrap()[0]
            .from_epoch
            .to_string()
    };

    // Unset GLOBAL policy blocks the standalone export — exactly like
    // switches; no default is ever applied for the user.
    let blocked = commands::pack_export_impl(state, &epoch_a).unwrap_err();
    assert!(
        blocked
            .to_string()
            .contains("Choose how secrets are handled in packs first"),
        "unset blocks: {blocked}"
    );

    commands::redaction_policy_set_global_impl(state, Some("strip")).unwrap();
    let path = commands::pack_export_impl(state, &epoch_a).unwrap();
    let dir = PathBuf::from(&path);
    assert!(dir.is_dir(), "pack directory written: {path}");
    assert!(dir.join("manifest.json").is_file(), "manifest present");

    // Zero canary bytes across the WHOLE export (E202's always-on
    // guard would have refused the write otherwise — this is the
    // belt to its braces).
    let mut files = vec![dir.clone()];
    let mut scanned = 0;
    while let Some(entry) = files.pop() {
        for child in std::fs::read_dir(&entry).unwrap().flatten() {
            let child = child.path();
            if child.is_dir() {
                files.push(child);
            } else {
                scanned += 1;
                let bytes = std::fs::read(&child).unwrap();
                assert!(
                    !bytes.windows(7).any(|window| window == b"sk-live"),
                    "canary survived in {}",
                    child.display()
                );
            }
        }
    }
    assert!(scanned >= 2, "manifest + conversation scanned: {scanned}");
}
