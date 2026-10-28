//! 0.1.5 review: opening a session's, project's, account's or tool's page
//! with many sessions showed the spinning wheel. A sync Tauri command
//! runs on the main thread, and these did their reads there: the switch
//! ranking (which also asked each tool for its version), the transcript
//! scan behind Usage, the processes and storage reads, each waiting on
//! the storage lock whenever a background pass held it.
//!
//! On a large data set (10 accounts, 600 sessions, 400 conversations
//! started outside AI Juggler, a 40 MB transcript, tools that take 300 ms
//! to say their version), each command a page asks for on open:
//! - is a future whose first poll returns at once, even while the
//!   storage lock is held elsewhere (it would block forever if its work
//!   ran on the polling thread), and
//! - completes off that thread once the lock is free.
//!
//! Each prints a `[perf]` line: the work's own time (what used to block
//! the main thread) and the polling thread's time now.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::future::Future;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Waker};
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::state::AppState;
use juggler_app::{commands, crossmachine, features, history};
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, SettingsOverlay, Timestamp, TranscriptRef,
};
use tauri::Manager as _;

#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

const SESSIONS: usize = 600;
const OUTSIDE: usize = 400;
const ACCOUNTS: usize = 10;

/// A tool that takes as long as a real one to say its version.
fn slow_tool(dir: &Path, name: &str, version: &str) -> PathBuf {
    let path = dir.join(name);
    stand_in::write(&path, format!("#!/bin/sh\nsleep 0.3\necho '{version}'\n"));
    path
}

/// A transcript of `lines` assistant turns, each with usage and a model.
fn big_transcript(path: &Path, lines: usize) {
    let mut file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let words = "lorem ipsum dolor sit amet ".repeat(12);
    for n in 0..lines {
        let minute = n % 60;
        writeln!(
            file,
            "{{\"type\":\"assistant\",\"timestamp\":\"2026-10-21T10:{minute:02}:00.000Z\",\"message\":{{\"model\":\"claude-sonnet-4-5\",\"content\":\"{words}\"}},\"usage\":{{\"input_tokens\":120,\"output_tokens\":40}}}}"
        )
        .unwrap();
    }
}

/// One outside conversation in the Claude Code layout.
fn outside_transcript(config: &Path, cwd: &Path, id: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"Ask {id}\"}}}}\n{{\"type\":\"assistant\",\"sessionId\":\"{id}\",\"timestamp\":\"2026-10-21T10:01:00.000Z\",\"message\":{{\"model\":\"claude-sonnet-4-5\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":5}}}}}}\n",
            cwd.display()
        ),
    )
    .unwrap();
}

struct Fixture {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    /// A session with the big transcript.
    session: String,
    account: String,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(slow_tool(
        &root,
        "claude",
        "2.0.0 (Claude Code)",
    ))));
    registry.register(Arc::new(CodexAdapter::with_bin(slow_tool(
        &root,
        "codex",
        "codex-cli 0.163.0",
    ))));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());

    // The account with the outside conversations, brought in where it is.
    let config = root.join("setup");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("settings.json"), "{}").unwrap();
    for n in 0..OUTSIDE {
        let repo = root.join(format!("Dev/app-{:02}", n % 20));
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        outside_transcript(&config, &repo, &format!("outside-{n:04}"));
    }
    let imported = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        config.to_str().unwrap(),
        "Work",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap()
    .account;
    commands::projects_backfill_impl(&state).unwrap();

    let transcript = root.join("big.jsonl");
    big_transcript(&transcript, 100_000);
    {
        let storage = state.storage.lock().unwrap();
        let projects: Vec<_> = (0..20)
            .map(|i| {
                storage
                    .projects()
                    .touch(&root.join(format!("Dev/app-{i:02}")), None)
                    .unwrap()
                    .id
            })
            .collect();
        let mut profiles: Vec<ProfileId> = storage
            .profiles()
            .list_by_account(&AccountId::new(imported.id.as_str()))
            .unwrap()
            .into_iter()
            .map(|profile| profile.id)
            .collect();
        assert!(!profiles.is_empty(), "the import made its launch settings");
        for i in 1..ACCOUNTS {
            let id = format!("acc-{i}");
            let state_dir = root.join("accounts").join(&id);
            std::fs::create_dir_all(&state_dir).unwrap();
            storage
                .accounts()
                .insert(&Account {
                    id: AccountId::new(id.as_str()),
                    harness: HarnessId::new(CLAUDE_CODE_ID),
                    label: format!("Account {i:02}"),
                    plan: None,
                    state_dir,
                    cleanup_opt_in: false,
                    renewal_date: None,
                    preview_opt_in: false,
                    import_source_path: None,
                    import_origin: None,
                    imported_at: None,
                    in_place: false,
                })
                .unwrap();
            let profile = ProfileId::new(format!("rec-{i}"));
            storage
                .profiles()
                .insert(&LaunchRecipe {
                    id: profile.clone(),
                    name: "Main".into(),
                    account: AccountId::new(id.as_str()),
                    settings_overlay: SettingsOverlay::default(),
                    default_cwd: None,
                })
                .unwrap();
            profiles.push(profile);
        }
        let statuses = [
            SessionStatus::Running,
            SessionStatus::WaitingUser,
            SessionStatus::Finished,
            SessionStatus::Stopped,
        ];
        for n in 0..SESSIONS {
            let mut record = InstanceRecord {
                id: SessionId::new(format!("s-{n:04}")),
                conversation: None,
                profile: profiles[n % profiles.len()].clone(),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: Some(projects[n % 20].clone()),
                cwd: root.join(format!("Dev/app-{:02}", n % 20)),
                status: SessionStatus::Starting,
                started_at: Timestamp::now(),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: (n == 0).then(|| TranscriptRef {
                    path: transcript.clone(),
                    native_id: None,
                }),
                window: None,
            };
            storage.sessions().insert(&record).unwrap();
            record.status = statuses[n % statuses.len()];
            storage.sessions().update(&record).unwrap();
        }
    }
    Fixture {
        _dir: dir,
        state,
        session: "s-0000".to_owned(),
        account: imported.id,
    }
}

