//! AJ-E1505 integration: Settings › Sessions. "Stuck after" reaches the
//! running heuristics at once and survives a restart; the terminal's font
//! size and scrollback are kept; each has a range and resets to its
//! default.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::settings_policy::{self, SessionPrefsInfo};
use juggler_app::state::AppState;

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

#[test]
fn stuck_after_applies_live_survives_a_restart_and_resets() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    assert_eq!(
        settings_policy::session_prefs_get_impl(&state).unwrap(),
        SessionPrefsInfo::default()
    );
    assert_eq!(state.heuristics.thresholds().stuck_no_output_secs, 300);

    let quiet_builds = SessionPrefsInfo {
        stuck_after_minutes: 10,
        terminal_font_size: 14.0,
        terminal_scrollback_lines: 20_000,
    };
    let saved = settings_policy::session_prefs_set_impl(&state, &quiet_builds).unwrap();
    assert_eq!(saved, quiet_builds);
    // Live: the same engine, no restart.
    let thresholds = state.heuristics.thresholds();
    assert_eq!(thresholds.stuck_no_output_secs, 600);
    assert_eq!(thresholds.stuck_transcript_stale_secs, 600);
    assert_eq!(
        thresholds.waiting_prompt_secs, 120,
        "the other thresholds stay"
    );

    drop(state);
    let state = app_state(work.path());
    assert_eq!(
        settings_policy::session_prefs_get_impl(&state).unwrap(),
        quiet_builds
    );
    assert_eq!(state.heuristics.thresholds().stuck_no_output_secs, 600);

    // Reset: the defaults, saved.
    let reset =
        settings_policy::session_prefs_set_impl(&state, &SessionPrefsInfo::default()).unwrap();
    assert_eq!(reset, SessionPrefsInfo::default());
    assert_eq!(state.heuristics.thresholds().stuck_no_output_secs, 300);
}

#[test]
fn each_value_keeps_to_its_range() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let base = SessionPrefsInfo::default();
    for (prefs, words) in [
        (
            SessionPrefsInfo {
                stuck_after_minutes: 0,
                ..base.clone()
            },
            "1 to 120 minutes",
        ),
        (
            SessionPrefsInfo {
                stuck_after_minutes: 121,
                ..base.clone()
            },
            "1 to 120 minutes",
        ),
        (
            SessionPrefsInfo {
                terminal_font_size: 8.5,
                ..base.clone()
            },
            "9 to 24 points",
        ),
        (
            SessionPrefsInfo {
                terminal_scrollback_lines: 100,
                ..base.clone()
            },
            "500 to 100000 lines",
        ),
    ] {
        let refused = settings_policy::session_prefs_set_impl(&state, &prefs).unwrap_err();
        assert!(refused.to_string().contains(words), "{refused}");
    }
    // Nothing was half-saved.
    assert_eq!(
        settings_policy::session_prefs_get_impl(&state).unwrap(),
        base
    );
    assert_eq!(state.heuristics.thresholds().stuck_no_output_secs, 300);
}
