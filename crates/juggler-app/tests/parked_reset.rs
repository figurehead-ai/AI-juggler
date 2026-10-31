//! AJ-E1003 integration: park a quota-blocked Instance until its window
//! resets (stub `claude`). Proves: only a quota-blocked Instance parks,
//! its pack is staged and the Measured reset recorded; nothing resumes on
//! its own; at the reset exactly ONE offer is raised (logged once) and it
//! persists until acted on; "not now" snoozes once, then the Instance
//! stays parked quietly; resume continues the ORIGINAL native session on
//! the ORIGINAL Account; an Estimated reset carries no countdown.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::state::AppState;
use juggler_app::{commands, parked};
use juggler_core::model::{SessionId, SessionStatus, Timestamp};
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn claude_stub(dir: &Path) -> PathBuf {
    let path = dir.join("claude-stub");
    stand_in::write(
        &path, // Written whole, then renamed into place: the test polls for the
        // file and must never read it half-written.
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/last-args.tmp\" && mv \"$CLAUDE_CONFIG_DIR/last-args.tmp\" \"$CLAUDE_CONFIG_DIR/last-args\"\n",
    );
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

/// A quota-blocked Instance with a 500-turn transcript and a Measured
/// exhausted 5 h window. Returns (state dir guard, state, session, account).
fn blocked(work: &Path, measured: bool) -> (Arc<AppState>, String, String, PathBuf) {
    let state = app_state(work);
    commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();
    let project = work.join("storefront");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let account =
        commands::create_account(&state, CLAUDE_CODE_ID, "Work Max", Some("max".into())).unwrap();
    let recipe = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &recipe.id, Some(cwd.clone())).unwrap();
    let config = state.accounts_dir().join(CLAUDE_CODE_ID).join(&account.id);
    let transcript = config
        .join("projects")
        .join(juggler_adapter::handover::mangled_dir(&cwd))
        .join("native-7.jsonl");
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    let mut body = String::new();
    for n in 0..500 {
        let role = if n % 2 == 0 { "user" } else { "assistant" };
        body.push_str(&format!(
            "{{\"type\":\"{role}\",\"uuid\":\"u{n}\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"{role}\",\"content\":\"turn {n}\"}}}}\n"
        ));
    }
    std::fs::write(&transcript, body).unwrap();
    let now = Timestamp::now().millis();
    {
        let storage = state.storage.lock().unwrap();
        let mut row = storage
            .sessions()
            .get(&SessionId::new(launched.session_id.clone()))
            .unwrap();
        row.status = SessionStatus::QuotaBlocked;
        row.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: transcript,
            native_id: Some("native-7".into()),
        });
        storage.sessions().update(&row).unwrap();
        let events: Vec<juggler_core::quota::UsageEvent> = (0..3)
            .map(|back| juggler_core::quota::UsageEvent {
                account: account.id.clone(),
                session: None,
                ts: now - i64::from(back) * 60_000,
                model: Some("claude-opus-4".into()),
                tokens: juggler_core::model::TokenCounts {
                    input: 1_000,
                    ..juggler_core::model::TokenCounts::default()
                },
                credits: None,
                usd_est: None,
                source: juggler_core::quota::UsageSource::Transcript,
                confidence: juggler_core::model::Confidence::Measured,
                msg_uuid: Some(format!("e-{back}")),
            })
            .collect();
        storage.usage().insert_events(&events).unwrap();
        if measured {
            storage
                .usage()
                .insert_sample(
                    &account.id,
                    juggler_core::quota::WindowKind::FiveHour,
                    &juggler_core::quota::QuotaSample {
                        ts: now - 60_000,
                        used_pct: 100.0,
                        resets_at: Some(now + 2 * 3_600_000),
                        confidence: juggler_core::model::Confidence::Measured,
                    },
                )
                .unwrap();
        }
    }
    (state, launched.session_id, account.id, config)
}

