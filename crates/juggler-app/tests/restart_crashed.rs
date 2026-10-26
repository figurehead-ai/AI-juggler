//! AJ-E1509 Req.1 on a real launch: a stub `claude` that always crashes.
//! With restarting off nothing happens; on (in Settings, or by a launch
//! setting's own choice) the session is started again on the same launch
//! settings, each try in both sessions' history, until the third try in
//! ten minutes, after which it stays ended and says why. The cases that
//! are never restarted (a finish, a stop, a usage limit, a failed sign-in)
//! are the restarter's unit tests; how an exit maps to them is too.

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

/// A `claude` that crashes at once, every time.
fn crashing_claude(dir: &Path) -> PathBuf {
    let path = dir.join("claude-crashes");
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

/// Waits for `session`'s exit, each event through the app's own
/// forwarder (which stores it and hands the crash to the restarter).
fn crashed(state: &Arc<AppState>, session: &str) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(Instant::now() < deadline, "{session} never exited");
        if let Some(event) = state.supervisor.recv_timeout(Duration::from_millis(200)) {
            let exited = matches!(&event, juggler_supervisor::LifecycleEvent::Exited { session: id, .. } if id.as_str() == session);
            lifecycle::forward(state, event);
            if exited {
                break;
            }
        }
    }
}

/// Every session, oldest first.
fn sessions(state: &Arc<AppState>) -> Vec<juggler_core::model::InstanceRecord> {
    let mut all = state.storage.lock().unwrap().sessions().list(None).unwrap();
    all.sort_by_key(|record| record.started_at.millis());
    all
}

/// The ids of every session now.
fn ids(state: &Arc<AppState>) -> Vec<String> {
    sessions(state)
        .iter()
        .map(|record| record.id.to_string())
        .collect()
}

/// Waits for a session that isn't one of `seen`, and returns it. Never by
/// position: a restart can start in the same millisecond as the session
/// it replaced, and then their order isn't fixed (#136's Linux run).
fn wait_for_new_session(state: &Arc<AppState>, seen: &[String]) -> String {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(new) = ids(state).into_iter().find(|id| !seen.contains(id)) {
            return new;
        }
        assert!(Instant::now() < deadline, "no new session after {seen:?}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn history(state: &Arc<AppState>, session: &str) -> Vec<String> {
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .lifecycle_events(&SessionId::new(session))
        .unwrap()
        .into_iter()
        .map(|(event, _)| event)
        .collect()
}

/// `session`'s history once it has an event containing `needle` (the
/// replacement is started before its notes are written): at most 10 s.
fn history_with(state: &Arc<AppState>, session: &str, needle: &str) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let said = history(state, session);
        if said.iter().any(|event| event.contains(needle)) || Instant::now() >= deadline {
            return said;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn set_restart(state: &Arc<AppState>, on: bool) {
    let prefs = juggler_app::settings_policy::SessionPrefsInfo {
        restart_on_crash: on,
        ..juggler_app::settings_policy::session_prefs_get_impl(state).unwrap()
    };
    juggler_app::settings_policy::session_prefs_set_impl(state, &prefs).unwrap();
}

#[test]
fn off_by_default_a_crash_stays_ended() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), &crashing_claude(work.path()));
    let profile = launch_settings(&state, work.path());
    let first = commands::launch_profile(&state, &profile, None).unwrap();
    crashed(&state, &first.session_id);
    std::thread::sleep(Duration::from_secs(4));
    let all = sessions(&state);
    assert_eq!(all.len(), 1, "nothing was started again");
    assert_eq!(all[0].status, SessionStatus::Failed);
    assert!(history(&state, &first.session_id).is_empty());
}

#[test]
fn on_it_restarts_three_times_in_ten_minutes_then_says_why() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), &crashing_claude(work.path()));
    let profile = launch_settings(&state, work.path());
    set_restart(&state, true);

    let mut current = commands::launch_profile(&state, &profile, None)
        .unwrap()
        .session_id;
    for attempt in 1..=3 {
        let seen = ids(&state);
        crashed(&state, &current);
        let next = wait_for_new_session(&state, &seen);
        assert_ne!(next, current);
        let said = history_with(&state, &current, "restarted after a crash");
        assert!(
            said.iter()
                .any(|event| event
                    .starts_with(&format!("restarted after a crash (try {attempt} of 3"))),
            "{said:?}"
        );
        assert!(
            history_with(&state, &next, "started again after")
                .iter()
                .any(|event| event.contains(&format!("crashed (try {attempt} of 3)"))),
            "the new session says where it came from"
        );
        // The same launch settings and folder.
        let all = sessions(&state);
        let (old, new) = (
            all.iter().find(|r| r.id.as_str() == current).unwrap(),
            all.iter().find(|r| r.id.as_str() == next).unwrap(),
        );
        assert_eq!(new.profile, old.profile);
        assert_eq!(new.cwd, old.cwd);
        current = next;
    }
    // The fourth crash in ten minutes: it stays ended, and says why.
    crashed(&state, &current);
    std::thread::sleep(Duration::from_secs(4));
    let all = sessions(&state);
    assert_eq!(all.len(), 4, "no fourth try");
    let last = all.iter().find(|r| r.id.as_str() == current).unwrap();
    assert_eq!(last.status, SessionStatus::Failed);
    let reason = last.attention.as_ref().expect("it says why");
    assert_eq!(reason.rule, "restart_gave_up");
    assert!(
        history(&state, &current)
            .iter()
            .any(|event| event.contains("crashed 3 times in 10 minutes"))
    );
    // A restart is history only: it never archives or reviews a session.
    let latest = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .lifecycle_latest()
        .unwrap();
    assert!(latest.is_empty(), "{latest:?}");
}

