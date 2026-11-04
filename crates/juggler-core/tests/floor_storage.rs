//! AJ-E3503 storage: the Floor's areas (schema v46).
//!
//! - Upgrading from v45 keeps a copy as it was, adds an empty
//!   `floor_areas` and keeps every table.
//! - Areas save in one transaction (an unknown App saves none), read back
//!   in order, keep the user's "hidden" through later saves, and leave
//!   with their App.
//! - Re-create writes the new seed and every look together, keeping the
//!   order and sizes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;

use juggler_core::storage::{FLOOR_SEED_KEY, FloorArea, Storage, pre_migration_copies};
use rusqlite::{Connection, OpenFlags};

const BEFORE: i64 = 45;

fn database_at_v45(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations()
        .iter()
        .enumerate()
        .take(usize::try_from(BEFORE).unwrap())
    {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
            .unwrap();
    }
    conn.execute_batch(
        "INSERT INTO apps (id, kind, tool, first_seen, last_seen) VALUES
             ('app-a', 'cli', 'tool-a', 1, 1), ('app-b', 'cli', 'tool-b', 1, 1);",
    )
    .unwrap();
}

fn tables(path: &Path) -> Vec<String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn area(app: &str, order: u32, desks: u32) -> FloorArea {
    FloorArea {
        app_id: app.into(),
        order,
        family: "workshop".into(),
        layout: "pods".into(),
        seed: 7,
        desks,
        fits_smaller_since: None,
        hidden: false,
    }
}

#[test]
fn upgrading_adds_an_empty_floor_and_keeps_a_copy_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at_v45(&db);
    let before = tables(&db);
    assert!(!before.contains(&"floor_areas".to_owned()));

    let storage = Storage::open(&db).unwrap();
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    assert_eq!(Storage::schema_version(), 46);
    assert!(storage.floor().areas().unwrap().is_empty());
    let after = tables(&db);
    assert!(after.contains(&"floor_areas".to_owned()));
    assert!(before.iter().all(|t| after.contains(t)), "every table kept");
    let copies = pre_migration_copies(&db);
    assert_eq!(copies[0].0, BEFORE);
    assert_eq!(tables(&copies[0].1), before);
}

#[test]
fn areas_save_together_read_in_order_and_leave_with_their_app() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at_v45(&db);
    let storage = Storage::open(&db).unwrap();
    let floor = storage.floor();

    floor
        .save(&[area("app-b", 0, 6), area("app-a", 1, 12)])
        .unwrap();
    let ids: Vec<String> = floor
        .areas()
        .unwrap()
        .into_iter()
        .map(|a| a.app_id)
        .collect();
    assert_eq!(ids, ["app-b", "app-a"]);

    // An unknown App: nothing of the batch is saved.
    let refused = floor.save(&[
        FloorArea {
            desks: 24,
            ..area("app-a", 1, 24)
        },
        area("ghost", 2, 6),
    ]);
    assert!(refused.is_err());
    assert_eq!(floor.areas().unwrap()[1].desks, 12);
    assert!(floor.save(&[area("app-a", 1, 0)]).is_err(), "no desks");

    // Hidden is the user's: a later save keeps it.
    assert!(floor.set_hidden("app-a", true).unwrap());
    floor
        .save(&[FloorArea {
            desks: 24,
            fits_smaller_since: Some(99),
            ..area("app-a", 1, 24)
        }])
        .unwrap();
    let a = floor
        .areas()
        .unwrap()
        .into_iter()
        .find(|a| a.app_id == "app-a")
        .unwrap();
    assert!(a.hidden);
    assert_eq!((a.desks, a.fits_smaller_since), (24, Some(99)));

    let other = Connection::open(&db).unwrap();
    other.pragma_update(None, "foreign_keys", "ON").unwrap();
    other
        .execute("DELETE FROM apps WHERE id = 'app-b'", [])
        .unwrap();
    let ids: Vec<String> = storage
        .floor()
        .areas()
        .unwrap()
        .into_iter()
        .map(|a| a.app_id)
        .collect();
    assert_eq!(ids, ["app-a"]);
}

#[test]
fn re_create_writes_the_seed_and_every_look_together() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at_v45(&db);
    let storage = Storage::open(&db).unwrap();
    storage
        .floor()
        .save(&[area("app-a", 0, 6), area("app-b", 1, 12)])
        .unwrap();
    storage
        .floor()
        .recreate(
            4_000_000_000,
            &[
                ("app-a".into(), "bench".into(), 11),
                ("app-b".into(), "radial".into(), 4_294_967_295),
            ],
        )
        .unwrap();
    assert_eq!(
        storage
            .app_settings()
            .get(FLOOR_SEED_KEY)
            .unwrap()
            .as_deref(),
        Some("4000000000")
    );
    let areas = storage.floor().areas().unwrap();
    assert_eq!(
        areas
            .iter()
            .map(|a| (
                a.app_id.as_str(),
                a.order,
                a.layout.as_str(),
                a.seed,
                a.desks
            ))
            .collect::<Vec<_>>(),
        [
            ("app-a", 0, "bench", 11, 6),
            ("app-b", 1, "radial", 4_294_967_295, 12)
        ]
    );
}