fn offer_log(state: &Arc<AppState>) -> Vec<String> {
    std::fs::read_to_string(state.data_dir.join("parked").join("offers.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_parked_instance_is_offered_back_once_at_the_reset_and_resumes_the_original() {
    let work = tempfile::tempdir().unwrap();
    let (state, session, _account, config) = blocked(work.path(), true);
    let started = Instant::now();
    let info = parked::instance_park_impl(&state, &session).unwrap();
    let took = started.elapsed();
    eprintln!("[perf] park incl. pack staging (500 turns): {took:?}");
    assert!(took < Duration::from_secs(2), "{took:?}");
    assert_eq!(info.confidence, "measured");
    assert!(
        info.resets_at_ms
            .is_some_and(|at| at > Timestamp::now().millis())
    );
    assert!(!info.due);
    assert!(
        Path::new(&info.pack_dir).join("PACK.md").is_file(),
        "pack staged"
    );

    // Before the reset: nothing offered, nothing launched.
    assert!(parked::parked_offers_impl(&state).unwrap().is_empty());
    // The setup launch's stub records its argv asynchronously: wait for
    // it, then clear it — anything after this is a new launch.
    let deadline = Instant::now() + Duration::from_secs(10);
    while !config.join("last-args").is_file() {
        assert!(Instant::now() < deadline, "setup stub never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(200));
    let _ = std::fs::remove_file(config.join("last-args"));

    // The window resets.
    parked::parked_set_reset_for_test(&state, &session, Timestamp::now().millis() - 1);
    let offers = parked::parked_offers_impl(&state).unwrap();
    assert_eq!(offers.len(), 1);
    assert!(
        offers[0].text.contains("Work Max has reset"),
        "{}",
        offers[0].text
    );
    // Polled again: the offer persists, but it was raised ONCE.
    assert_eq!(parked::parked_offers_impl(&state).unwrap().len(), 1);
    assert_eq!(parked::parked_offers_impl(&state).unwrap().len(), 1);
    assert_eq!(offer_log(&state).len(), 1, "exactly one notification");
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !config.join("last-args").exists(),
        "nothing resumed on its own"
    );

    // The user's click: the ORIGINAL native session on the same Account.
    let resumed = parked::parked_resume_impl(&state, &session).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !config.join("last-args").is_file() {
        assert!(Instant::now() < deadline, "the resume never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    let args = std::fs::read_to_string(config.join("last-args")).unwrap();
    assert!(
        args.contains("--resume") && args.contains("native-7"),
        "{args}"
    );
    assert!(
        parked::parked_list_impl(&state).is_empty(),
        "no longer parked"
    );
    assert_ne!(resumed.session_id, session);
}

#[test]
fn not_now_snoozes_once_then_stays_parked_quietly() {
    let work = tempfile::tempdir().unwrap();
    let (state, session, _, _) = blocked(work.path(), true);
    parked::instance_park_impl(&state, &session).unwrap();
    parked::parked_set_reset_for_test(&state, &session, Timestamp::now().millis() - 1);
    assert_eq!(parked::parked_offers_impl(&state).unwrap().len(), 1);
    parked::parked_dismiss_impl(&state, &session).unwrap();
    assert!(
        parked::parked_offers_impl(&state).unwrap().is_empty(),
        "snoozed"
    );
    parked::parked_end_snooze_for_test(&state, &session);
    let again = parked::parked_offers_impl(&state).unwrap();
    assert_eq!(again.len(), 1);
    assert!(again[0].last, "the snoozed offer is the last one");
    parked::parked_dismiss_impl(&state, &session).unwrap();
    assert!(parked::parked_offers_impl(&state).unwrap().is_empty());
    parked::parked_end_snooze_for_test(&state, &session);
    assert!(
        parked::parked_offers_impl(&state).unwrap().is_empty(),
        "no nagging"
    );
    assert_eq!(offer_log(&state).len(), 2);
    assert_eq!(parked::parked_list_impl(&state).len(), 1, "still parked");
}

#[test]
fn only_a_quota_blocked_instance_parks_and_an_estimated_reset_has_no_countdown() {
    let work = tempfile::tempdir().unwrap();
    let (state, session, _, _) = blocked(work.path(), false);
    let info = parked::instance_park_impl(&state, &session).unwrap();
    assert_ne!(info.confidence, "measured", "{info:?}");
    // Check now re-reads the engine (still not measured here).
    let checked = parked::parked_check_now_impl(&state, &session).unwrap();
    assert_ne!(checked.confidence, "measured");
    // A running Instance can't be parked.
    {
        let storage = state.storage.lock().unwrap();
        let mut row = storage
            .sessions()
            .get(&SessionId::new(session.clone()))
            .unwrap();
        row.status = SessionStatus::Running;
        storage.sessions().update(&row).unwrap();
    }
    assert!(parked::instance_park_impl(&state, &session).is_err());
}
