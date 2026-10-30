//! AJ-E2501 (schema v37): the notification model's history. An upgrade
//! from v36 keeps a copy and everything as it was; a need is open once per
//! key; what happened is told once; pages walk the history once, newest
//! first; 30 days are kept, never an open need; mutes come and go.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;

use juggler_core::storage::{NOTICE_KEEP_MS, NewNotice, Storage, Written, pre_migration_copies};
use rusqlite::Connection;

fn database_at(path: &Path, upto: i64) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations()
        .iter()
        .enumerate()
        .take(usize::try_from(upto).unwrap())
    {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
            .unwrap();
    }
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES ('notify.waiting', 'true')",
        [],
    )
    .unwrap();
}

fn tables(path: &Path) -> Vec<String> {
    let conn = Connection::open(path).unwrap();
    conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn need(key: &str, title: &str) -> NewNotice {
    NewNotice {
        key: key.to_owned(),
        kind: "asks".to_owned(),
        origin: "needs".to_owned(),
        needs: true,
        title: title.to_owned(),
        detail: "docs-site".to_owned(),
        project_id: Some("proj-1".to_owned()),
        project: Some("docs-site".to_owned()),
        data: "{}".to_owned(),
        action: Some("open".to_owned()),
        meter: None,
    }
}

fn happened(key: &str) -> NewNotice {
    NewNotice {
        key: key.to_owned(),
        kind: "finished".to_owned(),
        origin: "app".to_owned(),
        title: "A session finished".to_owned(),
        ..NewNotice::default()
    }
}

fn storage() -> (tempfile::TempDir, Storage) {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    (dir, storage)
}

#[test]
fn upgrading_from_tasks_keeps_a_copy_and_everything_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, 36);
    assert!(!tables(&db).contains(&"notices".to_owned()));

    let storage = Storage::open(&db).unwrap();
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    assert!(Storage::schema_version() >= 37, "v37 is the notices'");
    assert_eq!(pre_migration_copies(&db)[0].0, 36);
    let after = tables(&db);
    assert!(after.contains(&"notices".to_owned()));
    assert!(after.contains(&"notice_mutes".to_owned()));
    assert!(after.contains(&"need_snoozes".to_owned()));
    // What v36 held is still there.
    assert_eq!(
        storage
            .app_settings()
            .get("notify.waiting")
            .unwrap()
            .as_deref(),
        Some("true")
    );
    assert_eq!(storage.notices().open_count().unwrap(), 0);
}

#[test]
fn a_need_is_open_once_per_key_and_keeps_the_users_marks() {
    let (_dir, storage) = storage();
    let repo = storage.notices();
    let first = repo
        .open(&need("asks:s1", "“Docs bot” needs your reply"), 10)
        .unwrap();
    assert!(matches!(first, Written::New(_)));
    assert_eq!(
        repo.open(&need("asks:s1", "“Docs bot” needs your reply"), 20)
            .unwrap(),
        Written::Same(first.id())
    );
    repo.mark_read(first.id(), 21).unwrap();
    let changed = repo
        .open(&need("asks:s1", "“Docs bot” still needs your reply"), 30)
        .unwrap();
    assert_eq!(changed, Written::Changed(first.id()));
    let row = repo.get(first.id()).unwrap().unwrap();
    assert_eq!(row.read_at, Some(21));
    assert_eq!(row.created_at, 10);
    assert_eq!(row.updated_at, 30);
    assert_eq!(repo.open_count().unwrap(), 1);

    // Ended, then needed again: a new (unread) row.
    assert!(repo.resolve("asks:s1", 40).unwrap());
    assert_eq!(repo.open_count().unwrap(), 0);
    let again = repo
        .open(&need("asks:s1", "“Docs bot” needs your reply"), 50)
        .unwrap();
    assert!(matches!(again, Written::New(id) if id != first.id()));
    assert_eq!(repo.get(again.id()).unwrap().unwrap().read_at, None);
}

#[test]
fn needs_no_longer_listed_end_and_only_their_origins() {
    let (_dir, storage) = storage();
    let repo = storage.notices();
    repo.open(&need("asks:a", "A"), 1).unwrap();
    repo.open(&need("asks:b", "B"), 1).unwrap();
    let mut quota = need("quota:acct", "Work Max is almost out of room");
    quota.origin = "app".to_owned();
    repo.open(&quota, 1).unwrap();
    assert_eq!(
        repo.resolve_missing("needs", &["asks:b".to_owned()], 2)
            .unwrap(),
        1
    );
    let open: Vec<String> = repo
        .open_needs()
        .unwrap()
        .into_iter()
        .map(|row| row.key)
        .collect();
    assert_eq!(open, ["asks:b", "quota:acct"]);
}