/// The work's own time: what a sync command held the main thread for.
fn work<T>(what: &str, run: impl Fn() -> T) -> Duration {
    let mut runs: Vec<Duration> = (0..3)
        .map(|_| {
            let started = Instant::now();
            let _ = run();
            started.elapsed()
        })
        .collect();
    runs.sort();
    // The first ask is the page's first open (cold caches, version probes).
    let first = runs[runs.len() - 1];
    eprintln!(
        "[perf] {what}: its work takes {first:?} (fastest {:?})",
        runs[0]
    );
    first
}

/// Polls `future` once on this thread while the storage lock is held
/// here, then lets it finish. Returns the polling thread's time.
fn off_main<T>(
    what: &str,
    state: &Arc<AppState>,
    future: impl Future<Output = Result<T, juggler_app::error::AppError>>,
) -> (Duration, Result<T, juggler_app::error::AppError>) {
    let mut future: Pin<Box<_>> = Box::pin(future);
    let held = state.storage.lock().unwrap();
    let started = Instant::now();
    let first = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let blocked = started.elapsed();
    assert!(
        first.is_pending(),
        "{what}: finished in its first poll while storage was held, so its work ran on the calling thread"
    );
    drop(held);
    let result = tauri::async_runtime::block_on(future);
    eprintln!("[perf] {what}: the calling thread is held {blocked:?} now");
    assert!(
        blocked < Duration::from_millis(16),
        "{what}: held the calling thread {blocked:?}"
    );
    (blocked, result)
}

/// For a page that never touches storage (the processes graph lives in
/// memory): its first poll may already finish, since nothing it waits on
/// is held, so `is_pending` would race under load. What it must prove
/// instead: the first poll returns at once, and the page completes while
/// this thread still holds the storage lock, so it never takes it (a page
/// that did would wait here until the bounded timeout).
fn without_storage<T: Send>(
    what: &str,
    state: &Arc<AppState>,
    future: impl Future<Output = Result<T, juggler_app::error::AppError>> + Send,
) -> Result<T, juggler_app::error::AppError> {
    let mut future: Pin<Box<_>> = Box::pin(future);
    let held = state.storage.lock().unwrap();
    let started = Instant::now();
    let first = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    let blocked = started.elapsed();
    eprintln!("[perf] {what}: the calling thread is held {blocked:?} now");
    assert!(
        blocked < Duration::from_millis(16),
        "{what}: held the calling thread {blocked:?}"
    );
    match first {
        std::task::Poll::Ready(result) => {
            drop(held);
            result
        }
        // Finished elsewhere while storage stays held here; a page that
        // took the lock would still be waiting when the time runs out.
        std::task::Poll::Pending => std::thread::scope(|scope| {
            let (tx, rx) = std::sync::mpsc::channel();
            scope.spawn(move || {
                let _ = tx.send(tauri::async_runtime::block_on(future));
            });
            let finished = rx.recv_timeout(Duration::from_secs(30));
            // Let it go before failing, so the scope can end.
            drop(held);
            finished.unwrap_or_else(|_| {
                panic!("{what}: didn't finish while storage was held: it takes the storage lock")
            })
        }),
    }
}

