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
