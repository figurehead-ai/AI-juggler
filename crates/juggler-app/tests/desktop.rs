//! AJ-E413 app-level integration: the desktop launch contract — fake
//! desktop app launches, is tracked in the same ledger (no PTY), exits
//! propagate (clean + crash), and the restart reconciler closes dead
//! rows.

// Test crates are pragmatic by policy (QUALITY.md §2).
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
use juggler_adapters::FakeDesktopAdapter;
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, InstanceKind, LaunchRecipe, ProfileId, SettingsOverlay,
};

fn fake_desktop_bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_fake-desktop") {
        return PathBuf::from(path);
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join("fake-desktop");
        if candidate.exists() {
            return candidate;
        }
    }
    unreachable!("fake-desktop binary not found under {}", target.display())
}

fn desktop_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(FakeDesktopAdapter::new(fake_desktop_bin())));
    let state = Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap());
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-desktop"),
                harness: juggler_core::model::HarnessId::new("fake-desktop"),
                label: "Desktop App".into(),
                plan: None,
                state_dir: dir.join("state"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-desktop"),
                name: "Main".into(),
                account: AccountId::new("acc-desktop"),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            })
            .unwrap();
    }
    state
}

fn ctx() -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "fake-harness".into(),
        account_id: Some("acc-fake".into()),
        project_id: Some("proj-fake".into()),
        machine: juggler_core::model::MachineId::new("machine-test"),
        role: "root",
    }
}