/// D-49: a snooze is the subject's, not a row's: it outlasts a need that
/// ends and comes back, and it ends by itself (or when woken).
#[test]
fn a_snooze_belongs_to_its_subject_and_ends_by_itself() {
    let (_dir, storage) = storage();
    let repo = storage.notices();
    repo.snooze("s:1", Some(1_000), 10).unwrap();
    repo.snooze("signin:a", None, 10).unwrap();
    assert_eq!(
        repo.snoozes(500).unwrap(),
        [
            ("s:1".to_owned(), Some(1_000)),
            ("signin:a".to_owned(), None)
        ]
    );
    // Past its time it is no longer on; the tick forgets it.
    assert_eq!(
        repo.snoozes(1_000).unwrap(),
        [("signin:a".to_owned(), None)]
    );
    assert_eq!(repo.end_snoozes(1_000).unwrap(), 1);
    assert_eq!(repo.end_snoozes(1_000).unwrap(), 0);
    // Snoozed again: the new time wins. Woken: gone.
    repo.snooze("signin:a", Some(9_000), 20).unwrap();
    assert_eq!(
        repo.snoozes(500).unwrap(),
        [("signin:a".to_owned(), Some(9_000))]
    );
    assert!(repo.wake("signin:a").unwrap());
    assert!(!repo.wake("signin:a").unwrap());
    assert!(repo.snoozes(0).unwrap().is_empty());
}

#[test]
fn what_happened_is_told_once() {
    let (_dir, storage) = storage();
    let repo = storage.notices();
    assert!(
        repo.record(&happened("finished:s1:100"), 100)
            .unwrap()
            .is_some()
    );
    assert!(
        repo.record(&happened("finished:s1:100"), 200)
            .unwrap()
            .is_none()
    );
    assert_eq!(repo.earlier_count().unwrap(), 1);
    // It never counts as a need, whatever it was written as.
    assert_eq!(repo.open_count().unwrap(), 0);
    let row = &repo.earlier(None, 10).unwrap()[0];
    assert!(!row.needs);
    assert_eq!(row.resolved_at, Some(100));
}

#[test]
fn pages_walk_what_happened_once_newest_first() {
    let (_dir, storage) = storage();
    let repo = storage.notices();
    for at in 0..7 {
        repo.record(&happened(&format!("finished:{at}")), 1_000 + at)
            .unwrap();
    }
    // Two at the same moment: the id breaks the tie.
    repo.record(&happened("finished:tie"), 1_006).unwrap();
    let mut seen = Vec::new();
    let mut after = None;
    loop {
        let rows = repo.earlier(after, 3).unwrap();
        let more = rows.len() > 3;
        let page: Vec<_> = rows.into_iter().take(3).collect();
        after = page.last().map(|row| (row.created_at, row.id));
        seen.extend(page.into_iter().map(|row| row.key));
        if !more {
            break;
        }
    }
    assert_eq!(
        seen,
        [
            "finished:tie",
            "finished:6",
            "finished:5",
            "finished:4",
            "finished:3",
            "finished:2",
            "finished:1",
            "finished:0"
        ]
    );
}

#[test]
fn thirty_days_are_kept_and_an_open_need_always() {
    let (_dir, storage) = storage();
    let repo = storage.notices();
    let now = 100 * 24 * 3_600_000;
    repo.record(&happened("old"), now - NOTICE_KEEP_MS - 1)
        .unwrap();
    repo.record(&happened("recent"), now - NOTICE_KEEP_MS + 1)
        .unwrap();
    repo.open(&need("asks:ancient", "Still waiting"), 0)
        .unwrap();
    assert_eq!(repo.purge(now - NOTICE_KEEP_MS).unwrap(), 1);
    let kept: Vec<String> = repo
        .earlier(None, 10)
        .unwrap()
        .into_iter()
        .map(|row| row.key)
        .collect();
    assert_eq!(kept, ["recent"]);
    assert_eq!(repo.open_count().unwrap(), 1);
}

#[test]
fn read_all_dismiss_and_mutes() {
    let (_dir, storage) = storage();
    let repo = storage.notices();
    let a = repo.open(&need("asks:a", "A"), 1).unwrap().id();
    repo.record(&happened("finished:x"), 2).unwrap();
    assert_eq!(repo.mark_all_read(3).unwrap(), 2);
    assert_eq!(repo.mark_all_read(4).unwrap(), 0);
    assert!(repo.dismiss(a, 5).unwrap());
    assert_eq!(repo.get(a).unwrap().unwrap().dismissed_at, Some(5));
    assert!(!repo.dismiss(9_999, 5).unwrap());

    repo.mute("project", "proj-1", "docs-site", 6).unwrap();
    repo.mute("kind", "finished", "Finished sessions", 7)
        .unwrap();
    repo.mute("project", "proj-1", "docs (renamed)", 8).unwrap();
    let mutes = repo.mutes().unwrap();
    assert_eq!(mutes.len(), 2);
    assert_eq!(mutes[0].label, "docs (renamed)");
    assert!(repo.unmute("kind", "finished").unwrap());
    assert!(!repo.unmute("kind", "finished").unwrap());
    assert_eq!(repo.mutes().unwrap().len(), 1);
}
