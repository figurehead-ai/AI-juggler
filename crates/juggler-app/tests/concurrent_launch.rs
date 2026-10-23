//! AJ-E306 / EPICS M1-3, by AJ-E1522: flow (e) end to end on the fake
//! harness — the launch sheet's rows within budget, their headroom from
//! the cache, then two Instances on two Accounts launched at once, each in
//! its own folder, both running together, both ending cleanly.
//!
//! The budgets (AJ-E306): the sheet's data ≤ 200 ms, headroom from the
//! cache ≤ 50 ms, no live quota fetch on the launch path. They're this
//! thread's CPU time, with wall time printed beside it (the work runs on
//! this thread; another program's turns on a busy machine don't count).
//! The fixture HOME is a temp folder: nothing of the user's is read.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod common;

use std::path::Path;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::commands::{self, ranking_rows_inner};
use juggler_app::features::quota::quota_display_inner;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, LaunchRecipe, MachineId, ProfileId, ProjectId,
    SessionId, SessionRecord, SessionStatus, SettingsOverlay, Timestamp,
};

const PROJECT: &str = "proj-x";
/// Each of the two fake turns waits this long: the session lives ~4 s,
/// so both are certainly running when both launches have returned.
const TURN_DELAY_MS: &str = "2000";

/// Two accounts on the fake harness, each with its launch settings and a
/// known headroom: the tool's own reading of the window (a measured
/// sample, as the statusline ingest stores it), a cap, and a session on
/// the project that burned some.
fn fixture(home: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::FakeHarnessAdapter::new(
        common::fake_harness::bin(),
    )));
    let state = Arc::new(AppState::init_with_registry(home.join("app-data"), registry).unwrap());
    let storage = state.storage.lock().unwrap();
    for (n, label) in [(1, "Work"), (2, "Personal")] {
        let account = AccountId::new(format!("acc-{n}"));
        storage
            .accounts()
            .insert(&Account {
                id: account.clone(),
                harness: HarnessId::new("fake"),
                label: label.into(),
                plan: None,
                state_dir: home.join(format!("fake-{n}")),
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
                id: ProfileId::new(format!("rec-{n}")),
                name: "Main".into(),
                account: account.clone(),
                settings_overlay: SettingsOverlay {
                    model: None,
                    env: [("FAKE_HARNESS_DELAY_MS".to_owned(), TURN_DELAY_MS.to_owned())]
                        .into_iter()
                        .collect(),
                    restart_on_crash: None,
                },
                default_cwd: None,
            })
            .unwrap();
        // The burn the admission verdict's median is taken over.
        storage
            .sessions()
            .insert(&SessionRecord {
                id: SessionId::new(format!("earlier-{n}")),
                conversation: None,
                profile: ProfileId::new(format!("rec-{n}")),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: Some(ProjectId::new(PROJECT)),
                cwd: home.to_path_buf(),
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
        storage
            .app_settings()
            .set(
                &juggler_core::storage::cap_settings_key(
                    account.as_str(),
                    juggler_core::quota::WindowKind::FiveHour,
                ),
                "tokens=10000",
            )
            .unwrap();
        storage
            .usage()
            .insert_events(&[juggler_core::quota::UsageEvent {
                account: account.as_str().into(),
                session: Some(format!("earlier-{n}")),
                ts: Timestamp::now().millis() - 1_000,
                model: None,
                tokens: juggler_core::model::TokenCounts {
                    output: 2_000,
                    ..Default::default()
                },
                credits: None,
                usd_est: None,
                confidence: juggler_core::model::Confidence::Measured,
                msg_uuid: None,
                source: juggler_core::quota::UsageSource::Transcript,
            }])
            .unwrap();
        storage
            .usage()
            .insert_samples(&[(
                account.as_str().to_owned(),
                juggler_core::quota::WindowKind::FiveHour,
                juggler_core::quota::QuotaSample {
                    ts: Timestamp::now().millis() - 2_000,
                    used_pct: 20.0 + f64::from(n),
                    resets_at: Some(Timestamp::now().millis() + 3_600_000),
                    confidence: juggler_core::model::Confidence::Measured,
                },
            )])
            .unwrap();
    }
    drop(storage);
    state
}

/// This thread's CPU time and the wall time of `work`.
fn timed<T>(work: impl FnOnce() -> T) -> (T, Duration, Duration) {
    let wall = Instant::now();
    let cpu = cpu_time::ThreadTime::now();
    let out = work();
    (out, cpu.elapsed(), wall.elapsed())
}

/// What's stored for quota: the usage rows and the tool's readings.
fn quota_rows(state: &Arc<AppState>) -> (usize, usize) {
    let storage = state.storage.lock().unwrap();
    let usage = storage.usage();
    ["acc-1", "acc-2"]
        .iter()
        .fold((0, 0), |(events, samples), account| {
            (
                events + usage.events_for_account(account, 0).unwrap().len(),
                samples
                    + usage
                        .samples_for_account(account, juggler_core::quota::WindowKind::FiveHour)
                        .unwrap()
                        .len(),
            )
        })
}

#[test]
fn flow_e_launches_two_instances_on_two_accounts_at_once_within_its_budgets() {
    let home = tempfile::tempdir().unwrap();
    let state = fixture(home.path());
    let quota_before = quota_rows(&state);

    // The sheet's rows: both accounts, headroom known, a verdict each.
    let (rows, cpu, wall) = timed(|| ranking_rows_inner(&state, Some(PROJECT.to_owned())).unwrap());
    eprintln!("[budget] the sheet's rows: {cpu:?} CPU, {wall:?} wall");
    assert!(
        cpu < Duration::from_millis(200),
        "rows {cpu:?} CPU (wall {wall:?})"
    );
    for account in ["acc-1", "acc-2"] {
        let row = rows.iter().find(|row| row.account == account).unwrap();
        assert!(!row.headroom_unknown, "{account}: headroom known");
        assert_eq!(row.admission.as_deref(), Some("ok"), "{account}");
    }

    // Each account's headroom, from what's stored.
    let accounts: Vec<Account> = state.storage.lock().unwrap().accounts().list().unwrap();
    let (displays, cpu, wall) = timed(|| {
        let storage = state.storage.lock().unwrap();
        let now = Timestamp::now().millis();
        accounts
            .iter()
            .map(|account| quota_display_inner(&storage, account, now))
            .collect::<Vec<_>>()
    });
    eprintln!("[budget] headroom from the cache: {cpu:?} CPU, {wall:?} wall");
    assert!(
        cpu < Duration::from_millis(50),
        "headroom {cpu:?} CPU (wall {wall:?})"
    );
    assert_eq!(displays.len(), 2);
    for display in &displays {
        let five = display.five_hour.as_ref().expect("a five-hour window");
        assert!(five.percentage.is_some(), "the stored reading, not a guess");
    }

    // Two launches at the same moment, one per account.
    let barrier = Arc::new(Barrier::new(2));
    let cwd = home.path().join("work");
    std::fs::create_dir_all(&cwd).unwrap();
    let launches: Vec<_> = ["rec-1", "rec-2"]
        .into_iter()
        .map(|recipe| {
            let (state, barrier, cwd) = (Arc::clone(&state), Arc::clone(&barrier), cwd.clone());
            std::thread::spawn(move || {
                barrier.wait();
                commands::launch_profile(&state, recipe, Some(cwd.display().to_string()))
            })
        })
        .collect();
    let results: Vec<_> = launches
        .into_iter()
        .map(|launch| launch.join().unwrap().unwrap())
        .collect();
    let sessions: Vec<SessionId> = results
        .iter()
        .map(|result| SessionId::new(result.session_id.clone()))
        .collect();
    assert_ne!(sessions[0], sessions[1]);

    // Both running together.
    for session in &sessions {
        assert!(
            !state.supervisor.is_finished(session),
            "{session:?} ended before the other launched"
        );
    }

    // Each wrote into its own account's folder, and only there.
    for n in [1, 2] {
        let transcripts = home.path().join(format!("fake-{n}")).join("transcripts");
        let deadline = Instant::now() + Duration::from_secs(15);
        while std::fs::read_dir(&transcripts).map_or(0, Iterator::count) == 0 {
            assert!(
                Instant::now() < deadline,
                "fake-{n} never got its transcript"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(
            std::fs::read_dir(&transcripts).unwrap().count(),
            1,
            "fake-{n} has exactly its own session's transcript"
        );
    }

    // Both end cleanly.
    let deadline = Instant::now() + Duration::from_secs(30);
    while sessions
        .iter()
        .any(|session| !state.supervisor.is_finished(session))
    {
        assert!(Instant::now() < deadline, "a session never ended");
        std::thread::sleep(Duration::from_millis(100));
    }

    // No live quota fetch on the launch path: nothing was fetched or
    // recorded for quota; the readings are the ones stored before.
    assert_eq!(quota_rows(&state), quota_before);
}