#[test]
fn desktop_launch_tracks_in_the_same_ledger_without_a_pty() {
    let dir = tempfile::tempdir().unwrap();
    let state_ref = &desktop_state(dir.path());
    let started = Instant::now();
    let info = commands::desktop_launch_arc(state_ref, "acc-desktop").unwrap();
    // Launch-to-tracked budget (<3 s, Req. performance).
    assert!(started.elapsed() < Duration::from_secs(3));

    assert!(info.bundle_id.as_deref() == Some("dev.aijuggler.fake-desktop"));
    assert_eq!(
        info.isolation_label,
        "Single-account app — Juggler tracks, cannot isolate"
    );
    let session = juggler_core::model::SessionId::new(info.session_id.clone());
    {
        let storage = state_ref.storage.lock().unwrap();
        let record = storage.sessions().get(&session).unwrap();
        assert_eq!(record.kind, InstanceKind::Desktop);
        assert!(record.status == juggler_core::model::SessionStatus::Starting);
    }
    // No PTY by construction: input writes refuse with NotATty.
    assert!(state_ref.supervisor.write_input(&session, b"x").is_err());
    // Alive; stop cleanly through the ladder (the same one terminal
    // instances use — same ledger, same stop contract), then apply the
    // event exactly like the lifecycle forwarder does.
    state_ref
        .supervisor
        .stop(&session, Duration::from_secs(5))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if Instant::now() > deadline {
            panic!("desktop exit never propagated within the budget");
        }
        if let Some(event) = state_ref.supervisor.try_recv()
            && let juggler_supervisor::LifecycleEvent::Exited { .. } = &event
        {
            let storage = state_ref.storage.lock().unwrap();
            let updated = commands::apply_lifecycle_event(
                &storage,
                event,
                state_ref.supervisor.was_stopped(&session),
                None,
            )
            .expect("row updated");
            assert_eq!(updated.status, juggler_core::model::SessionStatus::Stopped);
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn desktop_crash_surfaces_as_failed_state() {
    let dir = tempfile::tempdir().unwrap();
    let state_ref = &desktop_state(dir.path());
    // The crash fixture: the real fake-desktop binary with --crash,
    // spawned through the same desktop contract with args overridden.
    let account = state_ref
        .storage
        .lock()
        .unwrap()
        .accounts()
        .get(&AccountId::new("acc-desktop"))
        .unwrap();
    let mut launch = juggler_adapter::DesktopLaunch {
        program: fake_desktop_bin(),
        bundle_id: Some("dev.aijuggler.fake-desktop".to_string()),
        args: vec!["--crash".into(), "--exit-after-secs".into(), "1".into()],
        env: Default::default(),
        cwd: account.state_dir.clone(),
        isolation_label: juggler_adapters::SINGLE_ACCOUNT_LABEL,
    };
    launch.cwd = dir.path().to_path_buf();
    let session = juggler_core::model::SessionId::new("sess-desktop-crash");
    let spawned = state_ref
        .supervisor
        .spawn_desktop(&session, &launch, &ctx())
        .unwrap();
    assert!(spawned.pid != 0);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if Instant::now() > deadline {
            panic!("crash fixture never exited");
        }
        if let Some(event) = state_ref.supervisor.try_recv()
            && let juggler_supervisor::LifecycleEvent::Exited { kind, .. } = &event
        {
            assert_eq!(*kind, juggler_supervisor::ExitKind::Code(7));
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn restart_reconciler_closes_dead_desktop_rows_never_fake_live() {
    let dir = tempfile::tempdir().unwrap();
    {
        let state_ref = &desktop_state(dir.path());
        let info = commands::desktop_launch_arc(state_ref, "acc-desktop").unwrap();
        state_ref
            .supervisor
            .stop(
                &juggler_core::model::SessionId::new(info.session_id.clone()),
                Duration::from_secs(5),
            )
            .unwrap();
        // Drain the exit so the row is terminal before "restart".
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let event = state_ref.supervisor.try_recv();
            if let Some(event) = event {
                let storage = state_ref.storage.lock().unwrap();
                let _ = commands::apply_lifecycle_event(&storage, event, true, None);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    // A fresh AppState over the same data dir reconciles: any row that
    // still claims Running/Starting but has no live adopted process is
    // closed as Stopped (the E410-era reconciler covers desktop rows —
    // kind-blind by design).
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(FakeDesktopAdapter::new(fake_desktop_bin())));
    let reopened = AppState::init_with_registry(dir.path().join("app-data"), registry).unwrap();
    let rows = reopened
        .storage
        .lock()
        .unwrap()
        .sessions()
        .list(None)
        .unwrap();
    let desktop_row = rows
        .iter()
        .find(|row| row.kind == InstanceKind::Desktop)
        .expect("desktop row survived the restart");
    assert_ne!(
        desktop_row.status,
        juggler_core::model::SessionStatus::Starting
    );
}

#[test]
fn providers_without_a_desktop_app_are_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    // Default registry: Claude Code only — no DESKTOP_APP capability.
    let state = Arc::new(
        AppState::init_with_registry(
            dir.path().join("app-data"),
            juggler_app::state::default_registry(),
        )
        .unwrap(),
    );
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-claude"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Work Max".into(),
                plan: None,
                state_dir: dir.path().join("claude-state"),
                cleanup_opt_in: false,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-claude"),
                name: "Main".into(),
                account: AccountId::new("acc-claude"),
                settings_overlay: SettingsOverlay::default(),
                default_cwd: None,
            })
            .unwrap();
    }
    let error = commands::desktop_launch_arc(&state, "acc-claude").unwrap_err();
    assert!(
        error.to_string().contains("ships no desktop app"),
        "capability refusal names the limit: {error}"
    );
}

/// AJ-E413 evidence: ONE real desktop harness launch through the same
/// spawn ledger — TextEdit, the sacrificial desktop app (a fresh
/// untitled window only; no documents are opened or touched). Ignored
/// in CI (no GUI session): run locally with
/// `cargo test -p juggler-app --test desktop real_desktop -- --ignored
/// --nocapture` and store the output as the launch record.
#[test]
#[ignore = "launches a real GUI app; local evidence run only"]
fn real_desktop_launch_record_textedit() {
    let program = PathBuf::from("/System/Applications/TextEdit.app/Contents/MacOS/TextEdit");
    assert!(
        program.exists(),
        "TextEdit not found at {} — macOS-only evidence run",
        program.display()
    );
    let dir = tempfile::tempdir().unwrap();
    let state = &desktop_state(dir.path());
    let launch = juggler_adapter::DesktopLaunch {
        program,
        bundle_id: Some("com.apple.TextEdit".to_string()),
        args: Vec::new(),
        env: Default::default(),
        cwd: dir.path().to_path_buf(),
        isolation_label: juggler_adapters::SINGLE_ACCOUNT_LABEL,
    };
    let session = juggler_core::model::SessionId::new("sess-textedit-record");

    let started = Instant::now();
    let spawned = state
        .supervisor
        .spawn_desktop(&session, &launch, &ctx())
        .unwrap();
    let tracked_in = started.elapsed();
    println!(
        "[record] spawned pid={} bundle=com.apple.TextEdit tracked_in={tracked_in:?}",
        spawned.pid
    );
    // Launch-to-tracked budget (<3 s) against a REAL app bundle.
    assert!(
        tracked_in < Duration::from_secs(3),
        "tracked in {tracked_in:?}"
    );

    // No PTY on the desktop path (input refuses, by construction).
    assert!(state.supervisor.write_input(&session, b"x").is_err());

    // Stop through the ladder; exit must propagate <1 s after the stop
    // call returns (budget: status propagation from process exit).
    state
        .supervisor
        .stop(&session, Duration::from_secs(5))
        .unwrap();
    let t_stop = Instant::now();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if Instant::now() > deadline {
            panic!("TextEdit exit never propagated");
        }
        if let Some(event) = state.supervisor.try_recv()
            && let juggler_supervisor::LifecycleEvent::Exited { .. } = &event
        {
            let propagated_in = t_stop.elapsed();
            println!("[record] exited after stop propagated_in={propagated_in:?}");
            assert!(
                propagated_in < Duration::from_secs(5),
                "exit propagation {propagated_in:?}"
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    // The supervision journal carries the spawn + exit audit lines.
    let journal = state
        .supervisor
        .journal_ref()
        .expect("AppState supervisors journal by construction");
    let lines = std::fs::read_to_string(journal.path()).unwrap();
    let record_lines: Vec<&str> = lines
        .lines()
        .filter(|line| line.contains("sess-textedit-record"))
        .collect();
    println!("[record] journal lines ({}):", record_lines.len());
    for line in &record_lines {
        println!("[record]   {line}");
    }
    assert!(
        record_lines.iter().any(|l| l.contains("spawned")),
        "journal records the spawn"
    );
    assert!(
        record_lines.iter().any(|l| l.contains("exited")),
        "journal records the exit"
    );
}

// --- AJ-E414: the focus command over the scripted runner seam ---------

/// A scripted focus runner: two windows; the pid path answers unless
/// `pid` is 0 (the synthesized-identity case), the bundle path then
/// re-discovers (pid 42424).
struct FocusFixture {
    permission_denied: std::sync::atomic::AtomicBool,
}

impl juggler_app::focus::ScriptRunner for FocusFixture {
    fn osascript(&self, source: &str) -> Result<String, String> {
        if self
            .permission_denied
            .load(std::sync::atomic::Ordering::SeqCst)
            && source.contains("System Events")
        {
            return Err(
                "execution error: osascript is not allowed assistive access (-25211)".into(),
            );
        }
        if source.contains("get name of every window") {
            if source.contains("unix id is 0") {
                return Err("execution error: Can't get application process (-600)".into());
            }
            return Ok("notes-a, notes-b".into());
        }
        if source.contains("get unix id of") {
            return Ok("42424".into());
        }
        Ok(String::new())
    }
    fn activate(&self, _bundle_id: &str) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn desktop_focus_runs_tiers_and_persists_first_sight_identity() {
    let dir = tempfile::tempdir().unwrap();
    let state = desktop_state(dir.path());
    let info = commands::desktop_launch_arc(&state, "acc-desktop").unwrap();
    // The launch recorded pid + bundle (no title yet).
    {
        let storage = state.storage.lock().unwrap();
        let record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                info.session_id.clone(),
            ))
            .unwrap();
        let identity = record.window.as_ref().unwrap();
        assert_eq!(identity.pid, info.pid);
        assert_eq!(
            identity.bundle_id.as_deref(),
            Some("dev.aijuggler.fake-desktop")
        );
        assert_eq!(identity.title, None);
    }
    let runner = std::sync::Arc::new(FocusFixture {
        permission_denied: std::sync::atomic::AtomicBool::new(false),
    });
    let outcome = commands::desktop_focus_with(&state, &info.session_id, runner).unwrap();
    assert_eq!(outcome.outcome, "Focused window");
    assert!(!outcome.permission_missing);
    assert_eq!(outcome.window_title.as_deref(), Some("notes-a"));
    // First-sight title persisted on the Instance (Req.2).
    let storage = state.storage.lock().unwrap();
    let record = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(
            info.session_id.clone(),
        ))
        .unwrap();
    assert_eq!(
        record.window.as_ref().unwrap().title.as_deref(),
        Some("notes-a")
    );
}

#[test]
fn desktop_focus_on_a_ended_instance_reports_not_running_without_scripts() {
    let dir = tempfile::tempdir().unwrap();
    let state = desktop_state(dir.path());
    let storage = state.storage.lock().unwrap();
    let mut record = juggler_core::model::InstanceRecord {
        id: juggler_core::model::SessionId::new("sess-desk-done"),
        conversation: None,
        profile: ProfileId::new("rec-desktop"),
        machine: juggler_core::model::MachineId::new("this-mac"),
        kind: InstanceKind::Desktop,
        project: None,
        cwd: dir.path().to_path_buf(),
        status: juggler_core::model::SessionStatus::Finished,
        started_at: juggler_core::model::Timestamp::now(),
        last_activity: None,
        usage: Default::default(),
        attention: None,
        running: None,
        window: None,
        transcript_ref: None,
    };
    storage.sessions().insert(&record).unwrap();
    record.status = juggler_core::model::SessionStatus::Finished;
    drop(storage);
    let runner = std::sync::Arc::new(FocusFixture {
        permission_denied: std::sync::atomic::AtomicBool::new(false),
    });
    let outcome = commands::desktop_focus_with(&state, "sess-desk-done", runner).unwrap();
    assert_eq!(outcome.outcome, "App not running — Relaunch?");
    assert_eq!(outcome.window_title, None);
}

#[test]
fn desktop_focus_refuses_terminal_instances_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let state = desktop_state(dir.path());
    let runner = std::sync::Arc::new(FocusFixture {
        permission_denied: std::sync::atomic::AtomicBool::new(false),
    });
    // A terminal-kind row (the read model forbids window focus).
    let storage = state.storage.lock().unwrap();
    storage
        .sessions()
        .insert(&juggler_core::model::InstanceRecord {
            id: juggler_core::model::SessionId::new("sess-term"),
            conversation: None,
            profile: ProfileId::new("rec-desktop"),
            machine: juggler_core::model::MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.path().to_path_buf(),
            status: juggler_core::model::SessionStatus::Running,
            started_at: juggler_core::model::Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            window: None,
            transcript_ref: None,
        })
        .unwrap();
    drop(storage);
    let error = commands::desktop_focus_with(&state, "sess-term", runner).unwrap_err();
    assert!(error.to_string().contains("desktop instances"), "{error}");
}

