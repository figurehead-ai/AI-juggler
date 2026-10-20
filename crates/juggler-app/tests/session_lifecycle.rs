//! AJ-E1007 integration: archive / unarchive round-trips exactly; delete
//! has no single-call path (preview token, refusals for live and
//! lineage, Provider transcript untouched); transcript search finds the
//! right session with a snippet over 1,000 sessions inside the budgets;
//! replay is byte-identical; retention never deletes without its dry
//! run's token and only lists what is past the cutoff.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::history;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, Timestamp, TranscriptRef,
};
use sha2::{Digest, Sha256};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn seed_account(state: &Arc<AppState>, dir: &Path) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-work"),
            harness: HarnessId::new("claude-code"),
            label: "Work Max".into(),
            plan: None,
            state_dir: dir.join("state-work"),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    storage
        .profiles()
        .insert(&LaunchRecipe {
            id: ProfileId::new("rec-work"),
            name: "Main".into(),
            account: AccountId::new("acc-work"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
}

fn transcript(dir: &Path, id: &str, lines: &[&str]) -> PathBuf {
    let path = dir.join(format!("{id}.jsonl"));
    let body: String = lines
        .iter()
        .map(|text| {
            format!(
                "{}\n",
                serde_json::json!({"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":text}]}})
            )
        })
        .collect();
    std::fs::write(&path, body).unwrap();
    path
}

fn session(
    state: &Arc<AppState>,
    dir: &Path,
    id: &str,
    status: SessionStatus,
    last_active_ms: i64,
    lines: &[&str],
) {
    let path = transcript(dir, id, lines);
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new(id),
            conversation: None,
            profile: ProfileId::new("rec-work"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status,
            started_at: Timestamp::from_millis(last_active_ms - 60_000),
            last_activity: Some(Timestamp::from_millis(last_active_ms)),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: Some(TranscriptRef {
                path,
                native_id: None,
            }),
            window: None,
        })
        .unwrap();
}

fn lifecycle_of(state: &Arc<AppState>, id: &str) -> Option<String> {
    juggler_app::commands::instances_shared_snapshot(state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == id)
        .and_then(|row| row.lifecycle)
}

#[test]
fn archive_round_trips_exactly_and_delete_has_no_single_call_path() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed_account(&state, work.path());
    let now = Timestamp::now().millis();
    session(
        &state,
        work.path(),
        "s-done",
        SessionStatus::Finished,
        now,
        &["shipped the invoice export"],
    );
    session(
        &state,
        work.path(),
        "s-live",
        SessionStatus::Running,
        now,
        &["still going"],
    );

    let before = juggler_app::commands::instances_shared_snapshot(&state).unwrap();
    let started = Instant::now();
    history::instance_archive_impl(&state, "s-done").unwrap();
    assert_eq!(lifecycle_of(&state, "s-done").as_deref(), Some("archived"));
    history::instance_unarchive_impl(&state, "s-done").unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
    let after = juggler_app::commands::instances_shared_snapshot(&state).unwrap();
    let strip = |rows: Vec<juggler_app::commands::InstanceInfo>| {
        rows.into_iter()
            .map(|mut row| {
                row.lifecycle = None;
                row.lifecycle_at_ms = None;
                row
            })
            .map(|row| serde_json::to_value(row).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(strip(after), strip(before), "unarchive restores it exactly");
    assert!(
        history::instance_archive_impl(&state, "s-live").is_err(),
        "live cannot be archived"
    );

    // No single-call delete: a guessed token is refused and nothing goes.
    assert!(history::instance_delete_impl(&state, "s-done", "guess").is_err());
    assert!(lifecycle_of(&state, "s-done").is_some());
    let preview = history::instance_delete_preview_impl(&state, "s-done").unwrap();
    assert!(preview.refused.is_none());
    assert!(preview.removes.iter().any(|line| line.contains("history")));
    assert!(
        preview
            .keeps
            .iter()
            .any(|line| line.contains("transcript file"))
    );
    let token = preview.token.unwrap();
    history::instance_delete_impl(&state, "s-done", &token).unwrap();
    assert!(
        juggler_app::commands::instances_shared_snapshot(&state)
            .unwrap()
            .iter()
            .all(|row| row.id != "s-done")
    );
    assert!(
        work.path().join("s-done.jsonl").exists(),
        "the Provider's transcript is never deleted"
    );
    assert!(
        history::instance_delete_impl(&state, "s-done", &token).is_err(),
        "a token is spent once"
    );

    let live = history::instance_delete_preview_impl(&state, "s-live").unwrap();
    assert!(live.refused.unwrap().contains("still live"));
    assert!(live.token.is_none());
}

#[test]
fn search_finds_the_session_with_a_snippet_over_1000_sessions_and_replay_is_byte_identical() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed_account(&state, work.path());
    let now = Timestamp::now().millis();
    for n in 0..1000 {
        let text = format!("routine maintenance note {n} about the build");
        let needle = "we traced the flaky Stripe webhook retry to a clock skew";
        let lines: Vec<&str> = if n == 777 {
            vec![text.as_str(), needle]
        } else {
            vec![text.as_str()]
        };
        session(
            &state,
            work.path(),
            &format!("s-{n}"),
            SessionStatus::Finished,
            now,
            &lines,
        );
    }
    let started = Instant::now();
    let report = history::index_transcripts(&state).unwrap();
    let built = started.elapsed();
    eprintln!("[perf] index build over 1,000 sessions: {built:?} ({report:?})");
    assert_eq!(report.sessions, 1000);
    assert!(built < Duration::from_secs(30), "{built:?}");

    let started = Instant::now();
    let hits = history::transcript_search_impl(&state, "stripe webhook").unwrap();
    let took = started.elapsed();
    eprintln!("[perf] transcript search over 1,000 sessions: {took:?}");
    assert!(took < Duration::from_millis(500), "{took:?}");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].session_id, "s-777");
    assert!(
        hits[0].snippet.contains("Stripe webhook retry"),
        "{}",
        hits[0].snippet
    );
    // Incremental: nothing new, nothing re-read.
    assert_eq!(history::index_transcripts(&state).unwrap().lines, 0);
    // Appended text is found on the next search.
    let path = work.path().join("s-3.jsonl");
    let mut body = std::fs::read_to_string(&path).unwrap();
    body.push_str(&format!(
        "{}\n",
        serde_json::json!({"message":{"content":"the kubernetes ingress"}})
    ));
    std::fs::write(&path, &body).unwrap();
    let hits = history::transcript_search_impl(&state, "kubernetes").unwrap();
    assert_eq!(
        hits.iter()
            .map(|h| h.session_id.as_str())
            .collect::<Vec<_>>(),
        ["s-3"]
    );
    // Index correctness on the fixture: every "note N" finds exactly s-N.
    for n in [0, 1, 500, 999] {
        let hits = history::transcript_search_impl(&state, &format!("note {n} about")).unwrap();
        assert!(
            hits.iter().any(|h| h.session_id == format!("s-{n}")),
            "{n}: {hits:?}"
        );
    }
    // FTS syntax from the user is just words.
    assert!(history::transcript_search_impl(&state, "NEAR(\"x\" OR").is_ok());

    history::instance_archive_impl(&state, "s-777").unwrap();
    let replay = history::instance_replay_impl(&state, "s-777").unwrap();
    let file = std::fs::read(work.path().join("s-777.jsonl")).unwrap();
    assert_eq!(replay.transcript.as_bytes(), &file[..], "byte-identical");
    let sha: String = Sha256::digest(&file)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(replay.sha256.as_deref(), Some(sha.as_str()));
    assert_eq!(replay.entries.len(), 2);
    assert_eq!(replay.entries[1].role, "assistant");
    assert!(replay.history.iter().any(|event| event.what == "archived"));
}

