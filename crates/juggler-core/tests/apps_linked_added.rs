//! AJ-E1552: a folder link alone never makes a desktop App the user's
//! (`App::is_users`: added, named or kept alive). The first open by a
//! build with that rule marks every desktop row the old rule counted by
//! its link alone `added`, once, so no App the user already had leaves a
//! list; a declined offer stays declined, an unlinked offer stays an
//! offer, and a row linked after that first open is an offer until the
//! user adds it, however often the data is opened again. No schema change:
//! 0.5.0 still opens the data.

#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_core::storage::{App, Storage};
use rusqlite::Connection;

#[test]
fn the_first_open_keeps_every_linked_app_the_users_and_a_link_alone_adds_none_after() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    {
        // An older build's data (its schema; this change adds none), never
        // opened by this build.
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        for (index, sql) in Storage::migrations().iter().enumerate() {
            conn.execute_batch(sql).unwrap();
            conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
                .unwrap();
        }
        conn.execute_batch(
            "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
                 VALUES ('work', 'claude-code', 'Work', NULL, '/u/work', 1);
             INSERT INTO apps (id, kind, tool, host_app_id, instance_dir, folder_id, hidden, first_seen, last_seen)
                 VALUES ('app:claude-app:/u/b-work', 'desktop', 'claude-code', 'claude-app', '/u/b-work', 'work', 0, 1, 1),
                        ('app:claude-app:/u/b-declined', 'desktop', 'claude-code', 'claude-app', '/u/b-declined', 'work', 1, 1, 1),
                        ('app:claude-app:/u/b-seen', 'desktop', 'claude-code', 'claude-app', '/u/b-seen', NULL, 0, 1, 1);",
        )
        .unwrap();
    }
    let get = |storage: &Storage, id: &str| -> App { storage.apps().get(id).unwrap().unwrap() };
    let linked = {
        let storage = Storage::open(&path).unwrap();
        let work = get(&storage, "app:claude-app:/u/b-work");
        assert!(
            work.added && work.is_users(),
            "linked before: still the user's"
        );
        let declined = get(&storage, "app:claude-app:/u/b-declined");
        assert!(
            !declined.added && !declined.is_users(),
            "a declined offer stays declined"
        );
        let seen = get(&storage, "app:claude-app:/u/b-seen");
        assert!(!seen.added && !seen.is_users(), "never linked: an offer");

        // From now on a link alone adds nothing.
        let apps = storage.apps();
        let linked = apps
            .upsert_desktop("claude-code", "claude-app", Some("/u/b-new"), 2)
            .unwrap();
        apps.set_folder(&linked, Some("work")).unwrap();
        assert!(!get(&storage, &linked).is_users(), "linked after: an offer");
        linked
    };
    // Opened again (the next launch): the marking ran once, never again.
    let storage = Storage::open(&path).unwrap();
    assert!(
        !get(&storage, &linked).is_users(),
        "linked after, reopened: still an offer"
    );
    let apps = storage.apps();
    apps.rename(&linked, Some("Lab")).unwrap();
    assert!(get(&storage, &linked).is_users(), "named: the user's");
    apps.rename(&linked, None).unwrap();
    apps.set_keep_alive(&linked, true, None).unwrap();
    assert!(get(&storage, &linked).is_users(), "kept alive: the user's");
    apps.set_keep_alive(&linked, false, None).unwrap();
    assert!(apps.add(&linked).unwrap());
    assert!(get(&storage, &linked).is_users(), "added: the user's");
}

/// A new database has nothing to keep: nothing linked later is added.
#[test]
fn a_new_database_marks_nothing_linked_later() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let linked = {
        let storage = Storage::open(&path).unwrap();
        storage
            .raw_conn()
            .execute_batch(
                "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
                     VALUES ('work', 'claude-code', 'Work', NULL, '/u/work', 1);",
            )
            .unwrap();
        let apps = storage.apps();
        let linked = apps
            .upsert_desktop("claude-code", "claude-app", Some("/u/b-new"), 2)
            .unwrap();
        apps.set_folder(&linked, Some("work")).unwrap();
        linked
    };
    let storage = Storage::open(&path).unwrap();
    let app = storage.apps().get(&linked).unwrap().unwrap();
    assert!(
        !app.added && !app.is_users(),
        "a link alone, on a new database"
    );
}

/// `put_desktop` (the places book's write) never adds an App, and never
/// takes the user's Add away: a new row takes `added` as given, an
/// existing one keeps its own.
#[test]
fn the_books_write_keeps_added_as_the_user_left_it() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    let row = |id: &str, added: bool| App {
        id: id.to_owned(),
        kind: "desktop".into(),
        tool: "claude-code".into(),
        host_app_id: Some("claude-app".into()),
        instance_dir: Some(format!("/u/{id}")),
        folder_id: None,
        label: None,
        hidden: false,
        keep_alive: false,
        paused_json: None,
        launch_json: None,
        sessions_json: None,
        first_seen: 1,
        last_seen: 1,
        added,
    };
    let apps = storage.apps();
    apps.put_desktop(&row("offer", false)).unwrap();
    apps.put_desktop(&row("mine", true)).unwrap();
    assert!(!apps.get("offer").unwrap().unwrap().added, "an offer");
    assert!(apps.get("mine").unwrap().unwrap().added, "added as given");
    apps.put_desktop(&row("mine", false)).unwrap();
    assert!(
        apps.get("mine").unwrap().unwrap().added,
        "the book's next write keeps the user's Add"
    );
    apps.put_desktop(&row("offer", true)).unwrap();
    assert!(
        !apps.get("offer").unwrap().unwrap().added,
        "only the user's Add adds an existing row"
    );
}