#[test]
fn every_page_opens_without_holding_the_main_thread() {
    let fx = fixture();
    let state = &fx.state;
    let app = tauri::test::mock_builder()
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(Arc::clone(state));
    let managed = || app.state::<Arc<AppState>>();
    let sid = fx.session.clone();
    let account = fx.account.clone();

    // Before: each one's work, as it ran on the main thread.
    let mut slowest = Duration::ZERO;
    for took in [
        work("switch_targets (session page)", || {
            commands::switch_targets_impl(state, &sid).map(|rows| rows.len())
        }),
        work("instance_quality (session page, Usage)", || {
            commands::instance_quality_impl(state, &sid).map(|q| q.avg_tokens_per_min)
        }),
        work("instance_processes (session page, Processes)", || {
            commands::instance_processes_arc(state, &sid).is_ok()
        }),
        work("fleet_switch_targets (session page)", || {
            crossmachine::fleet_switch_targets_impl(state, &sid).is_ok()
        }),
        work("quota_display (account page)", || {
            tauri::async_runtime::block_on(commands::quota_display(managed(), account.clone()))
                .is_ok()
        }),
        work("account_storage (account page)", || {
            history::account_storage_impl(state).map(|rows| rows.len())
        }),
        work("providers_discover (tool page)", || {
            commands::providers_discover_impl(state).map(|rows| rows.len())
        }),
        work("conversations_list (every page)", || {
            features::conversations::commands::list_impl(state).map(|rows| rows.len())
        }),
    ] {
        slowest = slowest.max(took);
    }
    eprintln!("[perf] slowest page-open work: {slowest:?}");

    // Now: none of it on the calling thread.
    let (_, rows) = off_main(
        "switch_targets",
        state,
        commands::switch_targets(managed(), sid.clone()),
    );
    rows.unwrap();
    let (_, quality) = off_main(
        "instance_quality",
        state,
        commands::instance_quality(managed(), sid.clone()),
    );
    let quality = quality.unwrap();
    assert!(
        quality.error_label.contains("100000 turns"),
        "the whole transcript was read: {}",
        quality.error_label
    );
    // AJ-E2407: a session's Processes tab reads the one graph, in memory:
    // it never takes the storage lock (and may finish in its first poll).
    let _ = without_storage(
        "processes_page",
        state,
        juggler_app::features::processes::commands::processes_page(
            managed(),
            juggler_app::features::processes::dto::ProcessQueryInput {
                view: "running".into(),
                session_id: Some(sid.clone()),
                ..Default::default()
            },
            juggler_app::paging::PageRequest::first(50),
        ),
    );
    let _ = off_main(
        "fleet_switch_targets",
        state,
        crossmachine::fleet_switch_targets(managed(), sid.clone()),
    );
    let _ = off_main(
        "fleet_switch_back_plan",
        state,
        crossmachine::switch_back::fleet_switch_back_plan(managed(), sid.clone()),
    );
    off_main(
        "fleet_divergences",
        state,
        crossmachine::switch_back::fleet_divergences(managed()),
    )
    .1
    .unwrap();
    off_main(
        "quota_display",
        state,
        commands::quota_display(managed(), account.clone()),
    )
    .1
    .unwrap();
    let _ = off_main(
        "cleanup_tiers_get",
        state,
        commands::cleanup_tiers_get(managed(), account.clone()),
    );
    off_main(
        "account_storage",
        state,
        history::account_storage(managed()),
    )
    .1
    .unwrap();
    off_main(
        "providers_discover",
        state,
        commands::providers_discover(managed()),
    )
    .1
    .unwrap();
    let listed = off_main(
        "conversations_list",
        state,
        features::conversations::commands::conversations_list(managed()),
    )
    .1
    .unwrap();
    // The window holds the working set; the sessions list pages them all.
    assert!(listed.len() <= features::conversations::commands::RECENT);
    let all = features::sessions::rows::sessions_counts_impl(
        state,
        &features::sessions::rows::SessionsQuery::default(),
    )
    .unwrap();
    assert_eq!(all.all, (OUTSIDE + SESSIONS) as u64);
}
