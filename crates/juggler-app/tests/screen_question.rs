//! The rc29 persona walks: a Claude Code session at its own first-run
//! screen sat in Needs you as "It waits on its own screen in its
//! terminal, before its session opens: answer it there", though the
//! screen asks "Choose the text style that looks best with your
//! terminal". Its first report overdue, the session's ask is what its
//! adapter reads that screen asking (`ToolConfig::screen_ask`), stored
//! for its row and shown on its runtime alike.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test: a failed setup step is the failure"
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::features::runtime::reports;
use juggler_app::state::AppState;
use juggler_core::model::{SessionId, Timestamp};

#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// A Claude Code stand-in that draws `screen` the way a terminal UI
/// does, then waits there.
fn claude_at(dir: &Path, name: &str, screen: &str) -> PathBuf {
    let path = dir.join(name);
    stand_in::write(&path, format!("#!/bin/sh\nprintf '{screen}'\nsleep 30\n"));
    path
}

fn state_with(dir: &Path, bin: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(bin));
    registry.register(claude.clone());
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// Starts a session on the stand-in, waits for its screen, and lets its
/// first report fall overdue: what it asks, stored and shown.
fn overdue_ask(screen: &str, shows: &str) -> (String, String) {
    let work = tempfile::tempdir().unwrap();
    let bin = claude_at(work.path(), "claude-stub", screen);
    let state = state_with(work.path(), &bin);
    let account =
        juggler_app::features::account::create_account(&state, CLAUDE_CODE_ID, "Work", None)
            .unwrap();
    let recipe =
        juggler_app::features::profile::create_profile(&state, &account.id, "Main", None, None)
            .unwrap();
    let cwd = work.path().to_string_lossy().into_owned();
    let started =
        juggler_app::features::launch::launch_profile(&state, &recipe.id, Some(cwd)).unwrap();
    let session = SessionId::new(started.session_id.as_str());
    let deadline = Instant::now() + Duration::from_secs(10);
    while !state
        .supervisor
        .output_tail(&session, 8192)
        .is_some_and(|tail| tail.contains(shows))
    {
        assert!(Instant::now() < deadline, "the stand-in drew nothing");
        std::thread::sleep(Duration::from_millis(50));
    }
    let now = Timestamp::now().millis();
    state.runtime.expect_reports(&session, now);
    reports::overdue_tick(&state, now + reports::FIRST_REPORT_MS);
    let stored = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&session)
        .unwrap()
        .attention
        .unwrap()
        .inputs[0]
        .clone();
    let shown = state.runtime.info(session.as_str()).unwrap().asks[0]
        .summary
        .clone();
    let _ = state.supervisor.stop(&session, Duration::from_secs(2));
    (stored, shown)
}

#[test]
fn a_session_at_its_tools_own_question_says_what_it_asks() {
    let theme = "\\033[2J\\033[1;1HLet'\"'\"'s get started.\\r\\n\\033[1mChoose\\033[0m the text style that looks best with your terminal\\r\\nTo change this later, run /theme\\r\\n\\033[36m> 1. Dark mode\\033[0m\\r\\n  2. Light mode\\r\\n";
    let question = "Choose the text style that looks best with your terminal";
    assert_eq!(
        overdue_ask(theme, "text style"),
        (question.to_owned(), question.to_owned())
    );
    // A screen its adapter doesn't know: short plain words, never a guess.
    let other = "Press Enter to continue\\r\\n";
    let (stored, shown) = overdue_ask(other, "Press Enter");
    assert_eq!(stored, reports::WAITS_IN_TERMINAL);
    assert_eq!(shown, reports::WAITS_IN_TERMINAL);
}
