//! AJ-E207 integration: the A→B→A round trip — resume the ORIGINAL
//! native session with exactly ONE delta turn, three-way
//! reconciliation over the fork (uuid sets + tracked files), S-F8
//! unresolved conflicts keep the work forked, S-F7 offers the
//! honest fallback, and the copy-back ban holds LIVE (the original
//! transcript's bytes never change through the whole round trip).

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
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
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

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = claude_stub(dir);
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn config_dir_of(state: &Arc<AppState>, account_id: &str) -> PathBuf {
    state.accounts_dir().join(CLAUDE_CODE_ID).join(account_id)
}

fn transcript_of(state: &Arc<AppState>, account_id: &str, cwd: &str, native: &str) -> PathBuf {
    config_dir_of(state, account_id)
        .join("projects")
        .join(cwd.replace('/', "-"))
        .join(format!("{native}.jsonl"))
}

/// Appends one uuid-bearing turn to a transcript (the append-only
/// jsonl contract).
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

/// The A→B half both E207 tests start from: A runs, switches to B
/// (transplant), and the fork edge's manifest gains a tracked
/// non-transcript row (`todos.json`) both sides will edit.
struct RoundTrip {
    state: Arc<AppState>,
    cwd: String,
    away_session: String,
    a_transcript: PathBuf,
    b_transcript: PathBuf,
}

fn switch_a_to_b(work: &Path) -> RoundTrip {
    let state = app_state(work);
    let project_dir = work.join("booknyx");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();

    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();

    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();
    // The setup launch's stub records its (empty) argv asynchronously;
    // wait for THAT write to land so it can never race the return's
    // argv record later (a late truncate clobbered its first byte in
    // flakes). One write then exit — existing file means done.
    {
        let setup_args = config_dir_of(&state, &account_a.id).join("last-args");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !setup_args.is_file() {
            assert!(std::time::Instant::now() < deadline, "setup stub never ran");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    let a_transcript = transcript_of(&state, &account_a.id, &cwd, "native-5");
    seed_transcript(&a_transcript, &cwd);
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.clone(),
            ))
            .unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: a_transcript.clone(),
            native_id: Some("native-5".to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }

    // The juggle: A → B.
    let switched = commands::switch_execute_impl(
        &state,
        &launched.session_id,
        &profile_b.id,
        "stop-now",
        None,
        &[],
    )
    .unwrap();
    let b_transcript = transcript_of(&state, &account_b.id, &cwd, "native-5");
    assert!(b_transcript.is_file(), "transcript transplanted to B");

    // The fork manifest's tracked todo artifact (the file domain —
    // kind != transcript). Both sides then edit it: a real conflict.
    let todos_row = vec![juggler_core::model::ManifestEntry {
        rel_path: "todos.json".to_owned(),
        sha256: "base-todos".to_owned(),
        bytes: 0,
        kind: "todos".to_owned(),
        disposition: "copy".to_owned(),
    }];
    {
        let storage = state.storage.lock().unwrap();
        storage
            .switch_graph()
            .append_manifest(
                &juggler_core::model::SwitchEdgeId::new(switched.edge_id.clone()),
                &todos_row,
            )
            .unwrap();
    }
    std::fs::write(
        b_transcript.parent().unwrap().join("todos.json"),
        "away todos",
    )
    .unwrap();
    std::fs::write(
        a_transcript.parent().unwrap().join("todos.json"),
        "original todos",
    )
    .unwrap();

    RoundTrip {
        state,
        cwd,
        away_session: switched.session_id,
        a_transcript,
        b_transcript,
    }
}

