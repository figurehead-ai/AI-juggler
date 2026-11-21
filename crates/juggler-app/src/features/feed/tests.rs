//! The feed's read model (D-118, AJ-E3702): the keyset page (stable,
//! never load-all, within its time budget on 5,000 events), every rollup
//! key (a level boundary is never crossed), and the filters.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use juggler_core::storage::Storage;
use rusqlite::params;

use super::dto::{FeedFiltersInfo, FeedItemInfo};
use super::page::{self, Read};
use crate::paging::PageRequest;

const T0: i64 = 1_800_000_000_000;

/// A store with two Accounts, a project, a pool, a thread and a task.
fn store() -> Storage {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .raw_conn()
        .execute_batch(
            "INSERT INTO accounts (id, provider_id, label, state_dir, created_at) VALUES
                ('acc-work', 'claude-code', 'Work Max', '/x/w', 0),
                ('acc-home', 'codex', 'Personal', '/x/h', 0);
             INSERT INTO launch_recipes (id, account_id, settings_json, created_at) VALUES
                ('prof-work', 'acc-work', '{}', 0), ('prof-home', 'acc-home', '{}', 0);
             INSERT INTO projects (id, root, name, created_at) VALUES
                ('proj-web', '/dev/web-app', 'web-app', 0), ('proj-notes', '/dev/trip-notes', 'trip-notes', 0);
             INSERT INTO pools (id, name, kind, project, created_at) VALUES ('pool-rel', 'Release', 'work', 'proj-web', 0);
             INSERT INTO agent_threads (id, pool_id, subject, started_by, created_at, updated_at, count)
                VALUES ('th-1', 'pool-rel', 'Export flow', 'builder-1', 0, 0, 7);
             INSERT INTO tasks (id, pool_id, kind, title, status, created_by, created_by_name, created_at, updated_at)
                VALUES ('task-1', 'pool-rel', 'task', 'Wire the export button', 'done', 'b2', 'builder-2', 0, 0);
             INSERT INTO work_items (id, project, title, created_at) VALUES ('work-1', 'proj-web', 'Answer the release question', 0);
             INSERT INTO epochs (id, work_id, account, provider) VALUES ('ep-1', 'work-1', 'acc-work', 'claude-code');",
        )
        .unwrap();
    storage
}

fn session(storage: &Storage, id: &str, profile: &str, project: &str, started: i64) {
    storage
        .raw_conn()
        .execute(
            "INSERT INTO instances (id, launch_recipe_id, machine, cwd, status, started_at, usage_json, project)
             VALUES (?1, ?2, 'this', ?3, 'finished', ?4, '{}', ?5)",
            params![id, profile, format!("/dev/{project}"), started, project],
        )
        .unwrap();
}

fn ended(storage: &Storage, id: &str, event: &str, at: i64) {
    storage
        .raw_conn()
        .execute(
            "INSERT INTO instance_lifecycle (session_id, event, at_ms) VALUES (?1, ?2, ?3)",
            params![id, event, at],
        )
        .unwrap();
}

fn notice(storage: &Storage, key: &str, kind: &str, needs: bool, data: &str, at: i64) {
    storage
        .raw_conn()
        .execute(
            "INSERT INTO notices (key, kind, origin, needs, title, data, meter, created_at, updated_at, resolved_at)
             VALUES (?1, ?2, 'app', ?3, 'A line', ?4, 8, ?5, ?5, ?5)",
            params![key, kind, i64::from(needs), data, at],
        )
        .unwrap();
}

fn switch(storage: &Storage, id: &str, state: &str, at: i64) {
    storage
        .raw_conn()
        .execute(
            "INSERT INTO switch_edges (id, work_id, from_epoch, from_account, to_account, from_provider,
                to_provider, mechanism, state, actor, created_at)
             VALUES (?1, 'work-1', 'ep-1', 'acc-work', 'acc-home', 'claude-code', 'codex', 'pack', ?2, 'user', ?3)",
            params![id, state, at],
        )
        .unwrap();
}