#[test]
fn a_launch_settings_own_choice_overrides_settings() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path(), &crashing_claude(work.path()));
    let profile = launch_settings(&state, work.path());
    {
        let storage = state.storage.lock().unwrap();
        let mut recipe = storage
            .profiles()
            .get(&juggler_core::model::ProfileId::new(profile.clone()))
            .unwrap();
        recipe.settings_overlay.restart_on_crash = Some(true);
        storage.profiles().update(&recipe).unwrap();
    }
    // Settings › Sessions stays off.
    let first = commands::launch_profile(&state, &profile, None).unwrap();
    let seen = ids(&state);
    crashed(&state, &first.session_id);
    let next = wait_for_new_session(&state, &seen);
    // Stop the replacement so the test ends with nothing running.
    let _ = state
        .supervisor
        .stop(&SessionId::new(next), Duration::from_secs(2));
}

/// A `claude` that runs until it's stopped.
fn steady_claude(dir: &Path) -> PathBuf {
    let path = dir.join("claude-steady");
    stand_in::write(&path, "#!/bin/sh\nexec sleep 60\n");
    path
}

/// AJ-E1509 Req.4: after AI Juggler quits as asked, the sessions that
/// didn't survive are stopped; after it crashed (its run marker still
/// says "running"), they're ended with that reason and, where restarting
/// is on, resumed once it's up.
#[test]
fn after_a_crash_its_sessions_say_so_and_resume_after_a_quit_they_stop() {
    let work = tempfile::tempdir().unwrap();
    let stub = steady_claude(work.path());
    let data = work.path().join("app-data");

    // A quit: the state closes as asked.
    let state = app_state(work.path(), &stub);
    let profile = launch_settings(&state, work.path());
    set_restart(&state, true);
    let quit = commands::launch_profile(&state, &profile, None)
        .unwrap()
        .session_id;
    let _ = state
        .supervisor
        .stop(&SessionId::new(quit.clone()), Duration::from_secs(2));
    {
        // Recorded running as the app went away.
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&SessionId::new(quit.clone()))
            .unwrap();
        record.status = SessionStatus::Running;
        storage.sessions().update(&record).unwrap();
    }
    drop(state);
    let state = app_state(work.path(), &stub);
    let record = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(quit))
        .unwrap();
    assert_eq!(record.status, SessionStatus::Stopped);
    assert!(record.attention.is_none());

    // A crash: the app never got to say it quit.
    let crashed = commands::launch_profile(&state, &profile, None)
        .unwrap()
        .session_id;
    let _ = state
        .supervisor
        .stop(&SessionId::new(crashed.clone()), Duration::from_secs(2));
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&SessionId::new(crashed.clone()))
            .unwrap();
        record.status = SessionStatus::Running;
        storage.sessions().update(&record).unwrap();
    }
    drop(state);
    std::fs::write(data.join("last-run"), "running\n").unwrap();
    let before = {
        let state = app_state(work.path(), &stub);
        let record = state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .get(&SessionId::new(crashed.clone()))
            .unwrap();
        assert_eq!(record.status, SessionStatus::Failed);
        let reason = record.attention.expect("it says why");
        assert_eq!(reason.rule, "app_quit_unexpectedly");
        let before = ids(&state);
        // Up again: restarting is on, so it resumes.
        juggler_app::features::app::crash::resume_after_crash(&state);
        let next = wait_for_new_session(&state, &before);
        // The replacement starts before its note is written: wait for it.
        assert!(
            history_with(&state, &crashed, "restarted after a crash (try 1 of 3")
                .iter()
                .any(|event| event.starts_with("restarted after a crash (try 1 of 3")),
            "the crashed session's history never said it was restarted"
        );
        let _ = state
            .supervisor
            .stop(&SessionId::new(next), Duration::from_secs(2));
        before.len()
    };
    assert!(before >= 2);
}
