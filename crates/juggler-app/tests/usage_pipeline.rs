//! AJ-E607 pipeline integration: a seeded account's managed dir
//! (statusline spool + transcript tree) flows into the AJ-E606 store
//! through ONE `ingest_pass`, idempotently — the second pass (no file
//! changes) inserts zero new rows, and the health map records the
//! sources.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_app::state::AppState;
use juggler_app::usage_pipeline;
use juggler_core::model::{
    Account, AccountId, HarnessId, LaunchRecipe, MachineId, ProfileId, SessionId, SessionStatus,
    Timestamp,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(
        AppState::init_with_registry(
            dir.join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    )
}

fn seed_claude_account(state: &Arc<AppState>, dir: &Path) {
    let managed = dir.join("managed-claude");
    std::fs::create_dir_all(managed.join("projects")).unwrap();
    // AI Juggler's own copy carries its marker (the status line is only
    // ever installed into a marked folder — ADR-0018).
    std::fs::write(
        managed.join(juggler_adapters::claude_code::MANAGED_MARKER),
        "",
    )
    .unwrap();
    // settings.json WITHOUT a prior statusline (a fresh managed dir).
    std::fs::write(
        managed.join("settings.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"model": "claude-opus-4"})).unwrap(),
    )
    .unwrap();
    // A transcript with two keyed usage lines + one resumed duplicate.
    std::fs::write(
        managed.join("projects").join("t.jsonl"),
        concat!(
            "{\"type\":\"assistant\",\"uuid\":\"p-1\",\"sessionId\":\"s\",\"timestamp\":\"2026-10-15T20:00:01.000Z\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}\n",
            "{\"type\":\"assistant\",\"uuid\":\"p-2\",\"sessionId\":\"s\",\"timestamp\":\"2026-10-15T20:00:02.000Z\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":20,\"output_tokens\":2}}}\n",
            "{\"type\":\"assistant\",\"uuid\":\"p-1\",\"sessionId\":\"s\",\"timestamp\":\"2026-10-15T20:00:01.000Z\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}\n",
        ),
    )
    .unwrap();
    // A spool with ONE quota-bearing payload.
    std::fs::create_dir_all(managed.join("quota-spool")).unwrap();
    std::fs::write(
        managed.join("quota-spool").join("statusline.jsonl"),
        "1760000000000\t{\"rate_limits\":{\"five_hour\":{\"used_percentage\":33.0,\"resets_at\":1760006000000}}}\n",
    )
    .unwrap();

    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-pipe"),
            harness: HarnessId::new("claude-code"),
            label: "pipe".into(),
            plan: None,
            state_dir: managed,
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .profiles()
        .insert(&LaunchRecipe {
            id: ProfileId::new("rec-pipe"),
            name: "pipe".into(),
            account: AccountId::new("acc-pipe"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    // A session row so the account is "real" to any read model.
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&juggler_core::model::SessionRecord {
            id: SessionId::new("sess-pipe"),
            conversation: None,
            profile: ProfileId::new("rec-pipe"),
            machine: MachineId::new("this-mac"),
            kind: juggler_core::model::InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Finished,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
}

#[test]
fn one_pass_ingests_all_sources_and_repeats_insert_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_claude_account(&state, dir.path());

    // Pass 1 (tick 4 = file-scan tick): spool + transcript + install.
    usage_pipeline::ingest_pass(&state, 4);
    let events = {
        let storage = state.storage.lock().unwrap();
        storage.usage().events_for_account("acc-pipe", 0).unwrap()
    };
    // p-1 once, p-2 once (the duplicate uuid absorbed at the store).
    assert_eq!(events.len(), 2);
    let samples = {
        let storage = state.storage.lock().unwrap();
        storage
            .usage()
            .samples_for_account("acc-pipe", juggler_core::quota::WindowKind::FiveHour)
            .unwrap()
    };
    assert_eq!(samples.len(), 1);
    assert!((samples[0].used_pct - 33.0).abs() < 1e-9);

    // The managed wrapper is installed and CHAINS nothing (no prior).
    let managed = dir.path().join("managed-claude");
    assert!(managed.join("juggler-statusline.sh").exists());

    // Pass 2 (tick 8): no file changes ⇒ zero new rows.
    usage_pipeline::ingest_pass(&state, 8);
    let again = {
        let storage = state.storage.lock().unwrap();
        storage.usage().events_for_account("acc-pipe", 0).unwrap()
    };
    assert_eq!(
        again.len(),
        2,
        "pipeline idempotency: no changes, no inserts"
    );

    // Health: both sources observed.
    let health = usage_pipeline::source_health(&state);
    let sources: Vec<&str> = health.iter().map(|info| info.source.as_str()).collect();
    assert!(sources.contains(&"statusline"), "health: {sources:?}");
    assert!(sources.contains(&"transcript"), "health: {sources:?}");
    let transcript = health
        .iter()
        .find(|info| info.source == "transcript")
        .unwrap();
    assert!(transcript.last_event_at.is_some());
    assert_eq!(transcript.parse_errors, 0);
}

/// The 0.0.12 report: Codex's own limit readings (rollout `token_count`
/// `rate_limits`) reach the store as Measured samples, the account's
/// windows read as measured, and its credits show. Fixture values are
/// invented; the shape is the real line's.
#[test]
fn codex_rollout_limits_reach_the_display_as_measured_with_credits() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let home = dir.path().join("codex-home");
    let day = home.join("sessions/2026/10/21");
    std::fs::create_dir_all(&day).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../juggler-adapters/tests/fixtures/quota/codex-rollout-rate-limits.jsonl"),
        day.join("rollout-2026-10-21T10-00-00-th-9.jsonl"),
    )
    .unwrap();
    let account = Account {
        id: AccountId::new("acc-codex"),
        harness: HarnessId::new("codex"),
        label: "codex".into(),
        plan: None,
        state_dir: home,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&account)
        .unwrap();

    usage_pipeline::ingest_pass(&state, 4);

    let storage = state.storage.lock().unwrap();
    let five = storage
        .usage()
        .samples_for_account("acc-codex", juggler_core::quota::WindowKind::FiveHour)
        .unwrap();
    let week = storage
        .usage()
        .samples_for_account(
            "acc-codex",
            juggler_core::quota::WindowKind::Weekly { anchor_ms: None },
        )
        .unwrap();
    assert_eq!(five.len(), 2);
    assert_eq!(week.len(), 1);

    let now = juggler_adapters::quota_ingest::iso_to_epoch_ms("2026-10-21T10:35:00.000Z").unwrap();
    let info = juggler_app::commands::quota_display_inner(&storage, &account, now);
    let five = info.five_hour.as_ref().unwrap();
    assert_eq!(five.state, "tracking");
    assert!(
        five.percentage.is_some_and(|p| (p - 20.0).abs() < 1e-9),
        "{five:?}"
    );
    assert_eq!(five.badge.as_deref(), Some("measured"));
    assert_eq!(five.reset_basis.as_deref(), Some("measured"));
    let week = info.weekly.as_ref().unwrap();
    assert!(
        week.percentage.is_some_and(|p| (p - 40.0).abs() < 1e-9),
        "{week:?}"
    );
    // Its "as of" is the reading's own time (10:30).
    assert_eq!(
        five.measured_at_ms,
        juggler_adapters::quota_ingest::iso_to_epoch_ms("2026-10-21T10:30:00.000Z")
    );
    let credits = info.credits.as_ref().unwrap();
    assert_eq!(credits.balance.as_deref(), Some("16.25"));
    assert!(credits.has_credits && !credits.unlimited);
    assert_eq!(credits.source, "Codex", "labelled with who reported it");
}

fn tokens_of(events: &[juggler_core::quota::UsageEvent]) -> (u64, u64, u64, u64) {
    events.iter().fold((0, 0, 0, 0), |(i, o, r, w), event| {
        (
            i + event.tokens.input,
            o + event.tokens.output,
            r + event.tokens.cache_read,
            w + event.tokens.cache_write,
        )
    })
}

/// P3 on an existing install: rows stored before 0.0.13 counted each
/// content block of a response (keyed per line), and the files were
/// already read to their end. One pass reads every file still on disk
/// again and replaces its sessions' rows; a session whose file is gone
/// keeps its rows; the re-read happens once.
#[test]
fn rows_counted_per_content_block_are_read_again_once() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_claude_account(&state, dir.path());
    let transcript = dir.path().join("managed-claude/projects/t.jsonl");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../juggler-adapters/tests/fixtures/quota/claude-transcript-content-blocks.jsonl",
        ),
        &transcript,
    )
    .unwrap();
    // What 0.0.12 left behind: one row per line, keyed by the line's
    // uuid, and the file's offset at its end.
    let old_row = |uuid: &str, session: &str, output: u64| juggler_core::quota::UsageEvent {
        account: "acc-pipe".into(),
        session: Some(session.into()),
        ts: 1_760_950_801_000,
        model: Some("claude-opus-4".into()),
        tokens: juggler_core::model::TokenCounts {
            input: 4,
            output,
            cache_read: 12_000,
            cache_write: 800,
            reasoning: 0,
        },
        credits: None,
        usd_est: None,
        source: juggler_core::quota::UsageSource::Transcript,
        confidence: juggler_core::model::Confidence::Measured,
        msg_uuid: Some(uuid.into()),
    };
    {
        let storage = state.storage.lock().unwrap();
        storage
            .usage()
            .insert_events(&[
                old_row("b-1", "sess-b", 310),
                old_row("b-2", "sess-b", 310),
                old_row("b-3", "sess-b", 310),
                old_row("b-4", "sess-b", 40),
                old_row("b-5", "sess-b", 52),
                old_row("g-1", "sess-gone", 7),
            ])
            .unwrap();
        let len = std::fs::metadata(&transcript).unwrap().len();
        let spool = dir
            .path()
            .join("managed-claude/quota-spool/statusline.jsonl");
        let spool_len = std::fs::metadata(&spool).unwrap().len();
        storage
            .app_settings()
            .set(
                "usage_ingest_state",
                &serde_json::json!({"offsets": {"acc-pipe": {
                    "spool": spool_len,
                    "files": {transcript.to_string_lossy(): len},
                }}})
                .to_string(),
            )
            .unwrap();
    }

    usage_pipeline::ingest_pass(&state, 4);
    let read = |session: &str| {
        let storage = state.storage.lock().unwrap();
        storage
            .usage()
            .events_for_account("acc-pipe", 0)
            .unwrap()
            .into_iter()
            .filter(|event| event.session.as_deref() == Some(session))
            .collect::<Vec<_>>()
    };
    let session = read("sess-b");
    assert_eq!(session.len(), 2, "one row per response");
    assert_eq!(tokens_of(&session), (10, 362, 24_800, 890));
    assert_eq!(read("sess-gone").len(), 1, "a gone file's rows stay");

    // Once: the next pass leaves the rows as they are.
    usage_pipeline::ingest_pass(&state, 8);
    assert_eq!(tokens_of(&read("sess-b")), (10, 362, 24_800, 890));
    assert_eq!(read("sess-gone").len(), 1);
}

