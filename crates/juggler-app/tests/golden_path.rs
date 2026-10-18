//! Golden-path integration (experience blueprint items 2+3): real project
//! history surfaces from imported transcripts, and hand-over moves a
//! session's context to another account and resumes it there — the
//! product's namesake moment, proven with a stub harness.

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

/// A claude stub that records its argv under the isolated config dir and
/// exits — enough to prove the handover launched with `--resume <id>`.
fn claude_stub(dir: &Path) -> std::path::PathBuf {
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

/// Writes a transcript shaped like Claude's: message lines carrying `cwd`
/// (the only unambiguous project-path source).
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

#[test]
fn backfill_surfaces_real_project_history_from_transcripts() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());

    let account = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let config = config_dir_of(&state, &account.id);
    seed_transcript(&config, "/Users/dev/booknyx", "sess-old-1");
    seed_transcript(&config, "/Users/dev/booknyx", "sess-old-2");
    seed_transcript(&config, "/Users/dev/payments", "sess-old-3");

    let enriched = commands::projects_backfill_impl(&state).unwrap();
    assert_eq!(enriched, 2, "two distinct project cwds");

    let projects = commands::projects_list_impl(&state).unwrap();
    let booknyx = projects
        .iter()
        .find(|p| p.name == "booknyx")
        .expect("real project surfaced");
    assert_eq!(booknyx.root, "/Users/dev/booknyx");
    assert_eq!(booknyx.accounts_used, vec!["Work Max".to_string()]);
    assert_eq!(booknyx.history_sessions, 2);
    assert_eq!(booknyx.instance_count, 0, "no AI Juggler runs yet");

    // Idempotent: re-running does not duplicate.
    assert_eq!(commands::projects_backfill_impl(&state).unwrap(), 2);
}

#[test]
fn handover_moves_context_to_another_account_and_resumes_it() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project_dir = work.path().join("booknyx");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();

    let account_a = commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let profile_a = commands::create_profile(&state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();

    // A runs in the project and produces a transcript (seeded directly —
    // the stub doesn't write real transcripts).
    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();
    seed_transcript(
        &config_dir_of(&state, &account_a.id),
        &cwd,
        "native-sess-42",
    );
    let source_transcript = config_dir_of(&state, &account_a.id)
        .join("projects")
        .join(cwd.replace('/', "-"))
        .join("native-sess-42.jsonl");
    let source_hash_before =
        juggler_handover::manifest::sha256_hex(&std::fs::read(&source_transcript).unwrap());
    {
        // What the watcher would have discovered (AJ-E204: the
        // record is the resolution source).
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.clone(),
            ))
            .unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: source_transcript.clone(),
            native_id: Some("native-sess-42".to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }
    let sessions_before = {
        let storage = state.storage.lock().unwrap();
        storage.sessions().list(None).unwrap()
    };
    assert_eq!(sessions_before.len(), 1);

    // The juggle: hand the session over to account B.
    let handed = commands::session_handover_impl(&state, &launched.session_id, &profile_b.id)
        .expect("handover");
    assert_ne!(handed.session_id, launched.session_id);

    // Transcript transplanted byte-for-byte into B's config, same encoded
    // project subpath.
    let encoded = cwd.replace('/', "-");
    let dest = config_dir_of(&state, &account_b.id)
        .join("projects")
        .join(&encoded)
        .join("native-sess-42.jsonl");
    assert!(dest.is_file(), "transcript transplanted");
    // AJ-E204: copy-never-move — the source hash is invariant.
    assert_eq!(
        source_hash_before,
        juggler_handover::manifest::sha256_hex(&std::fs::read(&source_transcript).unwrap()),
        "source byte-identical after transplant"
    );

    // AJ-E205: the switch recorded its edge in the Switch Graph —
    // one work item, two epochs, one CONFIRMED edge with manifest
    // rows (append-only history from day one).
    {
        let storage = state.storage.lock().unwrap();
        let graph = storage.switch_graph();
        let work = graph
            .works_for_project(&juggler_core::model::ProjectId::new(
                storage
                    .projects()
                    .by_root(&std::fs::canonicalize(&cwd).unwrap())
                    .unwrap()
                    .map(|p| p.id.as_str().to_owned())
                    .unwrap_or_default(),
            ))
            .unwrap();
        assert_eq!(work.len(), 1, "one work item per project");
        let snapshot = graph.graph(&work[0].id).unwrap();
        assert_eq!(snapshot.epochs.len(), 2, "source + target epochs");
        assert_eq!(snapshot.edges.len(), 1, "one edge");
        assert!(
            matches!(
                snapshot.edges[0].state,
                juggler_core::model::EdgeState::Confirmed
            ),
            "edge confirmed: {:?}",
            snapshot.edges[0].state
        );
        assert_eq!(
            snapshot.edges[0]
                .to_epoch
                .as_ref()
                .map(|e| e.as_str().is_empty()),
            Some(false)
        );
        assert!(
            !graph
                .manifest_for(&snapshot.edges[0].id)
                .unwrap()
                .is_empty()
        );
    }

    // B's instance launched with --resume native-sess-42 (the stub runs
    // asynchronously — wait for its argv record).
    let args_path = config_dir_of(&state, &account_b.id).join("last-args");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let args = loop {
        if let Ok(args) = std::fs::read_to_string(&args_path) {
            break args;
        }
        if std::time::Instant::now() > deadline {
            panic!("stub never recorded its args");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    assert!(
        args.contains("--resume") && args.contains("native-sess-42"),
        "resume args: {args}"
    );

    // Lineage: source HandedOver, continuation recorded on the same
    // project with its native id and transcript ref.
    let sessions = {
        let storage = state.storage.lock().unwrap();
        storage.sessions().list(None).unwrap()
    };
    let source = sessions
        .iter()
        .find(|s| s.id.as_str() == launched.session_id)
        .unwrap();
    assert_eq!(source.status, SessionStatus::HandedOver);
    let continuation = sessions
        .iter()
        .find(|s| s.id.as_str() == handed.session_id)
        .unwrap();
    assert_eq!(
        continuation.conversation.as_ref().map(|c| c.as_str()),
        Some("native-sess-42")
    );
    assert_eq!(continuation.project, source.project);
    assert!(continuation.transcript_ref.is_some());

    // One project, both accounts in history — the juggling view.
    let projects = commands::projects_list_impl(&state).unwrap();
    let mut used = projects[0].accounts_used.clone();
    used.sort();
    assert_eq!(
        used,
        vec!["Personal".to_string(), "Work Max".to_string()],
        "project history shows both accounts"
    );
    assert_eq!(projects[0].instance_count, 2);
}