#[test]
fn desktop_focus_without_recorded_identity_redisovers_by_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let state = desktop_state(dir.path());
    // A pre-v10-style desktop row: live, but no window identity.
    {
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .insert(&juggler_core::model::InstanceRecord {
                id: juggler_core::model::SessionId::new("sess-desk-legacy"),
                conversation: None,
                profile: ProfileId::new("rec-desktop"),
                machine: juggler_core::model::MachineId::new("this-mac"),
                kind: InstanceKind::Desktop,
                project: None,
                cwd: dir.path().to_path_buf(),
                status: juggler_core::model::SessionStatus::Running,
                started_at: juggler_core::model::Timestamp::now(),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                window: None,
                transcript_ref: None,
            })
            .unwrap();
    }
    let runner = std::sync::Arc::new(FocusFixture {
        permission_denied: std::sync::atomic::AtomicBool::new(false),
    });
    let outcome = commands::desktop_focus_with(&state, "sess-desk-legacy", runner).unwrap();
    assert_eq!(outcome.outcome, "Focused window");
    // The synthesized identity was enriched and persisted.
    let storage = state.storage.lock().unwrap();
    let record = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new("sess-desk-legacy"))
        .unwrap();
    let identity = record.window.as_ref().unwrap();
    assert_eq!(identity.pid, 42_424, "re-discovered pid persisted");
    assert_eq!(identity.title.as_deref(), Some("notes-a"));
}

#[test]
fn desktop_focus_permission_denial_degrades_with_flag() {
    let dir = tempfile::tempdir().unwrap();
    let state = desktop_state(dir.path());
    let info = commands::desktop_launch_arc(&state, "acc-desktop").unwrap();
    let runner = std::sync::Arc::new(FocusFixture {
        permission_denied: std::sync::atomic::AtomicBool::new(true),
    });
    let outcome = commands::desktop_focus_with(&state, &info.session_id, runner).unwrap();
    assert_eq!(outcome.outcome, "App opened — window not found");
    assert!(
        outcome.permission_missing,
        "the UI explainer keys on this flag"
    );
}
