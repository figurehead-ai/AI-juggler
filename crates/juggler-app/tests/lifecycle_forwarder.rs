//! The lifecycle forwarder keeps going through quiet spells (2026-10-26
//! lab): it used to end after its first half-second without an event, so
//! a session that exited later stayed "running", was never restarted and
//! never notified. Here the app's own forwarder thread runs as it does in
//! the app, and sessions exit only after it has been quiet for a while.

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
use juggler_app::commands;
use juggler_app::features::sessions::lifecycle;
use juggler_app::state::AppState;
use juggler_core::model::{SessionId, SessionStatus};
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// A `claude` that fails at once, every time.
fn failing_claude(dir: &Path) -> PathBuf {
    let path = dir.join("claude-fails");
    stand_in::write(
        &path,
        "#!/bin/sh\necho 'panicked: something broke' >&2\nexit 3\n",
    );
    path
}

fn app_state(dir: &Path, stub: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(stub)));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// An account and launch settings in `folder`.
fn launch_settings(state: &Arc<AppState>, folder: &Path) -> String {
    let account =
        commands::create_account(state, CLAUDE_CODE_ID, "Work Max", Some("max-20x".into()))
            .unwrap();
    commands::create_profile(
        state,
        &account.id,
        "Overnight",
        None,
        Some(folder.to_string_lossy().into_owned()),
    )
    .unwrap()
    .id
}

fn status(state: &Arc<AppState>, session: &str) -> SessionStatus {
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(session))
        .unwrap()
        .status
}

/// Waits up to 15 s for `session` to be stored as failed.
fn wait_failed(state: &Arc<AppState>, session: &str) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while status(state, session) != SessionStatus::Failed {
        assert!(
            Instant::now() < deadline,
            "the forwarder never stored the session's exit"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn it_records_exits_that_come_after_quiet_spells() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), &failing_claude(work.path()));
    let profile = launch_settings(&state, work.path());
    lifecycle::spawn_forwarder(Arc::clone(&state));

    // Quiet for longer than one wait: the old forwarder had ended by now.
    std::thread::sleep(Duration::from_millis(1500));
    let first = commands::launch_profile(&state, &profile, None).unwrap();
    wait_failed(&state, &first.session_id);

    // And again after another quiet spell.
    std::thread::sleep(Duration::from_millis(1500));
    let second = commands::launch_profile(&state, &profile, None).unwrap();
    wait_failed(&state, &second.session_id);
}
