//! E14 fixture corpus (SPEC §5.3, §12): supervised sessions whose children
//! really allocate memory teach the footprints (AJ-E1406), and the launch
//! advisor's projection for the next session of the same launch setting
//! lands within ±20 % of what that session actually peaks at (AJ-E1407
//! Req.4, SPEC §1).

// Test crates are pragmatic by policy (QUALITY.md §2).
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

use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::state::AppState;
use juggler_app::{commands, machine_health};
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, Timestamp,
};

const MB: u64 = 1_000_000;

fn seeded(dir: &Path) -> Arc<AppState> {
    let state = Arc::new(
        AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-e14"),
            harness: HarnessId::new("claude-code"),
            label: "Work Max".into(),
            plan: None,
            state_dir: dir.join("account"),
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
            id: ProfileId::new("rec-hog"),
            name: "Heavy".into(),
            account: AccountId::new("acc-e14"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    drop(storage);
    state
}

/// A session whose child holds `mb` megabytes, touched so they're resident
/// — the fake Provider's memory hog.
fn launch_hog(state: &Arc<AppState>, dir: &Path, id: &str, mb: u64) -> SessionId {
    let session = SessionId::new(id);
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: ProfileId::new("rec-hog"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            transcript_ref: None,
            attention: None,
            running: None,
            window: None,
        })
        .unwrap();
    let script = format!(
        "python3 -c \"import time; b = b'x' * ({mb} * 1000 * 1000); time.sleep(60)\" & wait"
    );
    state
        .supervisor
        .spawn(
            &session,
            &LaunchSpec {
                program: PathBuf::from("/bin/sh"),
                args: vec!["-c".into(), script],
                env: EnvMap::new(),
                cwd: dir.to_path_buf(),
                needs_pty: false,
            },
            &juggler_supervisor::tree::SpawnContext {
                provider: "claude-code".into(),
                account_id: Some("acc-e14".into()),
                project_id: None,
                machine: commands::local_machine_id(state),
                role: "root",
            },
        )
        .unwrap();
    session
}

/// Ticks until the session's tree has held more than `floor` for a few
/// ticks; returns the peak it reached.
fn run_until_resident(state: &Arc<AppState>, session: &SessionId, floor: u64) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut peak = 0;
    let mut above = 0;
    while Instant::now() < deadline && above < 3 {
        commands::processes_tick(state);
        let rss = state
            .resources
            .lock()
            .unwrap()
            .stats_snapshot()
            .get(session)
            .map_or(0, |stats| stats.rss_bytes);
        peak = peak.max(rss);
        if rss > floor {
            above += 1;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    assert!(peak > floor, "the hog never became resident ({peak} bytes)");
    peak
}

fn end(state: &Arc<AppState>, session: &SessionId) {
    state
        .supervisor
        .stop(session, Duration::from_secs(3))
        .unwrap();
    // The ticks after exit see the tree gone and write its footprint.
    for _ in 0..4 {
        commands::processes_tick(state);
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
fn footprints_teach_the_advisor_within_twenty_percent() {
    let dir = tempfile::tempdir().unwrap();
    let state = seeded(dir.path());

    // Before any history: the default projection, said honestly.
    let before = machine_health::machine_advice_impl(&state).unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].based_on, 0, "not measured yet");

    // Three sessions of the "Heavy" setting, holding 260, 300 and 340 MB —
    // real sessions of one setting vary.
    for (n, mb) in [260_u64, 300, 340].into_iter().enumerate() {
        let session = launch_hog(&state, dir.path(), &format!("hog-{n}"), mb);
        run_until_resident(&state, &session, (mb - 40) * MB);
        end(&state, &session);
    }
    let footprints = state
        .storage
        .lock()
        .unwrap()
        .footprints()
        .of_key("recipe:rec-hog")
        .unwrap();
    assert_eq!(footprints.len(), 3, "one footprint per ended session");
    for footprint in &footprints {
        assert!(footprint.peak_rss > 220 * MB, "{footprint:?}");
        assert!(footprint.peak_helpers >= 1, "the hog child counted");
        assert_eq!(footprint.account_id.as_deref(), Some("acc-e14"));
    }

    let advice = machine_health::machine_advice_impl(&state).unwrap();
    let heavy = advice.iter().find(|a| a.recipe_id == "rec-hog").unwrap();
    assert_eq!(heavy.based_on, 3);

    // The fourth session's real peak against the projection.
    let session = launch_hog(&state, dir.path(), "hog-check", 330);
    let observed = run_until_resident(&state, &session, 290 * MB);
    end(&state, &session);
    let projected = heavy.projected_bytes as f64;
    let error = (projected - observed as f64).abs() / observed as f64;
    eprintln!(
        "[e14] projection {:.0} MB vs observed peak {:.0} MB: error {:.1} %",
        projected / 1e6,
        observed as f64 / 1e6,
        error * 100.0
    );
    assert!(error <= 0.20, "projection off by {:.1} %", error * 100.0);
}

/// AJ-E1406 Req.1: a session that is still running (or about to crash)
/// already has a footprint — written every 10 minutes (300 ticks), not only
/// when it ends.
#[test]
fn a_running_session_leaves_a_footprint_before_it_ends() {
    let dir = tempfile::tempdir().unwrap();
    let state = seeded(dir.path());
    let session = launch_hog(&state, dir.path(), "hog-running", 200);
    run_until_resident(&state, &session, 150 * MB);
    let written = |state: &Arc<AppState>| {
        state
            .storage
            .lock()
            .unwrap()
            .footprints()
            .of_key("recipe:rec-hog")
            .unwrap()
    };
    // The 10-minute cadence, without waiting ten minutes: 300 ticks.
    let mut ticks = 0;
    while written(&state).is_empty() && ticks < 300 {
        commands::processes_tick(&state);
        ticks += 1;
    }
    let rows = written(&state);
    assert_eq!(rows.len(), 1, "written while running (after {ticks} ticks)");
    assert!(rows[0].peak_rss > 150 * MB);
    // Simulate a crash: the app never sees it end, the footprint stands.
    state
        .supervisor
        .stop(&session, Duration::from_secs(3))
        .unwrap();
}
