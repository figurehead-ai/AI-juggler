//! D-25: the OS access step's backend. Only the paths that never prompt
//! are exercised here — a test that pressed Allow would raise a real
//! system dialog on the machine running it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::os_access;
use juggler_app::state::AppState;

fn app_state(dir: &std::path::Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.to_path_buf(), AdapterRegistry::new()).unwrap())
}

#[test]
fn the_list_is_per_platform_and_every_entry_says_why_and_what_next() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = os_access::os_access_list_impl(&state);
    if cfg!(target_os = "macos") {
        let ids: Vec<&str> = list.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, ["focus", "preview"]);
    } else {
        assert!(list.is_empty(), "no OS access step where nothing is needed");
    }
    for entry in &list {
        assert!(!entry.title.is_empty() && !entry.why.is_empty() && !entry.system_name.is_empty());
        assert!(["allowed", "not_allowed", "not_asked"].contains(&entry.status.as_str()));
        assert!(["prompt", "settings", "none"].contains(&entry.action.as_str()));
        assert_eq!(entry.action == "none", entry.status == "allowed");
    }
}

#[test]
fn once_asked_and_still_not_allowed_the_way_forward_is_the_settings_pane() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    // As if Allow had been pressed and the user declined (no prompt here).
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set("os_access_asked:focus", "1")
        .unwrap();
    for entry in os_access::os_access_list_impl(&state) {
        if entry.id == "focus" && entry.status != "allowed" {
            assert_eq!(entry.status, "not_allowed");
            assert_eq!(entry.action, "settings");
        }
    }
}

#[test]
fn an_id_this_platform_does_not_list_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let refused = os_access::os_access_request_impl(&state, "full-disk").unwrap_err();
    assert!(refused.to_string().contains("isn't needed"), "{refused}");
}
