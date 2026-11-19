//! The watches on today's event stream, end to end: a session's status
//! published on the notification path reaches the engine on its thread,
//! fires, is logged, and waits as a typed alert for AJ-E3817; the reads
//! over IPC say it in plain words.

use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_core::model::SessionStatus;
use juggler_core::storage::memory::Writers;

use super::*;
use crate::features::ai::memory::RedactionSecrets;
use crate::features::watches::commands;
use crate::features::watches::runtime::WatchAlert;
use crate::features::watches::store::{Origin, WatchStore};
use crate::state::AppState;

/// A state with the fake terminal tool, an account, launch settings and
/// one running session `s1`, and the memory store's tables.
fn lab(dir: &std::path::Path) -> Arc<AppState> {
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::fake::FakeHarnessAdapter::new(
        "/bin/true",
    )));
    let state = Arc::new(AppState::init_with_registry(dir.to_path_buf(), registry).unwrap());
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&crate::test_support::account(
                "acct-1",
                juggler_adapters::fake::FAKE_HARNESS_ID,
                "Lab",
                dir.join("acct"),
            ))
            .unwrap();
        storage
            .profiles()
            .insert(&crate::test_support::launch_settings(
                "p1",
                "Lab terminal",
                "acct-1",
            ))
            .unwrap();
        storage
            .sessions()
            .insert(&crate::test_support::instance(
                "s1",
                "p1",
                SessionStatus::Running,
                dir.to_path_buf(),
            ))
            .unwrap();
    }
    state
}

fn wait_for_alerts(state: &AppState, n: usize) -> Vec<WatchAlert> {
    let started = Instant::now();
    let mut alerts = Vec::new();
    while alerts.len() < n && started.elapsed() < Duration::from_secs(10) {
        alerts.extend(state.watches.drain_alerts());
        std::thread::sleep(Duration::from_millis(10));
    }
    alerts
}

#[test]
fn a_session_status_fires_a_watch_on_its_thread() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let writers = Writers::for_tests();
    let now = crate::commands::now_millis();
    {
        let storage = state.storage.lock().unwrap();
        let registry = Registry::shipped();
        let store = WatchStore::new(&storage, &registry);
        let mut w = watch(
            "w-11",
            vec![session("s1")],
            vec![cond("fails")],
            Lifetime::Once,
        );
        w.created_ms = now;
        w.even_in_quiet_hours = true;
        store
            .create(
                &writers.watches,
                &RedactionSecrets,
                &w,
                &Origin::WatchesView,
                now,
            )
            .unwrap();
    }
    crate::features::notifications::session_status(&state, "s1", "failed");
    let alerts = wait_for_alerts(&state, 1);
    assert_eq!(alerts.len(), 1, "the fire reached the alert path");
    let fire = &alerts[0].fire;
    assert_eq!(
        (fire.condition.as_str(), fire.session.as_deref()),
        ("fails", Some("s1"))
    );
    assert!(
        fire.even_in_quiet_hours,
        "the quiet switch is carried to the alert path"
    );
    let listed =
        commands::watches_list_impl(&state, &crate::paging::PageRequest::first(50), Lang::En)
            .unwrap()
            .items;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].told, 1);
    assert_eq!(listed[0].state, commands::WatchStateInfo::Ended);
    assert_eq!(listed[0].ended, Some(EndReason::FirstFire));
    assert_eq!(
        listed[0].ended_words.as_deref(),
        Some("Ended: it told you once, as asked.")
    );
    assert_eq!(state.watches.hearing(), 0, "a once watch is heard no more");
}

#[test]
fn the_reads_offer_what_the_lab_can_report_and_say_a_draft() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    // The fake terminal tool reports no account room: "runs below" isn't offered.
    let offered: Vec<String> =
        commands::watches_catalogue_impl(&state, &WatchTarget::Session("s1".into()), Lang::En)
            .unwrap()
            .into_iter()
            .map(|e| e.code)
            .collect();
    assert!(offered.contains(&"stuck".to_owned()) && offered.contains(&"hits_limit".to_owned()));
    assert!(
        !offered
            .iter()
            .any(|c| c == "runway_below" || c == "commits")
    );
    let draft = commands::WatchDraftInfo {
        targets: vec![WatchTarget::Session("s1".into())],
        matching: Matching::Each,
        conditions: vec![cond("finishes")],
        lifetime: LifetimeChoice::Unsaid,
        even_in_quiet_hours: false,
        chat_turn: None,
    };
    let preview = commands::watches_preview_impl(&state, &draft, Lang::En).unwrap();
    assert_eq!(preview.problem, None);
    assert_eq!(
        preview.watch.lifetime,
        Lifetime::Once,
        "finishing on a session: once"
    );
    assert!(
        preview.words.starts_with("Tell you when “"),
        "{}",
        preview.words
    );
    assert!(
        preview.words.ends_with(" finishes. Once."),
        "{}",
        preview.words
    );
    let mut unsupported = draft.clone();
    unsupported.conditions = vec![cond_p("runway_below", Param::Percent(20))];
    unsupported.targets = vec![WatchTarget::Account("acct-1".into())];
    let refused = commands::watches_preview_impl(&state, &unsupported, Lang::En).unwrap();
    assert_eq!(
        refused.problem.as_deref(),
        Some("condition_not_for_target:runway_below:account")
    );
    // A read writes nothing.
    let storage = state.storage.lock().unwrap();
    let rows: i64 = storage
        .raw_conn()
        .query_row("SELECT count(*) FROM memory_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn without_the_memory_store_the_list_says_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(dir.path().to_path_buf());
    // A database without V54's store (one restored from before it).
    state
        .storage
        .lock()
        .unwrap()
        .raw_conn()
        .execute_batch("DROP TABLE memory_records;")
        .unwrap();
    let error =
        commands::watches_list_impl(&state, &crate::paging::PageRequest::first(50), Lang::En)
            .unwrap_err();
    assert!(error.to_string().contains("unavailable"), "{error}");
}