#[test]
fn retention_lists_first_and_never_deletes_without_its_dry_run() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed_account(&state, work.path());
    let now = Timestamp::now().millis();
    let day = 24 * 3_600_000;
    session(
        &state,
        work.path(),
        "s-old",
        SessionStatus::Finished,
        now - 100 * day,
        &["old work"],
    );
    session(
        &state,
        work.path(),
        "s-new",
        SessionStatus::Finished,
        now - day,
        &["new work"],
    );
    session(
        &state,
        work.path(),
        "s-old-live",
        SessionStatus::WaitingUser,
        now - 100 * day,
        &["waiting"],
    );

    // Keep forever by default: nothing to review.
    let storage = history::account_storage_impl(&state).unwrap();
    assert_eq!(storage[0].retention_days, None);
    assert_eq!(storage[0].instances, 3);
    assert!(storage[0].transcript_bytes > 0);
    assert!(
        history::retention_preview_impl(&state, "acc-work")
            .unwrap()
            .token
            .is_none()
    );

    history::retention_set_impl(&state, "acc-work", Some(90)).unwrap();
    assert!(history::retention_set_impl(&state, "acc-work", Some(0)).is_err());
    let preview = history::retention_preview_impl(&state, "acc-work").unwrap();
    assert_eq!(
        preview
            .deletes
            .iter()
            .map(|row| row.session_id.as_str())
            .collect::<Vec<_>>(),
        ["s-old"]
    );
    assert_eq!(
        preview.kept.len(),
        1,
        "the live one is kept, with its reason"
    );
    assert!(
        preview.kept[0]
            .kept_because
            .as_ref()
            .unwrap()
            .contains("live")
    );
    // The dry run deleted nothing; a wrong token deletes nothing.
    assert_eq!(
        history::account_storage_impl(&state).unwrap()[0].instances,
        3
    );
    assert!(history::retention_apply_impl(&state, "acc-work", "guess").is_err());
    assert_eq!(
        history::account_storage_impl(&state).unwrap()[0].instances,
        3
    );
    // Only the dry run's token applies it.
    let preview = history::retention_preview_impl(&state, "acc-work").unwrap();
    let deleted =
        history::retention_apply_impl(&state, "acc-work", preview.token.as_deref().unwrap())
            .unwrap();
    assert_eq!(deleted, 1);
    let after = history::account_storage_impl(&state).unwrap();
    assert_eq!(after[0].instances, 2);
    assert!(work.path().join("s-old.jsonl").exists(), "transcripts stay");
}
