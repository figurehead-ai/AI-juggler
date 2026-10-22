//! AJ-E805 backend load proof at the scale the product promises (10
//! Accounts over three Providers, 50 Instances in mixed statuses, 20
//! Projects): the backend share of cold start (opening the store and
//! reading what the first paint needs) and local terminal input echo
//! through a real PTY. Each prints a `[perf]` line the evidence quotes.

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
use std::time::{Duration, Instant};

use base64::Engine as _;
use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::state::AppState;
use juggler_app::{commands, terminal};
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, SettingsOverlay, Timestamp,
};

/// Each budget is measured alone: the others spawn tens of processes,
/// which a measurement running beside them would count as its own cost.
fn alone() -> std::sync::MutexGuard<'static, ()> {
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TURN.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn median(mut runs: Vec<Duration>) -> Duration {
    runs.sort();
    runs[runs.len() / 2]
}

fn open(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap())
}

/// 10 Accounts / 10 Launch settings / 20 Projects / 50 Instances.
fn seed(dir: &Path) {
    let state = open(dir);
    let storage = state.storage.lock().unwrap();
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
    for i in 0..10 {
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
    let statuses = [
        SessionStatus::Running,
        SessionStatus::WaitingUser,
        SessionStatus::Stuck,
        SessionStatus::QuotaBlocked,
        SessionStatus::Finished,
        SessionStatus::Stopped,
        SessionStatus::Failed,
    ];
    for n in 0..50 {
        let mut record = InstanceRecord {
            id: SessionId::new(format!("s-{n}")),
            conversation: None,
            profile: ProfileId::new(format!("rec-{}", n % 10)),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: Some(projects[n % 20].clone()),
            cwd: PathBuf::from(format!("/work/project-{:02}", n % 20)),
            status: SessionStatus::Starting,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        };
        storage.sessions().insert(&record).unwrap();
        record.status = statuses[n % statuses.len()];
        storage.sessions().update(&record).unwrap();
    }
}

/// Budget: cold start to interactive < 2.5 s. This is the backend
/// share — open (and migrate) the store, then read what the first paint
/// shows; the webview's own share is measured by the UI load spec.
#[test]
fn backend_cold_start_at_scale() {
    let _alone = alone();
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let mut runs = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        let state = open(dir.path());
        let sessions = state.storage.lock().unwrap().sessions().list(None).unwrap();
        let projects = commands::projects_list_impl(&state).unwrap();
        runs.push(started.elapsed());
        assert_eq!(sessions.len(), 50);
        assert_eq!(projects.len(), 20);
        drop(state);
    }
    let mid = median(runs.clone());
    eprintln!("[perf] backend cold start at scale: median {mid:?} (runs {runs:?})");
    assert!(mid < Duration::from_millis(2500), "{mid:?}");
}

/// Budget: terminal input echo < 50 ms local — from the dock's write to
/// the echoed bytes being readable for the terminal, through a real PTY.
#[test]
fn terminal_input_echo_under_50ms() {
    let _alone = alone();
    let work = tempfile::tempdir().unwrap();
    let state = open(&work.path().join("app-data"));
    let id = SessionId::new("s-echo");
    let spec = LaunchSpec {
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), "cat".into()],
        env: EnvMap::new(),
        cwd: work.path().to_path_buf(),
        needs_pty: true,
    };
    let ctx = juggler_supervisor::tree::SpawnContext {
        provider: "fake-harness".into(),
        account_id: Some("acc-fake".into()),
        project_id: Some("proj-fake".into()),
        machine: MachineId::new("machine-test"),
        role: "root",
    };
    state.supervisor.spawn(&id, &spec, &ctx).unwrap();
    let id = id.as_str().to_string();
    let echoed = |token: &str| {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(terminal::terminal_snapshot_impl(&state, &id).unwrap())
            .unwrap();
        bytes.windows(token.len()).any(|w| w == token.as_bytes())
    };
    // Warm up: the shell is ready once the first line comes back.
    terminal::terminal_write_impl(&state, &id, "ready\n").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !echoed("ready") && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut runs = Vec::new();
    for n in 0..20 {
        let token = format!("echo-{n:03}");
        let started = Instant::now();
        terminal::terminal_write_impl(&state, &id, &token).unwrap();
        while !echoed(&token) {
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "no echo for {token}"
            );
            std::thread::sleep(Duration::from_micros(200));
        }
        runs.push(started.elapsed());
    }
    terminal::terminal_kill_impl(&state, &id).unwrap();
    let mut sorted = runs.clone();
    sorted.sort();
    let mid = median(runs);
    eprintln!(
        "[perf] terminal input echo: median {mid:?}, p95 {:?}, max {:?}",
        sorted[18], sorted[19]
    );
    assert!(mid < Duration::from_millis(50), "{mid:?}");
}

