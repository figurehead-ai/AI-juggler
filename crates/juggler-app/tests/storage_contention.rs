//! AJ-E2017 R3: the measurement behind ADR-0030 (storage concurrency).
//!
//! Everything the app reads or writes goes through one `Mutex<Storage>`
//! (`AppState::storage`). This measures, at the scale the product promises
//! plus a long history (10 accounts, 50 working sessions, 2,000 ended
//! ones), how long the hot paths hold that lock, and how long a page's
//! read waits for it while the background ticks run. It then runs the same
//! reads on a second, read-only connection to the same WAL database, the
//! alternative the ADR weighs.
//!
//! Ignored by default (it measures; it doesn't guard). Run it with
//! `cargo test -p juggler-app --test storage_contention -- --ignored --nocapture`;
//! it prints one `[contention]` line per figure.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a measurement prints what it measured"
)]
// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a measurement test, not shipped code"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::features::projects::list::projects_list_impl;
use juggler_app::features::sessions::read_model::{session_infos_snapshot, working_records};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, SettingsOverlay, Timestamp,
};
use juggler_core::storage::Storage;

const ACCOUNTS: usize = 10;
const WORKING: usize = 50;
const ENDED: usize = 2_000;

fn seed(storage: &Storage) {
    let providers = ["claude-code", "codex", "zcode"];
    let projects: Vec<_> = (0..20)
        .map(|i| {
            storage
                .projects()
                .touch(Path::new(&format!("/work/project-{i:02}")), None)
                .unwrap()
                .id
        })
        .collect();
    for i in 0..ACCOUNTS {
        let id = format!("acc-{i}");
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new(id.as_str()),
                harness: HarnessId::new(providers[i % 3]),
                label: format!("Account {i:02}"),
                plan: None,
                state_dir: PathBuf::from("/fixture/accounts").join(&id),
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
                id: ProfileId::new(format!("rec-{i}")),
                name: "Main".into(),
                account: AccountId::new(id.as_str()),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            })
            .unwrap();
    }
    // Ended long ago: out of the working set, still in every full list.
    let long_ago = Timestamp::from_millis(Timestamp::now().millis() - 30 * 86_400_000);
    for n in 0..WORKING + ENDED {
        let working = n < WORKING;
        let mut record = InstanceRecord {
            id: SessionId::new(format!("s-{n}")),
            conversation: None,
            profile: ProfileId::new(format!("rec-{}", n % ACCOUNTS)),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(projects[n % 20].clone()),
            cwd: PathBuf::from(format!("/work/project-{:02}", n % 20)),
            status: SessionStatus::Starting,
            started_at: if working { Timestamp::now() } else { long_ago },
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        };
        storage.sessions().insert(&record).unwrap();
        record.status = if working {
            SessionStatus::Running
        } else {
            SessionStatus::Finished
        };
        storage.sessions().update(&record).unwrap();
    }
}

fn percentiles(mut runs: Vec<Duration>) -> (Duration, Duration, Duration) {
    runs.sort();
    let at = |q: f64| runs[((runs.len() - 1) as f64 * q).round() as usize];
    (at(0.5), at(0.95), *runs.last().unwrap())
}

fn report(what: &str, runs: Vec<Duration>) -> Duration {
    let count = runs.len();
    let (p50, p95, max) = percentiles(runs);
    println!("[contention] {what}: p50 {p50:?} p95 {p95:?} max {max:?} (n={count})");
    p95
}

/// How long `op` holds the lock, `rounds` times.
fn hold(state: &AppState, rounds: usize, op: impl Fn(&Storage)) -> Vec<Duration> {
    (0..rounds)
        .map(|_| {
            let storage = state.storage.lock().unwrap();
            let started = Instant::now();
            op(&storage);
            let held = started.elapsed();
            drop(storage);
            held
        })
        .collect()
}