#[test]
fn a_to_b_to_a_resumes_original_with_one_delta_and_no_copy_back() {
    let work = tempfile::tempdir().unwrap();
    let trip = switch_a_to_b(work.path());

    // Both sides progress: B adds away turns (one RE-EMITTED — the
    // dedupe-first-wins contract), A adds an original-only turn.
    append_turn(
        &trip.b_transcript,
        "b1",
        "user",
        "fixed the login bug",
        &trip.cwd,
    );
    append_turn(
        &trip.b_transcript,
        "b1",
        "user",
        "fixed the login bug",
        &trip.cwd,
    );
    append_turn(
        &trip.b_transcript,
        "b2",
        "assistant",
        "suite green",
        &trip.cwd,
    );
    append_turn(
        &trip.a_transcript,
        "a1",
        "user",
        "renamed the module",
        &trip.cwd,
    );

    // The suggestion exists (suggest-only) with the verbatim copy.
    let suggestions = commands::switch_back_suggestions_impl(&trip.state).unwrap();
    assert_eq!(suggestions.len(), 1, "one open excursion");
    assert_eq!(
        suggestions[0].copy,
        "Work Max has reset. Resume the original conversation there? Work from Personal \
         arrives as one 'while you were away' update."
    );
    assert_eq!(suggestions[0].session_id, trip.away_session);

    // The plan: exact three-way counts, the file conflict, the
    // delta preview, S-F7 clean.
    let plan = commands::switch_back_plan_impl(&trip.state, &trip.away_session).unwrap();
    assert_eq!(plan.original_native_id, "native-5");
    assert!(plan.original_alive);
    assert_eq!(plan.base_turns, 2, "u1+u2 shared");
    assert_eq!(plan.away_only_turns, 2, "b1 (deduped) + b2");
    assert_eq!(plan.original_only_turns, 1, "a1");
    assert_eq!(plan.conflicts.len(), 1, "todos.json edited on both sides");
    assert_eq!(plan.conflicts[0].rel_path, "todos.json");
    assert!(
        plan.delta_preview
            .contains("While you were away on Personal")
    );
    assert!(plan.delta_preview.contains("fixed the login bug"));
    assert!(
        plan.divergence
            .contains("2 new turns and 1 file conflicts need your choice")
    );
    assert!(plan.away_live, "the away instance still runs");

    // Unresolved conflicts keep the work forked (S-F8) — and touch
    // nothing.
    let forked = commands::switch_back_execute_impl(&trip.state, &trip.away_session, &[], true);
    assert!(forked.is_err(), "S-F8 refuses to guess");

    // Execute with the user's choice: take the away todos, keep B
    // running (legal — the both-live copy said so).
    let hash_before_return =
        juggler_handover::manifest::sha256_hex(&std::fs::read(&trip.a_transcript).unwrap());
    let choices = vec![commands::SwitchBackChoiceIn {
        rel_path: "todos.json".to_owned(),
        choice: "take-away".to_owned(),
    }];
    let returned =
        commands::switch_back_execute_impl(&trip.state, &trip.away_session, &choices, true)
            .unwrap();
    assert_ne!(returned.session_id, trip.away_session);

    // THE copy-back ban, live: the return wrote NOTHING to A's
    // transcript (the a1 turn we appended is still the last change).
    assert_eq!(
        hash_before_return,
        juggler_handover::manifest::sha256_hex(&std::fs::read(&trip.a_transcript).unwrap()),
        "original transcript never written"
    );
    // B's transcript is untouched too (the return resumes A, it
    // does not rewrite B).
    assert!(
        std::fs::read_to_string(&trip.b_transcript)
            .unwrap()
            .contains("suite green")
    );

    // The choice applied: original side's todos.json is the away
    // version now.
    assert_eq!(
        std::fs::read_to_string(trip.a_transcript.parent().unwrap().join("todos.json")).unwrap(),
        "away todos"
    );

    // A resumed with its OWN native id plus exactly ONE delta turn
    // (the stub writes its argv asynchronously — poll for content).
    let args_path = config_dir_of_from(&trip.state, &trip.a_transcript).join("last-args");
    let args = poll_args(&args_path);
    assert!(
        args.contains("--resume") && args.contains("native-5"),
        "resumes the ORIGINAL native id: {args}"
    );
    assert_eq!(
        args.matches("While you were away").count(),
        1,
        "exactly one delta turn"
    );
    assert!(
        args.contains("fixed the login bug"),
        "delta carries away work"
    );
    assert!(
        !args.contains("suite green\n- [user] start"),
        "deduped first-wins holds"
    );

    // The graph: fork + switch-back, parent-linked, resumed_from the
    // ORIGINAL epoch, the choice recorded in the manifest.
    {
        let storage = trip.state.storage.lock().unwrap();
        let graph = storage.switch_graph();
        let project = storage
            .projects()
            .by_root(&std::fs::canonicalize(&trip.cwd).unwrap())
            .unwrap()
            .expect("project recorded");
        let works = graph.works_for_project(&project.id).unwrap();
        assert_eq!(works.len(), 1);
        let edges = graph.edges_for_work(&works[0].id).unwrap();
        assert_eq!(edges.len(), 2, "fork + return");
        let back = &edges[1];
        assert_eq!(
            back.parent_edge_a.as_ref().map(|id| id.as_str()),
            Some(edges[0].id.as_str())
        );
        let epochs = graph.epochs_for_work(&works[0].id).unwrap();
        let original_epoch = epochs
            .iter()
            .find(|epoch| epoch.id == edges[0].from_epoch)
            .unwrap();
        assert_eq!(
            back.parent_edge_a.as_ref(),
            Some(&edges[0].id),
            "parent = the fork"
        );
        let back_epoch = epochs
            .iter()
            .find(|epoch| Some(&epoch.id) == back.to_epoch.as_ref())
            .unwrap();
        assert_eq!(
            back_epoch.resumed_from_epoch,
            Some(original_epoch.id.clone()),
            "resumes the ORIGINAL epoch"
        );
        let rows = graph.manifest_for(&back.id).unwrap();
        assert!(
            rows.iter().any(|row| row.kind == "conflict-choice"
                && row.rel_path == "todos.json"
                && row.disposition == "take-away"),
            "choice recorded: {rows:?}"
        );
    }

    // The excursion is closed: no further suggestion.
    assert!(
        commands::switch_back_suggestions_impl(&trip.state)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn sf7_offers_the_honest_fallback_when_the_original_is_deleted() {
    let work = tempfile::tempdir().unwrap();
    let trip = switch_a_to_b(work.path());
    append_turn(&trip.b_transcript, "b1", "user", "away work", &trip.cwd);

    // The original transcript is gone (deleted/expired).
    std::fs::remove_file(&trip.a_transcript).unwrap();

    let plan = commands::switch_back_plan_impl(&trip.state, &trip.away_session).unwrap();
    assert!(!plan.original_alive);
    assert_eq!(plan.sf7_fallback, "fresh-f0", "no surviving epoch on A");
    assert!(
        plan.sf7_copy.contains("S-F7"),
        "§7.1 copy names the situation: {}",
        plan.sf7_copy
    );
}

fn config_dir_of_from(state: &Arc<AppState>, transcript: &Path) -> PathBuf {
    // A's config dir root: <config>/projects/<encoded>/<file> → up 3.
    let _ = state;
    transcript
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn poll_args(path: &Path) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let ready = std::fs::read_to_string(path)
            .ok()
            .filter(|args| !args.trim().is_empty());
        if let Some(args) = ready {
            return args;
        }
        if std::time::Instant::now() > deadline {
            panic!("stub never recorded its resume args at {}", path.display());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