/// P15(a): a Codex rollout's usage lines carry no model; the turn's
/// `turn_context` does. A slice that starts mid-turn keeps the model the
/// file's previous slice ended with.
#[test]
fn codex_usage_keeps_its_turns_model_across_passes() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let home = dir.path().join("codex-home");
    let day = home.join("sessions/2026/10/21");
    std::fs::create_dir_all(&day).unwrap();
    let rollout = day.join("rollout-2026-10-21T10-00-00-th-7.jsonl");
    let record = |response: &str| {
        format!(
            "{{\"timestamp\":\"2026-10-21T10:00:0{response}.000Z\",\"type\":\"token_usage_record\",\"payload\":{{\"thread_id\":\"th-7\",\"turn_id\":\"tu-1\",\"response_id\":\"r{response}\",\"usage\":{{\"input_tokens\":100,\"cached_input_tokens\":60,\"output_tokens\":20,\"reasoning_output_tokens\":5,\"total_tokens\":120}}}}}}\n"
        )
    };
    std::fs::write(
        &rollout,
        format!(
            "{}{}",
            "{\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"type\":\"turn_context\",\"payload\":{\"turn_id\":\"tu-1\",\"model\":\"gpt-5-codex\"}}\n",
            record("1")
        ),
    )
    .unwrap();
    let account = Account {
        id: AccountId::new("acc-codex"),
        harness: HarnessId::new("codex"),
        label: "codex".into(),
        plan: None,
        state_dir: home,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&account)
        .unwrap();

    usage_pipeline::ingest_pass(&state, 4);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&rollout)
        .and_then(|mut file| std::io::Write::write_all(&mut file, record("2").as_bytes()))
        .unwrap();
    usage_pipeline::ingest_pass(&state, 8);

    let events = state
        .storage
        .lock()
        .unwrap()
        .usage()
        .events_for_account("acc-codex", 0)
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|event| event.model.as_deref() == Some("gpt-5-codex")),
        "every row carries the turn's model"
    );
    // P14: each token once — the classes sum to Codex's total_tokens.
    for event in &events {
        let t = &event.tokens;
        assert_eq!(t.input + t.cache_read + t.output + t.reasoning, 120);
    }
}

