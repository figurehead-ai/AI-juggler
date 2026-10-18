//! AJ-E411 app-level: the renewal-date command (D-12 validation —
//! honest rejection, never a repair) and its AccountInfo round-trip.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::{Account, AccountId};

fn state_with_account(dir: &Path) -> Arc<AppState> {
    let state = Arc::new(
        AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-renewal"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Work Max".into(),
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
    }
    state
}

#[test]
fn renewal_set_validate_clear_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let state_ref = &state_with_account(dir.path());
    // No date yet: unknown, never a guess (storage read; the command
    // surface is covered by the arc calls below).
    {
        let storage = state_ref.storage.lock().unwrap();
        assert_eq!(
            storage
                .accounts()
                .get(&AccountId::new("acc-renewal"))
                .unwrap()
                .renewal_date,
            None
        );
    }

    // Valid date round-trips.
    let info =
        commands::account_set_renewal_arc(state_ref, "acc-renewal", Some("2026-11-02")).unwrap();
    assert_eq!(info.renewal_date.as_deref(), Some("2026-11-02"));
    {
        let storage = state_ref.storage.lock().unwrap();
        assert_eq!(
            storage
                .accounts()
                .get(&AccountId::new("acc-renewal"))
                .unwrap()
                .renewal_date,
            Some("2026-11-02".into())
        );
    }

    // Invalid formats are rejected — honest rejection, never a repair.
    for bad in [
        "2026/11/02",
        "11-02-2026",
        "2026-13-01",
        "2026-02-30",
        "not-a-date",
        "2026-11-02T00:00",
    ] {
        let error =
            commands::account_set_renewal_arc(state_ref, "acc-renewal", Some(bad)).unwrap_err();
        assert!(
            error.to_string().contains("YYYY-MM-DD"),
            "format {bad:?} should be rejected: {error}"
        );
    }

    // Clear works.
    let info = commands::account_set_renewal_arc(state_ref, "acc-renewal", None).unwrap();
    assert_eq!(info.renewal_date, None);
}

#[test]
fn renewal_unknown_account_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let state_ref = &state_with_account(dir.path());
    let error =
        commands::account_set_renewal_arc(state_ref, "ghost", Some("2026-11-02")).unwrap_err();
    assert!(error.to_string().contains("ghost"));
}