/// Budget: CPU idle < 2 % with 10 live Instances, memory < 600 MB
/// excluding child processes. A backend proxy: 10 quiet PTY Instances
/// under the supervisor and the app's 2 s per-tree sampling tick for
/// 30 s, measured as this process's CPU time over wall time and its
/// resident size. The release app with 10 real Instances is measured on
/// the reference Mac (`docs/qa/evidence/AJ-E805/manifest.md`). Ignored
/// by default (30 s); `just load-gate` runs it alone.
#[test]
#[ignore = "30 s idle measurement; run by `just load-gate`"]
fn idle_with_ten_live_instances() {
    use cpu_time::ProcessTime;
    let work = tempfile::tempdir().unwrap();
    let state = open(&work.path().join("app-data"));
    let ctx = juggler_supervisor::tree::SpawnContext {
        provider: "fake-harness".into(),
        account_id: Some("acc-fake".into()),
        project_id: Some("proj-fake".into()),
        machine: MachineId::new("machine-test"),
        role: "root",
    };
    let ids: Vec<SessionId> = (0..10)
        .map(|n| SessionId::new(format!("idle-{n}")))
        .collect();
    for id in &ids {
        let spec = LaunchSpec {
            program: PathBuf::from("/bin/sleep"),
            args: vec!["300".into()],
            env: EnvMap::new(),
            cwd: work.path().to_path_buf(),
            needs_pty: true,
        };
        state.supervisor.spawn(id, &spec, &ctx).unwrap();
    }
    // Settle, then measure 15 ticks of the 2 s cadence.
    commands::processes_tick(&state);
    std::thread::sleep(Duration::from_secs(2));
    let cpu = ProcessTime::now();
    let wall = Instant::now();
    for _ in 0..15 {
        commands::processes_tick(&state);
        std::thread::sleep(Duration::from_secs(2));
    }
    let used = cpu.elapsed();
    let elapsed = wall.elapsed();
    // The tick really sampled every tree (a skipped tree would be cheap).
    {
        let collector = state.resources.lock().unwrap();
        for id in &ids {
            assert!(
                collector.rows(id).is_some_and(|rows| !rows.is_empty()),
                "{id:?} was never sampled"
            );
        }
    }
    let pct = used.as_secs_f64() / elapsed.as_secs_f64() * 100.0;
    let rss_kb: u64 = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0);
    for id in &ids {
        let _ = terminal::terminal_kill_impl(&state, id.as_str());
    }
    eprintln!(
        "[perf] idle with 10 live Instances: CPU {pct:.2} % ({used:?} over {elapsed:?}), RSS {} MB",
        rss_kb / 1024
    );
    assert!(pct < 2.0, "{pct:.2} %");
    assert!(rss_kb > 0 && rss_kb < 600 * 1024, "{rss_kb} KB");
}