/// #126 follow-up: the pipeline forgets the offsets and carries of files
/// that are gone, so its state doesn't grow with every rollout there ever
/// was; a sessions folder that is missing for a moment forgets nothing.
#[test]
fn a_deleted_rollout_is_forgotten_and_a_missing_folder_forgets_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let home = dir.path().join("codex-home");
    let day = home.join("sessions/2026/10/21");
    std::fs::create_dir_all(&day).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../juggler-adapters/tests/fixtures/quota/codex-rollout-rate-limits.jsonl");
    let kept = day.join("rollout-2026-10-21T10-00-00-th-1.jsonl");
    let gone = day.join("rollout-2026-10-21T11-00-00-th-2.jsonl");
    std::fs::copy(&fixture, &kept).unwrap();
    std::fs::copy(&fixture, &gone).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-codex"),
            harness: HarnessId::new("codex"),
            label: "codex".into(),
            plan: None,
            state_dir: home.clone(),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    let tracked = || {
        let blob = state
            .storage
            .lock()
            .unwrap()
            .app_settings()
            .get("usage_ingest_state")
            .unwrap()
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&blob).unwrap();
        let mut files: Vec<String> = value["offsets"]["acc-codex"]["files"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        files.sort();
        files
    };
    usage_pipeline::ingest_pass(&state, 4);
    assert_eq!(tracked().len(), 2);
    // Codex keys name their own thread: its rows keep no holders.
    let holders: i64 = rusqlite::Connection::open(dir.path().join("app-data/juggler.sqlite"))
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM usage_holders WHERE account = 'acc-codex'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(holders, 0);
    assert!(
        !state
            .storage
            .lock()
            .unwrap()
            .usage()
            .derived_dirty()
            .unwrap()
    );
    std::fs::remove_file(&gone).unwrap();
    usage_pipeline::ingest_pass(&state, 8);
    assert_eq!(tracked(), [kept.to_string_lossy().to_string()]);
    // The whole folder away for a moment: nothing is forgotten.
    let aside = dir.path().join("sessions-aside");
    std::fs::rename(home.join("sessions"), &aside).unwrap();
    usage_pipeline::ingest_pass(&state, 12);
    assert_eq!(tracked(), [kept.to_string_lossy().to_string()]);
}