fn task_event(storage: &Storage, detail: &str, at: i64) {
    storage
        .raw_conn()
        .execute(
            "INSERT INTO task_events (at, pool_id, task_id, actor_ref, actor_name, verb, detail)
             VALUES (?1, 'pool-rel', 'task-1', 'b2', 'builder-2', 'status', ?2)",
            params![at, detail],
        )
        .unwrap();
}

fn message(storage: &Storage, id: &str, kind: &str, at: i64) {
    storage
        .raw_conn()
        .execute(
            "INSERT INTO agent_messages (id, thread_id, pool_id, sender_ref, sender_kind, sender_name, kind,
                subject, body, body_sha256, hop, created_at)
             VALUES (?1, 'th-1', 'pool-rel', 'rv', 'agent', 'reviewer', ?2, 'Export flow',
                'The migration misses its down step.', 'x', 0, ?3)",
            params![id, kind, at],
        )
        .unwrap();
}

/// One of each F1 item, a minute apart, newest last.
fn day(storage: &Storage) {
    session(storage, "s-ok", "prof-work", "proj-web", T0);
    session(storage, "s-bad", "prof-work", "proj-web", T0);
    session(storage, "s-mine", "prof-home", "proj-notes", T0);
    ended(storage, "s-ok", "ended: finished", T0 + 60_000);
    ended(storage, "s-bad", "ended: error (exit code 1)", T0 + 120_000);
    ended(storage, "s-mine", "ended: stopped by you", T0 + 180_000);
    notice(
        storage,
        "finished:s-ok",
        "finished",
        false,
        r#"{"target":{"session_id":"s-ok"}}"#,
        T0 + 61_000,
    );
    notice(
        storage,
        "asks:s-ok",
        "asks",
        true,
        r#"{"target":{"session_id":"s-ok"}}"#,
        T0 + 240_000,
    );
    notice(
        storage,
        "reset:acc-home:1",
        "reset",
        false,
        r#"{"target":{"account_id":"acc-home"},"facts":{"account":"Personal","window":"5-hour"}}"#,
        T0 + 300_000,
    );
    switch(storage, "edge-1", "confirmed", T0 + 360_000);
    switch(storage, "edge-pending", "launched", T0 + 361_000);
    task_event(storage, "In progress → Done", T0 + 420_000);
    task_event(storage, "To do → In progress", T0 + 421_000);
    message(storage, "msg-1", "review_request", T0 + 480_000);
    message(storage, "msg-note", "note", T0 + 481_000);
}

fn read(
    storage: &Storage,
    cursor: Option<&str>,
    limit: u32,
    filters: &FeedFiltersInfo,
) -> crate::paging::Page<FeedItemInfo> {
    let request = PageRequest {
        cursor: cursor.map(str::to_owned),
        limit: Some(limit),
    };
    page::page(storage, &request, filters, Read::default()).unwrap()
}

fn kinds(items: &[FeedItemInfo]) -> Vec<&str> {
    items.iter().map(|item| item.kind.as_str()).collect()
}

#[test]
fn a_day_reads_newest_first_with_every_journal_and_nothing_in_progress_or_chatter() {
    let storage = store();
    day(&storage);
    let page = read(&storage, None, 30, &FeedFiltersInfo::default());
    assert_eq!(
        kinds(&page.items),
        [
            "message",
            "task_done",
            "switch",
            "reset",
            "answered",
            "failed",
            "finished",
            "finished"
        ],
        "a launched switch, a task started, a note and the user's own stop are not news"
    );
    assert!(page.next_cursor.is_none(), "all of it fits");
    // One session, one card: its ending and its finished notice share a key.
    let finished: Vec<_> = page
        .items
        .iter()
        .filter(|item| item.kind == "finished")
        .collect();
    assert_eq!(finished[0].coalesce, finished[1].coalesce);
    assert_eq!(finished[0].coalesce.as_deref(), Some("s:s-ok"));
    let failed = page
        .items
        .iter()
        .find(|item| item.kind == "failed")
        .unwrap();
    assert_eq!(failed.level, 3);
    assert_eq!(failed.facts.detail.as_deref(), Some("exit code 1"));
    assert_eq!(failed.facts.project.as_deref(), Some("web-app"));
    assert_eq!(failed.author.folder_label.as_deref(), Some("Work Max"));
    assert_eq!(failed.facts.duration_ms, Some(120_000));
}