/// AJ-E1403 budget (SPEC §5.3): This Mac's aggregation ≤ 200 ms at 50
/// Instances — 50 live supervised trees (each a shell with a helper),
/// with full rings, hog rules and the ledger's leftover count.
#[test]
fn this_mac_at_fifty_instances() {
    let _alone = alone();
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let state = open(dir.path());
    let ctx = juggler_supervisor::tree::SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-0".into()),
        project_id: None,
        machine: commands::local_machine_id(&state),
        role: "root",
    };
    let mut live = Vec::new();
    {
        let storage = state.storage.lock().unwrap();
        for record in storage.sessions().list(None).unwrap() {
            let mut record = record;
            record.status = SessionStatus::Running;
            storage.sessions().update(&record).unwrap();
            live.push(record.id.clone());
        }
    }
    for session in &live {
        state
            .supervisor
            .spawn(
                session,
                &LaunchSpec {
                    program: PathBuf::from("/bin/sh"),
                    args: vec!["-c".into(), "sleep 120 & wait".into()],
                    env: EnvMap::new(),
                    cwd: dir.path().to_path_buf(),
                    needs_pty: false,
                },
                &ctx,
            )
            .unwrap();
    }
    for _ in 0..4 {
        commands::processes_tick(&state);
        std::thread::sleep(Duration::from_millis(200));
    }
    let mut runs = Vec::new();
    let mut trees = 0;
    for _ in 0..5 {
        let started = Instant::now();
        let info =
            juggler_app::machine_health::this_mac_impl(&state, Timestamp::now().millis()).unwrap();
        runs.push(started.elapsed());
        trees = info.ours.count;
    }
    for session in &live {
        let _ = state.supervisor.stop(session, Duration::from_secs(2));
    }
    let mid = median(runs.clone());
    eprintln!("[perf] this mac at 50 instances ({trees} trees): median {mid:?} (runs {runs:?})");
    assert_eq!(trees, 50, "every live tree counted");
    assert!(mid < Duration::from_millis(200), "{mid:?}");
}

/// AJ-E1410 budget (SPEC §1): resource monitoring costs < 1 % of one core.
/// One simulated minute: 30 of the 2 s ticks and 12 of the 5 s This Mac
/// emissions, with 5 live sessions. The budget is held by doing little, and
/// that is asserted deterministically:
/// - a full scan (every process on the Mac) at most every 5th tick;
/// - a list-only read of every process (P16) at most every 2nd tick
///   between them;
/// - otherwise only the known processes are read, a handful per tick.
///
/// CPU time is asserted too, but it measures the machine as well as us: a
/// full scan's cost grows with every process on it, and the other tools'
/// trees (the Claude app and what it runs, builds included) are read every
/// tick. The budget itself, 1 %, is held where those are few (under 100
/// processes a tick); on a CI runner (1.118 % once, AJ-E806) or beside a
/// busy Claude app (1.36 % at 290 of its processes a tick, 0.79 % at 155)
/// the bound is 2 %.
#[test]
fn monitoring_costs_under_one_percent_of_a_core() {
    let _alone = alone();
    use cpu_time::ThreadTime;
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let state = open(dir.path());
    let ctx = juggler_supervisor::tree::SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-0".into()),
        project_id: None,
        machine: commands::local_machine_id(&state),
        role: "root",
    };
    let live: Vec<SessionId> = (0..5).map(|n| SessionId::new(format!("s-{n}"))).collect();
    {
        let storage = state.storage.lock().unwrap();
        for session in &live {
            let mut record = storage.sessions().get(session).unwrap();
            record.status = SessionStatus::Running;
            storage.sessions().update(&record).unwrap();
        }
    }
    for session in &live {
        state
            .supervisor
            .spawn(
                session,
                &LaunchSpec {
                    program: PathBuf::from("/bin/sh"),
                    args: vec!["-c".into(), "sleep 120 & wait".into()],
                    env: EnvMap::new(),
                    cwd: dir.path().to_path_buf(),
                    needs_pty: false,
                },
                &ctx,
            )
            .unwrap();
    }
    // Warm-up: the first ticks list the disks and scan everything.
    for _ in 0..4 {
        commands::processes_tick(&state);
    }
    let before = state.resources.lock().unwrap().work();
    let started = ThreadTime::now();
    for tick in 0..30 {
        commands::processes_tick(&state);
        if tick % 5 == 2 || tick % 5 == 4 {
            // 12 emissions a minute (every 5 s).
            juggler_app::machine_health::this_mac_impl(&state, Timestamp::now().millis()).unwrap();
        }
    }
    let used = started.elapsed();
    let after = state.resources.lock().unwrap().work();
    // The breakdown: E14's machine half (sampler, running tools,
    // footprints, This Mac) on its own, over another minute.
    let machine_started = ThreadTime::now();
    for tick in 0..30 {
        juggler_app::machine_health::tick(&state, Timestamp::now().millis() + tick * 2_000);
        if tick % 5 == 2 || tick % 5 == 4 {
            juggler_app::machine_health::this_mac_impl(&state, Timestamp::now().millis()).unwrap();
        }
    }
    let machine_share = machine_started.elapsed().as_secs_f64() / 60.0 * 100.0;
    for session in &live {
        let _ = state.supervisor.stop(session, Duration::from_secs(2));
    }
    let share = used.as_secs_f64() / 60.0 * 100.0;
    let ticks = after.ticks - before.ticks;
    let full = after.full_scans - before.full_scans;
    let ours = after.targeted_ours - before.targeted_ours;
    let observed = after.targeted_observed - before.targeted_observed;
    let listings = after.listings - before.listings;
    eprintln!(
        "[perf] monitoring: {ticks} ticks, {full} full scans, {listings} listings; between them {ours} of our sessions' processes and {observed} of other tools' read; {used:?} CPU per simulated minute = {share:.3} % of one core (5 sessions); the E14 machine half alone {machine_share:.3} %"
    );
    assert_eq!(ticks, 30);
    assert!(full <= 6, "a full scan at most every 5th tick: {full}");
    // Between full scans, only what is watched is read: each session's
    // root shell and its `sleep`, once each (2 reads a tick; a root read
    // twice made it 3), never the Mac's other processes. Other tools' trees
    // (the Claude app, if this Mac runs one) are read as they are; how many
    // is up to the machine.
    let between = ticks - full;
    assert!(
        ours > 0 && ours <= between * 5 * 2,
        "our sessions' processes, once each: {ours} over {between} ticks"
    );
    assert!(
        listings > 0 && listings <= between.div_ceil(2),
        "a listing at most every 2nd tick: {listings} over {between} ticks"
    );
    let bound = if std::env::var_os("CI").is_some() || observed >= between * 100 {
        2.0
    } else {
        1.0
    };
    assert!(share < bound, "{share:.3} % of a core (bound {bound} %)");
}

