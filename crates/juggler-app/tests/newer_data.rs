//! AJ-E1010 Req.2 — the migration fixture and the way back. An older
//! build can't open data a newer one saved (schema too new): it says so,
//! offers the newest backup it can read, and going back restores that
//! backup while keeping the newer data beside it. A failed go-back puts
//! everything back as it was.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::error::AppError;
use juggler_app::newer_data;
use juggler_app::state::AppState;
use juggler_core::CoreError;
use juggler_core::model::{Account, AccountId, HarnessId};
use juggler_core::storage::Storage;

fn open(dir: std::path::PathBuf) -> Result<AppState, AppError> {
    AppState::init_with_registry(dir, AdapterRegistry::new())
}

fn account(dir: &Path, id: &str, label: &str) -> Account {
    Account {
        id: AccountId::new(id),
        harness: HarnessId::new("claude-code"),
        label: label.into(),
        plan: None,
        state_dir: dir.join("accounts").join(id),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

fn labels(state: &AppState) -> Vec<String> {
    let mut labels: Vec<String> = state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .map(|a| a.label)
        .collect();
    labels.sort();
    labels
}

/// Data as the previous version left it (with its pre-update backup),
/// then "updated": a newer schema and an account the newer version
/// added.
fn updated_data(dir: &Path) -> std::path::PathBuf {
    let backup = dir.join("backups/before-update-0.0.8-to-0.0.9-1.ajbackup");
    {
        let state = Arc::new(open(dir.to_path_buf()).unwrap());
        state
            .storage
            .lock()
            .unwrap()
            .accounts()
            .insert(&account(dir, "work", "Work"))
            .unwrap();
        std::fs::create_dir_all(backup.parent().unwrap()).unwrap();
        juggler_app::backup::backup_export_impl(&state, &backup, &mut |_, _| {}).unwrap();
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&account(dir, "added", "Added by the newer version"))
            .unwrap();
        storage
            .raw_conn()
            .pragma_update(None, "user_version", Storage::schema_version() + 1)
            .unwrap();
    }
    backup
}

#[test]
fn an_older_build_refuses_newer_data_and_offers_the_backup_from_before() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("AI Juggler Dev");
    let backup = updated_data(&dir);

    // The fixture: this build can't open newer data, and says why.
    let refused = open(dir.clone()).err().expect("newer data is refused");
    assert!(
        matches!(
            refused,
            AppError::Core(CoreError::SchemaTooNew { found, supported })
                if found == supported + 1
        ),
        "{refused}"
    );

    let offer = newer_data::newest_restorable(&dir).expect("the pre-update backup");
    assert_eq!(offer.backup, backup);
    let (title, text, button) = newer_data::launch_message("AI Juggler Dev", "0.0.8", Some(&offer));
    assert_eq!(title, "This data is from a newer AI Juggler Dev");
    assert!(text.contains("AI Juggler Dev 0.0.8 can't open"), "{text}");
    assert!(text.contains("nothing is deleted"), "{text}");
    assert!(button.unwrap().starts_with("Use the backup from "));

    let kept = newer_data::go_back(&dir, &offer.backup, &open).unwrap();
    let state = open(dir.clone()).expect("starts on the restored data");
    assert_eq!(labels(&state), ["Work"], "the data from before the update");
    drop(state);
    // The newer data is kept, untouched, beside it.
    let newer = schema_of(&kept);
    assert_eq!(newer, Storage::schema_version() + 1);
}

#[test]
fn a_go_back_that_fails_leaves_the_newer_data_where_it_was() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("data");
    let backup = updated_data(&dir);
    // The backup is damaged after it was written.
    let mut bytes = std::fs::read(&backup).unwrap();
    let middle = bytes.len() / 2;
    bytes.truncate(middle);
    std::fs::write(&backup, bytes).unwrap();

    assert!(newer_data::go_back(&dir, &backup, &open).is_err());
    assert_eq!(
        schema_of(&dir.join("juggler.sqlite")),
        Storage::schema_version() + 1,
        "the newer data is back in place"
    );
    assert!(
        open(dir.clone()).is_err(),
        "still refused, still not changed"
    );
}

#[test]
fn without_a_readable_backup_it_says_what_to_do() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("data");
    let backup = updated_data(&dir);
    std::fs::remove_file(&backup).unwrap();
    assert_eq!(newer_data::newest_restorable(&dir), None);
    let (_, text, button) = newer_data::launch_message("AI Juggler", "0.0.8", None);
    assert_eq!(button, None);
    assert!(text.contains("install the newer version again"), "{text}");
}

fn schema_of(path: &Path) -> i64 {
    // Read without opening it (Storage::open would refuse it): SQLite
    // keeps `user_version` big-endian at byte 60 of the file header. Every
    // connection is closed, so the WAL has been checkpointed into it.
    let header = std::fs::read(path).unwrap();
    i64::from(i32::from_be_bytes(header[60..64].try_into().unwrap()))
}