#[test]
fn a_cursor_never_repeats_or_skips_even_when_items_arrive_between_pages() {
    let storage = store();
    day(&storage);
    let first = read(&storage, None, 3, &FeedFiltersInfo::default());
    assert_eq!(first.items.len(), 3);
    // New items arrive at the top while the reader is on page one.
    message(&storage, "msg-new", "status", T0 + 900_000);
    ended(&storage, "s-bad", "ended: crashed (signal 9)", T0 + 901_000);
    let mut seen: Vec<String> = first.items.iter().map(|item| item.id.clone()).collect();
    let mut cursor = first.next_cursor;
    while let Some(at) = cursor {
        let next = read(&storage, Some(&at), 3, &FeedFiltersInfo::default());
        seen.extend(next.items.iter().map(|item| item.id.clone()));
        cursor = next.next_cursor;
    }
    let unique: HashSet<_> = seen.iter().collect();
    assert_eq!(unique.len(), seen.len(), "no item twice: {seen:?}");
    assert_eq!(
        seen.len(),
        8,
        "the eight items of the day, none of the newer two: {seen:?}"
    );
    // Read again from the top, the newer two lead.
    let top = read(&storage, None, 2, &FeedFiltersInfo::default());
    assert_eq!(kinds(&top.items), ["crashed", "message"]);
}

#[test]
fn a_cursor_this_app_did_not_make_is_refused() {
    let storage = store();
    let request = PageRequest {
        cursor: Some("k1:5:61".to_owned()),
        limit: Some(30),
    };
    assert!(
        page::page(
            &storage,
            &request,
            &FeedFiltersInfo::default(),
            Read::default()
        )
        .is_err()
    );
}

#[test]
fn every_rollup_key_carries_its_level_so_nothing_broken_folds_into_a_quiet_one() {
    let storage = store();
    day(&storage);
    ended(
        &storage,
        "s-mine",
        "ended: crashed (signal 11)",
        T0 + 600_000,
    );
    let items = read(&storage, None, 30, &FeedFiltersInfo::default()).items;
    for item in &items {
        if let Some(key) = &item.rollup {
            assert!(
                key.ends_with(&format!(":L{}", item.level)),
                "{} {key}",
                item.kind
            );
        }
        if item.level == 3 {
            assert!(
                item.rollup.is_none(),
                "a broken item stands alone: {}",
                item.id
            );
        }
    }
    // Two endings of one App and project share a key only at one level.
    let finished = items
        .iter()
        .find(|item| item.kind == "finished" && item.id.starts_with("l:"))
        .unwrap();
    let failed = items.iter().find(|item| item.kind == "failed").unwrap();
    assert_eq!(finished.scope, failed.scope, "the same App");
    assert_ne!(finished.rollup, failed.rollup);
    assert_eq!(
        finished.rollup.as_deref(),
        Some("v:finished:acc-work:proj-web:L1")
    );
    let task = items.iter().find(|item| item.kind == "task_done").unwrap();
    assert_eq!(task.rollup.as_deref(), Some("t:pool-rel:task_done:L0"));
    assert_eq!(task.rollup_window_ms, super::items::TASKS_MS);
    let review = items.iter().find(|item| item.kind == "message").unwrap();
    assert_eq!(
        review.rollup.as_deref(),
        Some("m:th-1:L1"),
        "a review request is a note"
    );
    assert_eq!(review.facts.thread_count, Some(7), "a note is only counted");
}