/// P16 (QA): a tree's new processes counted at once. The collector read the
/// whole process list only every 5th tick, so a build that fanned out under
/// a session between those scans was missing from its tree for up to 10 s
/// (one 17.2 GB tree read about 2 GB). The session's shell starts two
/// children after the warm-up's full scans; the next tick, a targeted one,
/// already counts them.
#[test]
fn a_trees_new_processes_count_at_the_next_tick() {
    let _alone = alone();
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path());
    let state = open(dir.path());
    let session = SessionId::new("s-0");
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage.sessions().get(&session).unwrap();
        record.status = SessionStatus::Running;
        storage.sessions().update(&record).unwrap();
    }
    let go = dir.path().join("go");
    let kids = dir.path().join("kids");
    let script = format!(
        "while [ ! -e '{go}' ]; do sleep 0.05; done; \
         sleep 120 & echo $! >> '{kids}'; sleep 120 & echo $! >> '{kids}'; wait",
        go = go.display(),
        kids = kids.display()
    );
    state
        .supervisor
        .spawn(
            &session,
            &LaunchSpec {
                program: PathBuf::from("/bin/sh"),
                args: vec!["-c".into(), script],
                env: EnvMap::new(),
                cwd: dir.path().to_path_buf(),
                needs_pty: false,
            },
            &juggler_supervisor::tree::SpawnContext {
                provider: "claude-code".into(),
                account_id: Some("acc-0".into()),
                project_id: None,
                machine: commands::local_machine_id(&state),
                role: "root",
            },
        )
        .unwrap();
    // The warm-up's full scans (ticks 1–3) see only the waiting shell.
    for _ in 0..3 {
        commands::processes_tick(&state);
    }
    std::fs::write(&go, "").unwrap();
    let started = std::time::Instant::now();
    let children: Vec<u32> = loop {
        let text = std::fs::read_to_string(&kids).unwrap_or_default();
        let pids: Vec<u32> = text
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect();
        if pids.len() == 2 {
            break pids;
        }
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "the children never started"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    // Tick 4 is a targeted tick (no full scan until tick 5), and one that
    // lists the processes (every 2nd tick).
    commands::processes_tick(&state);
    let collector = state.resources.lock().unwrap();
    assert_eq!(collector.work().full_scans, 3, "tick 4 did no full scan");
    let rows: Vec<u32> = collector
        .rows(&session)
        .unwrap()
        .iter()
        .map(|row| row.pid)
        .collect();
    drop(collector);
    let _ = state.supervisor.stop(&session, Duration::from_secs(2));
    for child in &children {
        assert!(
            rows.contains(child),
            "child {child} in the tree at once: {rows:?}"
        );
    }
}
