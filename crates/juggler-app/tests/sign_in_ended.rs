//! AJ-E1612 (the dry run, 2026-11-16): when an App's Sign in terminal
//! ends, its sign-in is read again at once and Needs you drops "needs its
//! sign-in", not when the last check of the accounts ages out (Needs you
//! reuses that check for half a minute, so Home and the rail said "lab A
//! needs its sign-in" after Kimi Code's sign-in had finished).

#![allow(clippy::unwrap_used, reason = "a test: a failed unwrap is the failure")]

use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{KIMI_CODE_ID, KimiCodeAdapter};
use juggler_app::features::notifications::{centre, needs, tray, views};
use juggler_app::state::AppState;
use juggler_core::model::Timestamp;

fn sign_in_needs(state: &Arc<AppState>) -> Vec<String> {
    let storage = state.storage.lock().unwrap();
    centre::open_needs(&storage, Timestamp::now().millis(), tray::NEEDS_LISTED)
        .into_iter()
        .filter(|row| row.kind == "signin")
        .map(|row| row.subject)
        .collect()
}

#[test]
fn a_sign_in_that_ends_takes_its_need_away_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("vendor-home");
    let mut registry = AdapterRegistry::new();
    // Kimi Code's sign-in is read from its files only (no Keychain).
    registry.register(Arc::new(
        KimiCodeAdapter::with_bin("/usr/bin/true").with_home(&home),
    ));
    let state =
        Arc::new(AppState::init_with_registry(dir.path().join("app-data"), registry).unwrap());
    let app = juggler_app::features::account::create_account(&state, KIMI_CODE_ID, "lab A", None)
        .unwrap();
    let state_dir = state.accounts_dir().join(KIMI_CODE_ID).join(&app.id);

    needs::refresh(&state);
    let subject = format!("signin:{}", app.id);
    assert_eq!(
        sign_in_needs(&state),
        vec![subject.clone()],
        "not signed in yet"
    );

    // The tool's own sign-in writes its membership file (2.5's name), as
    // the lab stand-in does.
    let credentials = state_dir.join("credentials");
    std::fs::create_dir_all(&credentials).unwrap();
    std::fs::write(
        credentials.join("kimi-code-env-0123456789abcdef.json"),
        "{}",
    )
    .unwrap();

    // The tick alone reuses its recent check: the need would stay for up
    // to half a minute.
    needs::refresh(&state);
    assert_eq!(
        sign_in_needs(&state),
        vec![subject.clone()],
        "the tick reuses its last check (why the sign-in's end must re-read it)"
    );

    // The Sign in terminal's end re-reads it now.
    let before = views::summary_now(&state);
    assert!(
        needs::sign_in_ended(&state),
        "Needs you changed, so the views are told"
    );
    assert!(
        sign_in_needs(&state).is_empty(),
        "signed in: no sign-in need"
    );
    // Dry run 2: the bell followed but the rail kept the row. The views are
    // told the summary; the rail (and every list) reads Needs you again
    // only when its version moves as the window reads it (an f64).
    let after = views::summary_now(&state);
    assert_eq!(after.count + 1, before.count, "the bell's count follows");
    assert_ne!(
        as_window(before.version),
        as_window(after.version),
        "the window sees the version move, so the rail reads its list again"
    );
}

/// A number as the window reads it (JSON into JavaScript: an f64).
fn as_window(version: i64) -> u64 {
    let json = serde_json::to_string(&version).unwrap();
    serde_json::from_str::<f64>(&json).unwrap().to_bits()
}

#[test]
fn only_a_sign_in_terminal_counts_as_one() {
    assert!(juggler_app::terminal::is_login_terminal("login-7f3a"));
    assert!(!juggler_app::terminal::is_login_terminal("view-7f3a"));
    assert!(!juggler_app::terminal::is_login_terminal("s-login-7f3a"));
}