/// L-c (#126 re-review): after going back to a build without the holders
/// and forward again, the next pass reads everything again, so the holders
/// are whole, and clears the mark.
#[test]
fn a_pass_after_an_older_build_wrote_reads_everything_again() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_claude_account(&state, dir.path());
    usage_pipeline::ingest_pass(&state, 4);
    let db = dir.path().join("app-data/juggler.sqlite");
    let holders = || {
        rusqlite::Connection::open(&db)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM usage_holders", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap()
    };
    assert!(holders() > 0);
    // The older build: it never kept holders, and it stores a message.
    {
        let older = rusqlite::Connection::open(&db).unwrap();
        older.execute("DELETE FROM usage_holders", []).unwrap();
        older
            .execute(
                "INSERT INTO usage_events (account, session, ts, output, source, confidence, msg_uuid)
                 VALUES ('acc-pipe', 's', 5, 9, 'transcript', 'measured', 'older-1')",
                [],
            )
            .unwrap();
    }
    assert!(
        state
            .storage
            .lock()
            .unwrap()
            .usage()
            .derived_dirty()
            .unwrap()
    );
    usage_pipeline::ingest_pass(&state, 8);
    assert!(holders() > 0, "the holders are read again");
    assert!(
        !state
            .storage
            .lock()
            .unwrap()
            .usage()
            .derived_dirty()
            .unwrap()
    );
}