#[test]
fn the_filters_narrow_by_chip_project_pool_and_account_and_my_acts_wait_to_be_asked() {
    let storage = store();
    day(&storage);
    let chip = |category: &str| FeedFiltersInfo {
        category: Some(category.to_owned()),
        ..FeedFiltersInfo::default()
    };
    assert_eq!(
        kinds(&read(&storage, None, 30, &chip("needs")).items),
        ["answered", "failed"]
    );
    assert_eq!(
        kinds(&read(&storage, None, 30, &chip("finished")).items),
        ["finished", "finished"]
    );
    assert_eq!(
        kinds(&read(&storage, None, 30, &chip("switches")).items),
        ["switch", "reset"]
    );
    assert_eq!(
        kinds(&read(&storage, None, 30, &chip("pools")).items),
        ["message", "task_done"]
    );
    let pool = FeedFiltersInfo {
        pool: Some("pool-rel".to_owned()),
        ..FeedFiltersInfo::default()
    };
    assert_eq!(
        kinds(&read(&storage, None, 30, &pool).items),
        ["message", "task_done"]
    );
    let home = FeedFiltersInfo {
        folders: Some(vec!["acc-home".to_owned()]),
        include_mine: true,
        ..FeedFiltersInfo::default()
    };
    assert_eq!(
        kinds(&read(&storage, None, 30, &home).items),
        ["switch", "reset", "stopped"]
    );
    let notes = FeedFiltersInfo {
        project: Some("proj-notes".to_owned()),
        ..FeedFiltersInfo::default()
    };
    assert!(
        read(&storage, None, 30, &notes).items.is_empty(),
        "the stop is the user's own"
    );
}

/// 5,000 events across the journals, a few seconds apart.
fn busy(storage: &Storage) {
    for at in 0..20 {
        session(
            storage,
            &format!("s-{at}"),
            if at % 2 == 0 {
                "prof-work"
            } else {
                "prof-home"
            },
            "proj-web",
            T0,
        );
    }
    let conn = storage.raw_conn();
    conn.execute_batch("BEGIN").unwrap();
    for n in 0..5_000_i64 {
        let at = T0 + n * 7_000;
        match n % 5 {
            0 => ended(storage, &format!("s-{}", n % 20), "ended: finished", at),
            1 => notice(
                storage,
                &format!("finished:{n}"),
                "finished",
                false,
                &format!(r#"{{"target":{{"session_id":"s-{}"}}}}"#, n % 20),
                at,
            ),
            2 => task_event(storage, "In review → Done", at),
            3 => message(storage, &format!("m-{n}"), "status", at),
            _ => switch(storage, &format!("e-{n}"), "confirmed", at),
        }
    }
    conn.execute_batch("COMMIT").unwrap();
}

#[test]
fn five_thousand_events_page_thirty_at_a_time_within_the_budget_never_reading_them_all() {
    let storage = store();
    busy(&storage);
    let filters = FeedFiltersInfo::default();
    let mut cursor: Option<String> = None;
    let mut slowest = Duration::ZERO;
    let mut seen = HashSet::new();
    let mut last_at = i64::MAX;
    // Three pages load by themselves, then Load more: read ten.
    for _ in 0..10 {
        let started = Instant::now();
        let page = read(&storage, cursor.as_deref(), 30, &filters);
        slowest = slowest.max(started.elapsed());
        assert_eq!(page.items.len(), 30, "a whole page each time");
        for item in &page.items {
            assert!(item.at_ms <= last_at, "newest first across pages");
            last_at = item.at_ms;
            assert!(seen.insert(item.id.clone()), "{} twice", item.id);
        }
        cursor = page.next_cursor;
        assert!(cursor.is_some());
    }
    eprintln!("feed_page on 5,000 events: slowest of 10 pages {slowest:?}");
    assert!(
        slowest <= Duration::from_millis(50),
        "a page took {slowest:?} (budget 50 ms)"
    );
    // Each read asks one page of each journal, so the whole log is never
    // read: the query plans walk the time indexes.
    let plan: String = storage
        .raw_conn()
        .query_row(
            "EXPLAIN QUERY PLAN SELECT rowid FROM instance_lifecycle WHERE at_ms < ?1 ORDER BY at_ms DESC, rowid DESC LIMIT 30",
            [T0],
            |row| row.get(3),
        )
        .unwrap();
    assert!(plan.contains("idx_instance_lifecycle_at"), "{plan}");
}
