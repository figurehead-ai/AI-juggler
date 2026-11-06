//! D-84 (v47): what 0.4.0's upgrade got wrong in the desktop Apps' rows.
//!
//! - Every row gets `added` (0: the user hasn't said so; the app decides
//!   from the folder link, keep alive or a name whether it's theirs).
//! - A profile under the system's temporary folder (a test's leaked
//!   "Lab Claude profile") is dropped, and its sessions unlinked.
//! - A row linked to an imported folder keeps the earliest first-seen
//!   it can show: the move from 0.3's book reset it to the upgrade's
//!   time, and the folder's own creation bounds it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_core::storage::Storage;
use rusqlite::Connection;

/// 0.4.0's schema: the version before D-84's.
const SCHEMA_0_4_0: usize = 46;

const UPGRADE: i64 = 1_793_960_844_947;
const IMPORTED: i64 = 1_792_609_402_233;

#[test]
fn the_upgrade_drops_temporary_profiles_and_keeps_first_seen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    {
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        for (index, sql) in Storage::migrations().iter().enumerate().take(SCHEMA_0_4_0) {
            conn.execute_batch(sql).unwrap();
            conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
                .unwrap();
        }
        conn.execute_batch(&format!(
            "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
                 VALUES ('work', 'claude-code', 'Work Max', NULL, '/u/work', {IMPORTED});
             INSERT INTO launch_recipes (id, account_id, settings_json, default_cwd, created_at, name)
                 VALUES ('rec-work', 'work', '{{}}', NULL, 1, 'Main');
             INSERT INTO apps (id, kind, tool, host_app_id, instance_dir, folder_id, first_seen, last_seen)
                 VALUES ('app:claude-app:/u/b-work', 'desktop', 'claude-code', 'claude-app', '/u/b-work', 'work', {UPGRADE}, {UPGRADE}),
                        ('app:claude-app:/u/aij-test', 'desktop', 'claude-code', 'claude-app', '/u/aij-test', NULL, {UPGRADE}, {UPGRADE}),
                        ('app:claude-app:/private/var/folders/s7/x/T/.tmpLab', 'desktop', 'claude-code', 'claude-app',
                         '/private/var/folders/s7/x/T/.tmpLab', NULL, 5, 5);
             INSERT INTO instances (id, conversation_id, launch_recipe_id, machine, cwd, status,
                                    started_at, last_activity, usage_json, kind, app_id)
                 VALUES ('s-lab', NULL, 'rec-work', 'this-mac', '/tmp/lab', 'ended', 1, 1, '{{}}', 'terminal',
                         'app:claude-app:/private/var/folders/s7/x/T/.tmpLab');"
        ))
        .unwrap();
    }
    let storage = Storage::open(&path).unwrap();
    let apps = storage.apps();
    let work = apps.get("app:claude-app:/u/b-work").unwrap().unwrap();
    assert_eq!(work.first_seen, IMPORTED, "bounded by the folder's import");
    assert!(work.is_users(), "linked to an imported folder: the user's");
    let test = apps.get("app:claude-app:/u/aij-test").unwrap().unwrap();
    assert_eq!(test.first_seen, UPGRADE, "no folder: nothing to bound it");
    assert!(!test.added && !test.is_users(), "never added: an offer");
    assert!(
        apps.get("app:claude-app:/private/var/folders/s7/x/T/.tmpLab")
            .unwrap()
            .is_none(),
        "a temporary profile is dropped"
    );
    drop(storage);
    let conn = Connection::open(&path).unwrap();
    let linked: Option<String> = conn
        .query_row(
            "SELECT app_id FROM instances WHERE id = 's-lab'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(linked, None, "its session no longer points at it");
}
