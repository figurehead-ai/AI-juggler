//! AJ-E2402 (D-41a): the sightings store's guardrails. What is kept of a
//! process in a session another app runs is its name, times and a hashed
//! command line, for 7 days; never its environment or its command line.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use juggler_core::storage::{
    MarkKind, ProcessMark, SIGHTING_COLUMNS, SIGHTING_KEEP_MS, Sighting, Storage,
};

const DAY: i64 = 24 * 3_600_000;

fn sighting(pid: u32, start: i64, seen: i64) -> Sighting {
    Sighting {
        pid,
        start_time: start,
        boot_id: "boot-1".into(),
        exe_name: "node".into(),
        cmd_hash: "ab12".into(),
        tool: "tool-a".into(),
        session_id: "conv-1".into(),
        app: Some("An app".into()),
        session_root: false,
        parent_pid: Some(10),
        first_seen: seen,
        last_seen: seen,
    }
}

fn columns(storage: &Storage, table: &str) -> Vec<String> {
    let conn = storage.raw_conn();
    let mut stmt = conn
        .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .unwrap();
    stmt.query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// The guardrail: the table holds exactly the reviewed columns, and none
/// of them could hold an environment or a command line.
#[test]
fn a_sighting_never_holds_the_environment_or_the_command_line() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("j.sqlite")).unwrap();
    let have = columns(&storage, "process_sightings");
    assert_eq!(have, SIGHTING_COLUMNS, "a new column is a reviewed change");
    for column in &have {
        let lower = column.to_ascii_lowercase();
        for banned in ["env", "cmdline", "args", "argv", "cwd", "command"] {
            assert!(
                !lower.contains(banned),
                "column {column} could hold what D-41 forbids"
            );
        }
    }
    // The command line goes in only as its hash.
    storage
        .sightings()
        .record(&[sighting(20, 100, 1_000)])
        .unwrap();
    let kept = storage.sightings().get(20, 100, "boot-1").unwrap().unwrap();
    assert_eq!(kept.cmd_hash, "ab12");
    assert_eq!(kept.exe_name, "node");
}

/// D-41a: kept 7 days after it was last seen, then forgotten.
#[test]
fn a_sighting_is_forgotten_seven_days_after_it_was_last_seen() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("j.sqlite")).unwrap();
    let now = 30 * DAY;
    let repo = storage.sightings();
    repo.record(&[
        sighting(1, 1, now - 8 * DAY),
        sighting(2, 2, now - 6 * DAY),
        sighting(3, 3, now),
    ])
    .unwrap();
    assert_eq!(SIGHTING_KEEP_MS, 7 * DAY);
    let removed = repo.prune(now - SIGHTING_KEEP_MS, now).unwrap();
    assert_eq!(removed, 1);
    assert!(repo.get(1, 1, "boot-1").unwrap().is_none());
    assert!(repo.get(2, 2, "boot-1").unwrap().is_some());
    assert_eq!(repo.count().unwrap(), 2);
}

/// Seen again: last seen moves on, first seen and the session stay.
#[test]
fn seeing_a_process_again_moves_only_its_last_seen() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("j.sqlite")).unwrap();
    let repo = storage.sightings();
    repo.record(&[sighting(7, 70, 1_000)]).unwrap();
    let mut again = sighting(7, 70, 5_000);
    again.first_seen = 5_000;
    again.session_id = "conv-other".into();
    repo.record(&[again]).unwrap();
    let kept = repo.get(7, 70, "boot-1").unwrap().unwrap();
    assert_eq!(kept.first_seen, 1_000);
    assert_eq!(kept.last_seen, 5_000);
    assert_eq!(kept.session_id, "conv-1");
    // A reused pid (another start time) is another process.
    repo.record(&[sighting(7, 71, 6_000)]).unwrap();
    assert_eq!(repo.latest_of(7, "boot-1").unwrap().unwrap().start_time, 71);
    assert_eq!(repo.seen_since("boot-1", 4_000).unwrap().len(), 2);
    assert!(repo.seen_since("boot-2", 0).unwrap().is_empty());
}

/// The user's marks: Keep stays, Remind me later lapses at its time.
#[test]
fn a_reminder_lapses_and_keep_stays() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("j.sqlite")).unwrap();
    let repo = storage.sightings();
    let mark = |pid, kind, until_ms| ProcessMark {
        pid,
        start_time: 1,
        boot_id: "boot-1".into(),
        kind,
        until_ms,
        created_at: 1_000,
    };
    repo.set_mark(&mark(1, MarkKind::Keep, None)).unwrap();
    repo.set_mark(&mark(2, MarkKind::Snooze, Some(10_000)))
        .unwrap();
    assert_eq!(repo.marks("boot-1", 5_000).unwrap().len(), 2);
    let later = repo.marks("boot-1", 20_000).unwrap();
    assert_eq!(later.len(), 1);
    assert_eq!(later[0].kind, MarkKind::Keep);
    repo.clear_mark(1, 1, "boot-1").unwrap();
    assert_eq!(repo.marks("boot-1", 5_000).unwrap().len(), 1);
}
