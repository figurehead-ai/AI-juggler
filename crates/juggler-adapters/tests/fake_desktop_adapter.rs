//! The fake desktop adapter (AJ-E413), at the adapter (AJ-E1522: adapter
//! coverage): the reference desktop implementation's contract — found or
//! not, single-account and honest about it, desktop launches only, its
//! managed folder prepared, and a watcher that never yields events.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::time::Duration;

use juggler_adapter::{Capabilities, HarnessAdapter, InstallStatus, LaunchMode};
use juggler_adapters::{FAKE_DESKTOP_ID, FakeDesktopAdapter, SINGLE_ACCOUNT_LABEL};
use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId};

fn account(dir: &std::path::Path) -> Account {
    Account {
        id: AccountId::new("desk"),
        harness: HarnessId::new(FAKE_DESKTOP_ID),
        label: "Desk".into(),
        plan: None,
        state_dir: dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

#[test]
fn a_desktop_app_is_found_launched_and_labelled_honestly() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("fake-desktop");
    std::fs::write(&bin, "#!/bin/sh\n").unwrap();
    let adapter = FakeDesktopAdapter::new(&bin);
    assert_eq!(adapter.bin(), bin.as_path());
    assert_eq!(adapter.id().as_str(), FAKE_DESKTOP_ID);
    assert_eq!(adapter.spec().short_name, "Fake");
    assert!(
        adapter
            .capabilities()
            .contains(Capabilities::DESKTOP_APP | Capabilities::DESKTOP_SINGLE_ACCOUNT)
    );
    assert_eq!(adapter.verified_against(), &["0.1.0"]);
    assert_eq!(
        adapter.desktop_isolation_label(),
        Some(SINGLE_ACCOUNT_LABEL)
    );

    let report = adapter.discover();
    assert_eq!(
        report.install,
        InstallStatus::Installed {
            version: "0.1.0".into()
        }
    );
    assert_eq!(report.binary.as_deref(), Some(bin.as_path()));
    assert!(report.notes.is_empty());

    let state = dir.path().join("account");
    std::fs::create_dir_all(&state).unwrap();
    assert!(
        adapter.prepare_account(&state).unwrap().is_empty(),
        "nothing to isolate"
    );
    let launch = adapter.desktop_launch(&account(&state)).unwrap();
    assert_eq!(launch.program, bin);
    assert_eq!(launch.cwd, state);
    assert_eq!(launch.isolation_label, SINGLE_ACCOUNT_LABEL);
    assert!(launch.env.is_empty() && launch.args.is_empty());

    // No CLI engine: a terminal session isn't offered.
    let profile = Profile {
        id: ProfileId::new("p"),
        name: "p".into(),
        account: AccountId::new("desk"),
        settings_overlay: Default::default(),
        default_cwd: None,
    };
    assert!(
        adapter
            .launch_spec(
                &account(&state),
                &profile,
                dir.path(),
                &LaunchMode::Interactive
            )
            .is_err()
    );

    // Its watcher never yields events.
    let mut watcher = adapter.watcher(dir.path()).unwrap();
    assert!(watcher.poll(Duration::from_millis(10)).unwrap().is_none());
}

#[test]
fn a_missing_binary_is_reported_and_never_launched() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = FakeDesktopAdapter::new(dir.path().join("not-built"));
    let report = adapter.discover();
    assert_eq!(report.install, InstallStatus::NotFound);
    assert!(report.binary.is_none());
    assert_eq!(
        report.notes,
        vec!["fake-desktop binary not built".to_owned()]
    );
    let error = adapter.desktop_launch(&account(dir.path())).unwrap_err();
    assert!(
        error.to_string().contains("desktop binary missing"),
        "{error}"
    );
}