/// Fix batch 5: a Codex rollout that Codex compresses after it was read
/// (`rollout-….jsonl` → `rollout-….jsonl.zst`, same thread) counts once:
/// the plain file's state is forgotten and the compressed one adds nothing
/// new. Mid-compression, with both files there, nothing doubles either.
#[test]
fn a_rollout_compressed_after_it_was_read_counts_once() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let home = dir.path().join("codex-home");
    let day = home.join("sessions/2026/10/21");
    std::fs::create_dir_all(&day).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../juggler-adapters/tests/fixtures/codex/rollout-paginated.jsonl");
    let plain = day.join("rollout-2026-10-21T10-00-00-th-1.jsonl");
    std::fs::copy(&fixture, &plain).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-codex"),
            harness: HarnessId::new("codex"),
            label: "codex".into(),
            plan: None,
            state_dir: home.clone(),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    let totals = || {
        let events = state
            .storage
            .lock()
            .unwrap()
            .usage()
            .events_for_account("acc-codex", 0)
            .unwrap();
        let tokens: u64 = events
            .iter()
            .map(|e| e.tokens.input + e.tokens.output + e.tokens.cache_read + e.tokens.reasoning)
            .sum();
        (events.len(), tokens)
    };
    usage_pipeline::ingest_pass(&state, 4);
    let read_plain = totals();
    assert!(read_plain.0 > 0, "the plain rollout's usage");
    // Codex compresses it: for a moment both are there, then the plain goes.
    let compressed = day.join("rollout-2026-10-21T10-00-00-th-1.jsonl.zst");
    std::fs::write(
        &compressed,
        zstd::encode_all(&std::fs::read(&plain).unwrap()[..], 3).unwrap(),
    )
    .unwrap();
    usage_pipeline::ingest_pass(&state, 8);
    assert_eq!(totals(), read_plain, "both files there: counted once");
    std::fs::remove_file(&plain).unwrap();
    usage_pipeline::ingest_pass(&state, 12);
    assert_eq!(
        totals(),
        read_plain,
        "only the compressed one: counted once"
    );
}

