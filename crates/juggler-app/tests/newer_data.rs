//! AJ-E1010 Req.2 — the migration fixture and the ways back.
//!
//! - An older build can't open data a newer one saved (schema too new): it
//!   says so, offers the newest thing it can read — the copy kept before
//!   the upgrade, or a backup — and going back puts it in place while
//!   keeping the newer data beside it. A failed go-back puts everything
//!   back as it was.
//! - The newer build offers "Go back to <version>" from the copy it kept
//!   before upgrading: the copy replaces the data as it is (the older
//!   build opens it), the data as it was is kept, and it waits while
//!   something would be cut short.

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
    assert_eq!(offer.source, backup);
    assert_eq!(offer.kind, newer_data::Source::Backup);
    let (title, text, button) = newer_data::launch_message("AI Juggler Dev", "0.0.8", Some(&offer));
    assert_eq!(title, "This data is from a newer AI Juggler Dev");
    assert!(text.contains("AI Juggler Dev 0.0.8 can't open"), "{text}");
    assert!(text.contains("nothing is deleted"), "{text}");
    assert!(button.unwrap().starts_with("Use the backup from "));

    let kept = newer_data::go_back(&dir, &offer, &open).unwrap();
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

    let offer = newer_data::newest_restorable(&dir).expect("offered before it's tried");
    assert_eq!(offer.source, backup);
    assert!(newer_data::go_back(&dir, &offer, &open).is_err());
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

/// The newer build migrated this data and kept the copy from before;
/// then a still newer one moved it on. Going back from the launch uses
/// that copy: the data exactly as it was before the upgrade.
#[test]
fn the_launch_offers_the_copy_kept_before_the_upgrade() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("data");
    {
        let state = open(dir.clone()).unwrap();
        state
            .storage
            .lock()
            .unwrap()
            .accounts()
            .insert(&account(&dir, "work", "Work"))
            .unwrap();
        // As a newer build leaves it: the copy of this schema, then a newer one.
        let storage = state.storage.lock().unwrap();
        storage
            .snapshot_to(&dir.join(format!(
                "juggler.sqlite.before-v{}",
                Storage::schema_version()
            )))
            .unwrap();
        storage
            .raw_conn()
            .pragma_update(None, "user_version", Storage::schema_version() + 1)
            .unwrap();
    }
    let offer = newer_data::newest_restorable(&dir).expect("the copy is offered");
    assert_eq!(offer.kind, newer_data::Source::Copy);
    let (_, text, button) = newer_data::launch_message("AI Juggler", "0.0.9", Some(&offer));
    assert!(text.contains("just before it was upgraded"), "{text}");
    assert!(button.unwrap().starts_with("Use the data from "));
    let kept = newer_data::go_back(&dir, &offer, &open).unwrap();
    assert_eq!(schema_of(&kept), Storage::schema_version() + 1);
    let state = open(dir.clone()).expect("starts on the copy");
    assert_eq!(labels(&state), ["Work"]);
}

/// A database exactly as schema `version` left it, with one account.
fn database_at(path: &Path, version: i64) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations()
        .iter()
        .enumerate()
        .take(usize::try_from(version).unwrap())
    {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
            .unwrap();
    }
    conn.execute(
        "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
         VALUES ('work', 'claude-code', 'Work', NULL, '/tmp/work', 111)",
        [],
    )
    .unwrap();
}

/// The schema and account labels, read as an older build would (no
/// migration).
fn as_older_build_reads(path: &Path) -> (i64, Vec<String>) {
    let conn = rusqlite::Connection::open(path).unwrap();
    let version = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    let mut labels: Vec<String> = conn
        .prepare("SELECT label FROM accounts")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    labels.sort();
    (version, labels)
}

#[test]
fn the_newer_build_prepares_going_back_to_the_version_before() {
    let previous = Storage::schema_version() - 1;
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("data");
    database_at(&dir.join("juggler.sqlite"), previous);

    // The update: this build opens (and upgrades) it, then work goes on.
    let state = Arc::new(open(dir.clone()).unwrap());
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&account(&dir, "added", "Added after the update"))
        .unwrap();
    let options = newer_data::going_back_options_impl(&state).unwrap();
    assert_eq!(options.len(), 1, "{options:?}");
    assert_eq!(options[0].schema, previous);
    assert_eq!(
        options[0].version.as_deref(),
        newer_data::version_reading(previous)
    );

    // Not while something would be cut short.
    {
        let _bringing_in = state.updates.bring_in();
        let refused = newer_data::going_back_prepare_impl(&state, previous)
            .unwrap_err()
            .to_string();
        assert!(refused.contains("being brought in"), "{refused}");
    }

    // Seconds after the bring-in, nothing would be cut short.
    let later = juggler_core::model::Timestamp::now().millis() + 11_000;
    assert_eq!(juggler_app::updates::busy_at(&state, later).unwrap(), None);
    let refused = newer_data::going_back_prepare_at(&state, previous + 7, later)
        .unwrap_err()
        .to_string();
    assert!(refused.contains("no copy"), "{refused}");
    let kept = newer_data::going_back_prepare_at(&state, previous, later).unwrap();
    drop(state);

    // What the older build finds: its own schema, the data from before.
    assert_eq!(
        as_older_build_reads(&dir.join("juggler.sqlite")),
        (previous, vec!["Work".to_owned()])
    );
    // What was there is kept, untouched.
    assert_eq!(
        as_older_build_reads(&kept),
        (
            Storage::schema_version(),
            vec!["Added after the update".to_owned(), "Work".to_owned()]
        )
    );
}