/// The background ticks' storage work, as the app runs it (the bodies'
/// storage parts: 15 s heuristics over every session, 5 s resources and
/// 30 s quota over the working set), sped up 10× so a few seconds hold
/// many ticks, plus a write every 200 ms (a status change or a usage row).
fn ticks(state: Arc<AppState>, stop: Arc<AtomicBool>) -> Vec<std::thread::JoinHandle<()>> {
    let every = |period: Duration, op: Box<dyn Fn(&Storage) + Send>| {
        let state = Arc::clone(&state);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                {
                    let storage = state.storage.lock().unwrap();
                    op(&storage);
                }
                std::thread::sleep(period);
            }
        })
    };
    vec![
        every(
            Duration::from_millis(1_500),
            Box::new(|storage| {
                let _ = storage.sessions().list(None).unwrap();
            }),
        ),
        every(
            Duration::from_millis(500),
            Box::new(|storage| {
                let _ = working_records(storage).unwrap();
            }),
        ),
        every(
            Duration::from_millis(3_000),
            Box::new(|storage| {
                let _ = session_infos_snapshot(storage).unwrap();
            }),
        ),
        every(
            Duration::from_millis(200),
            Box::new(|storage| {
                let mut record = storage.sessions().get(&SessionId::new("s-1")).unwrap();
                record.last_activity = Some(Timestamp::now());
                storage.sessions().update(&record).unwrap();
            }),
        ),
    ]
}

/// A page's read every 50 ms for `span`: how long it waited for the lock.
fn page_reads(state: &AppState, span: Duration) -> Vec<Duration> {
    let until = Instant::now() + span;
    let mut waits = Vec::new();
    while Instant::now() < until {
        let asked = Instant::now();
        let storage = state.storage.lock().unwrap();
        waits.push(asked.elapsed());
        let _ = session_infos_snapshot(&storage).unwrap();
        drop(storage);
        std::thread::sleep(Duration::from_millis(50));
    }
    waits
}

#[test]
#[ignore = "a measurement for ADR-0030; run with --ignored --nocapture"]
fn storage_lock_hold_and_wait_times() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().to_path_buf(), AdapterRegistry::new()).unwrap(),
    );
    seed(&state.storage.lock().unwrap());
    println!("[contention] scale: {ACCOUNTS} accounts, {WORKING} working sessions, {ENDED} ended");

    // 1. How long each hot path holds the one lock.
    report(
        "hold: every session (the 15 s heuristics tick)",
        hold(&state, 30, |storage| {
            let _ = storage.sessions().list(None).unwrap();
        }),
    );
    report(
        "hold: the working set (5 s resources, 30 s quota)",
        hold(&state, 30, |storage| {
            let _ = working_records(storage).unwrap();
        }),
    );
    report(
        "hold: the sessions list's read model (each page open, each change)",
        hold(&state, 30, |storage| {
            let _ = session_infos_snapshot(storage).unwrap();
        }),
    );
    report(
        "hold: one session's status write",
        hold(&state, 30, |storage| {
            let mut record = storage.sessions().get(&SessionId::new("s-2")).unwrap();
            record.last_activity = Some(Timestamp::now());
            storage.sessions().update(&record).unwrap();
        }),
    );
    let projects: Vec<Duration> = (0..30)
        .map(|_| {
            let started = Instant::now();
            let _ = projects_list_impl(&state).unwrap();
            started.elapsed()
        })
        .collect();
    report("hold: the projects list (lock taken inside)", projects);

    // 2. A page's wait for the lock: idle, then with the ticks running.
    report(
        "wait: a page's read, nothing else running",
        page_reads(&state, Duration::from_secs(2)),
    );
    let stop = Arc::new(AtomicBool::new(false));
    let running = ticks(Arc::clone(&state), Arc::clone(&stop));
    let busy = report(
        "wait: a page's read, the ticks running",
        page_reads(&state, Duration::from_secs(6)),
    );

    // 3. The alternative: the page reads on its own read-only connection
    // (WAL lets it read beside the writer), the ticks unchanged.
    let reader = Mutex::new(Storage::open(&dir.path().join("juggler.sqlite")).unwrap());
    let until = Instant::now() + Duration::from_secs(6);
    let mut waits = Vec::new();
    while Instant::now() < until {
        let asked = Instant::now();
        let storage = reader.lock().unwrap();
        let _ = session_infos_snapshot(&storage).unwrap();
        waits.push(asked.elapsed());
        drop(storage);
        std::thread::sleep(Duration::from_millis(50));
    }
    let own = report(
        "read on its own connection (wait + read), the ticks running",
        waits,
    );
    stop.store(true, Ordering::Relaxed);
    for tick in running {
        tick.join().unwrap();
    }
    println!("[contention] page p95: one lock {busy:?}; own connection {own:?} (incl. the read)");
}