/// A Codex home whose history is mostly compressed: its usage is read.
#[test]
fn a_compressed_codex_history_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let home = dir.path().join("codex-home");
    let day = home.join("sessions/2026/09/01");
    std::fs::create_dir_all(&day).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../juggler-adapters/tests/fixtures/codex/rollout-paginated.jsonl");
    std::fs::write(
        day.join("rollout-2026-09-01T10-00-00-th-old.jsonl.zst"),
        zstd::encode_all(&std::fs::read(&fixture).unwrap()[..], 3).unwrap(),
    )
    .unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-codex"),
            harness: HarnessId::new("codex"),
            label: "codex".into(),
            plan: None,
            state_dir: home,
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    usage_pipeline::ingest_pass(&state, 4);
    let events = state
        .storage
        .lock()
        .unwrap()
        .usage()
        .events_for_account("acc-codex", 0)
        .unwrap();
    assert!(
        !events.is_empty(),
        "the compressed rollout's usage is counted"
    );
}

/// #131 review: credits never go back in time. A plain rollout reports a
/// balance today; an old compressed rollout read afterwards reports the
/// balance of weeks ago. The card keeps today's.
#[test]
fn an_old_compressed_rollout_never_moves_the_credits_back() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let home = dir.path().join("codex-home");
    let credits_line = |ts: &str, balance: &str| {
        format!(
            "{{\"timestamp\":\"{ts}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":null,\"rate_limits\":{{\"limit_id\":\"l\",\"primary\":null,\"secondary\":null,\"credits\":{{\"has_credits\":true,\"unlimited\":false,\"balance\":\"{balance}\"}}}}}}}}\n"
        )
    };
    let today = home.join("sessions/2026/10/21");
    std::fs::create_dir_all(&today).unwrap();
    std::fs::write(
        today.join("rollout-2026-10-21T10-00-00-th-new.jsonl"),
        credits_line("2026-10-21T10:00:00.000Z", "16.25"),
    )
    .unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-codex"),
            harness: HarnessId::new("codex"),
            label: "codex".into(),
            plan: None,
            state_dir: home.clone(),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    let balance = || {
        let blob = state
            .storage
            .lock()
            .unwrap()
            .app_settings()
            .get(&format!("{}acc-codex", usage_pipeline::CREDITS_KEY_PREFIX))
            .unwrap()
            .unwrap();
        serde_json::from_str::<serde_json::Value>(&blob).unwrap()["balance"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    usage_pipeline::ingest_pass(&state, 4);
    assert_eq!(balance(), "16.25");
    // Codex compressed an old one; it is read now, whole.
    let old = home.join("sessions/2026/09/01");
    std::fs::create_dir_all(&old).unwrap();
    let zst = old.join("rollout-2026-09-01T10-00-00-th-old.jsonl.zst");
    std::fs::write(
        &zst,
        zstd::encode_all(
            credits_line("2026-09-01T10:00:00.000Z", "40.00").as_bytes(),
            3,
        )
        .unwrap(),
    )
    .unwrap();
    usage_pipeline::ingest_pass(&state, 8);
    assert_eq!(balance(), "16.25", "the newest balance stays");
}

/// P21: an older build stored a second row for a compaction that names
/// another thread's model call (`…/compacted/…`, that call counted
/// again). The next pass reads the threads again and drops it; the call
/// counts once, under the thread that made it.
#[test]
fn a_compaction_counted_twice_by_an_older_build_is_read_again() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let home = dir.path().join("codex-home");
    let day = home.join("sessions/2026/10/21");
    std::fs::create_dir_all(&day).unwrap();
    let usage = |input: u64| {
        format!(
            r#"{{"input_tokens":{input},"cached_input_tokens":0,"cache_write_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0,"total_tokens":{input}}}"#
        )
    };
    let line = |ts: &str, kind: &str, payload: String| {
        format!(
            r#"{{"timestamp":"2026-10-21T10:00:{ts}.000Z","type":"{kind}","payload":{payload}}}"#
        ) + "\n"
    };
    let record = |ts: &str, thread: &str, turn: &str, response: &str, input: u64| {
        line(
            ts,
            "token_usage_record",
            format!(
                r#"{{"thread_id":"{thread}","turn_id":"{turn}","response_id":"{response}","usage":{}}}"#,
                usage(input)
            ),
        )
    };
    let count = |ts: &str, last: u64, total: u64| {
        line(
            ts,
            "event_msg",
            format!(
                r#"{{"type":"token_count","info":{{"last_token_usage":{},"total_token_usage":{}}},"rate_limits":null}}"#,
                usage(last),
                usage(total)
            ),
        )
    };
    let parent = [
        record("01", "th-parent", "tp", "r-compact", 136_658),
        count("02", 136_658, 1_899_060),
    ]
    .concat();
    let child = [
        line(
            "03",
            "compacted",
            format!(
                r#"{{"compaction_response_id":null,"latest_token_usage_record":{{"thread_id":"th-parent","turn_id":"tp","response_id":"r-compact","root_turn_id":"tp","session_id":"th-parent","usage":{u},"turn_token_usage":{u},"thread_token_usage":{u}}},"window_number":1}}"#,
                u = usage(136_658)
            ),
        ),
        count("04", 136_658, 1_899_060),
        line("05", "event_msg", r#"{"type":"task_started"}"#.to_owned()),
        record("06", "th-sub", "t1", "r-t1", 100),
    ]
    .concat();
    let parent_path = day.join("rollout-2026-10-21T10-00-00-th-parent.jsonl");
    let child_path = day.join("rollout-2026-10-21T10-00-03-th-sub.jsonl");
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-codex"),
                harness: HarnessId::new("codex"),
                label: "codex".into(),
                plan: None,
                state_dir: home,
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
                in_place: false,
            })
            .unwrap();
    }
    // A first pass with nothing to read settles the new account (its own
    // reason to read everything again).
    usage_pipeline::ingest_pass(&state, 0);
    std::fs::write(&parent_path, &parent).unwrap();
    std::fs::write(&child_path, &child).unwrap();
    {
        let storage = state.storage.lock().unwrap();
        // What the older build left: both calls, the same call again under
        // the child's compaction, and both files read to their end.
        let row = |session: &str, key: &str, input: u64| juggler_core::quota::UsageEvent {
            account: "acc-codex".into(),
            session: Some(session.into()),
            ts: 1_792_576_801_000,
            model: None,
            tokens: juggler_core::model::TokenCounts {
                input,
                ..Default::default()
            },
            credits: None,
            usd_est: None,
            source: juggler_core::quota::UsageSource::Rollout,
            confidence: juggler_core::model::Confidence::Measured,
            msg_uuid: Some(key.into()),
        };
        storage
            .usage()
            .store_events(&[
                row("th-parent", "th-parent/tp/r-compact", 136_658),
                row("th-sub", "th-sub/compacted/1792576804000/1", 136_658),
                row("th-sub", "th-sub/t1/r-t1", 100),
            ])
            .unwrap();
        // Only the version asks for the re-read here.
        assert!(!storage.usage().derived_dirty().unwrap());
        storage
            .app_settings()
            .set(
                "usage_ingest_state",
                &serde_json::json!({
                    "reread": 4,
                    "offsets": {"acc-codex": {"spool": 0, "files": {
                        parent_path.to_string_lossy(): parent.len(),
                        child_path.to_string_lossy(): child.len(),
                    }}},
                })
                .to_string(),
            )
            .unwrap();
    }

    usage_pipeline::ingest_pass(&state, 4);
    let stored: std::collections::BTreeMap<String, u64> = state
        .storage
        .lock()
        .unwrap()
        .usage()
        .events_for_account("acc-codex", 0)
        .unwrap()
        .into_iter()
        .map(|event| (event.msg_uuid.unwrap(), event.tokens.input))
        .collect();
    assert_eq!(
        stored,
        std::collections::BTreeMap::from([
            ("th-parent/tp/r-compact".to_owned(), 136_658),
            ("th-sub/t1/r-t1".to_owned(), 100),
        ])
    );
}
