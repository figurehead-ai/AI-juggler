//! The usage history's summary queries (0.1.5): what they sum, and their
//! time budget on a large store (a million messages over a year on seven
//! profiles): under 50 ms a query for up to 90 days (a year is read once
//! and kept: the app's settled-hours memory).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::time::{Duration, Instant};

use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{QuotaSample, UsageEvent, UsageSource, WindowKind};
use juggler_core::storage::{HOUR_MS, Storage};
use rusqlite::{Connection, params};

const DAY: i64 = 86_400_000;
const NOW: i64 = 1_792_094_400_000;

fn event(account: &str, session: &str, ts: i64, model: &str, input: u64, uuid: &str) -> UsageEvent {
    UsageEvent {
        account: account.into(),
        session: Some(session.into()),
        ts,
        model: Some(model.into()),
        tokens: TokenCounts {
            input,
            output: input / 4,
            cache_read: input * 10,
            cache_write: 0,
            reasoning: 0,
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Measured,
        msg_uuid: Some(uuid.into()),
    }
}

#[test]
fn the_summaries_sum_per_hour_session_and_model() {
    let storage = Storage::open_in_memory().unwrap();
    let base = NOW - NOW % HOUR_MS;
    storage
        .usage()
        .insert_events(&[
            event("work", "s-1", base + 1_000, "opus", 100, "u1"),
            event("work", "s-1", base + 2_000, "opus", 200, "u2"),
            event("work", "s-1", base + 3_000, "sonnet", 50, "u3"),
            event("work", "s-2", base + HOUR_MS + 5, "opus", 400, "u4"),
            event("work", "s-2", base - DAY, "opus", 7, "u5"),
            event("codex", "c-1", base + 10, "gpt-5", 999, "u6"),
        ])
        .unwrap();
    let history = storage.usage_history();

    let (rows, cut) = history
        .hourly("work", base, base + 2 * HOUR_MS, 100)
        .unwrap();
    assert!(!cut);
    // Newest hour first; each (hour, session, model) once.
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].hour_ms, base + HOUR_MS);
    assert_eq!(rows[0].tokens.input, 400);
    let opus = rows
        .iter()
        .find(|row| row.hour_ms == base && row.model.as_deref() == Some("opus"))
        .unwrap();
    assert_eq!(opus.tokens.input, 300);
    assert_eq!(opus.tokens.output, 75);
    assert_eq!(opus.tokens.cache_read, 3_000);
    assert_eq!(opus.messages, 2);

    let (rows, cut) = history.hourly("work", base, base + 2 * HOUR_MS, 2).unwrap();
    assert!(cut, "a cut summary says so");
    assert_eq!(rows.len(), 2);

    assert_eq!(history.first_ts("work").unwrap(), Some(base - DAY));
    assert_eq!(history.first_ts("nobody").unwrap(), None);

    let (turns, cut) = history.session_turns("work", "s-1", 2).unwrap();
    assert!(cut);
    assert_eq!(turns.len(), 2, "the newest two, oldest first");
    assert_eq!(turns[0].ts, base + 2_000);
    assert_eq!(turns[1].model.as_deref(), Some("sonnet"));

    let (all, mine) = history.share_since("work", "s-1", base).unwrap();
    assert_eq!(all.input, 750);
    assert_eq!(mine.input, 350);

    let by_model = history.session_by_model("work", "s-1").unwrap();
    assert_eq!(by_model.len(), 2);
    let opus = by_model
        .iter()
        .find(|row| row.model.as_deref() == Some("opus"))
        .unwrap();
    assert_eq!(
        (opus.first_ms, opus.tokens.input, opus.messages),
        (base + 1_000, 300, 2)
    );
}

#[test]
fn a_windows_readings_come_back_oldest_first() {
    let storage = Storage::open_in_memory().unwrap();
    let usage = storage.usage();
    for (n, pct) in [10.0, 30.0, 55.0, 5.0].into_iter().enumerate() {
        let ts = NOW + n as i64 * HOUR_MS;
        let sample = QuotaSample {
            ts,
            used_pct: pct,
            resets_at: Some(NOW + if n < 3 { 5 } else { 10 } * HOUR_MS),
            confidence: Confidence::Measured,
        };
        usage
            .insert_sample("work", WindowKind::FiveHour, &sample)
            .unwrap();
    }
    let history = storage.usage_history();
    let samples = history
        .samples_between("work", WindowKind::FiveHour, NOW, NOW + DAY, 10)
        .unwrap();
    assert_eq!(
        samples.iter().map(|s| s.used_pct).collect::<Vec<_>>(),
        vec![10.0, 30.0, 55.0, 5.0]
    );
    let newest_two = history
        .samples_between("work", WindowKind::FiveHour, NOW, NOW + DAY, 2)
        .unwrap();
    assert_eq!(
        newest_two.iter().map(|s| s.used_pct).collect::<Vec<_>>(),
        vec![55.0, 5.0]
    );
    let latest = history
        .latest_sample("work", WindowKind::FiveHour, NOW + 2 * HOUR_MS)
        .unwrap()
        .unwrap();
    assert_eq!(latest.used_pct, 55.0);
    assert!(
        history
            .samples_between(
                "work",
                WindowKind::Weekly { anchor_ms: None },
                NOW,
                NOW + DAY,
                10
            )
            .unwrap()
            .is_empty(),
        "a five-hour reading is never a weekly one"
    );
}

/// A deterministic store: seven profiles, a year, about a million
/// messages, with a weekly rhythm (quiet weekends) and heavy days.
fn large_store(path: &std::path::Path) -> usize {
    drop(Storage::open(path).unwrap());
    let mut conn = Connection::open(path).unwrap();
    let tx = conn.transaction().unwrap();
    let mut seed: u64 = 42;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) as i64
    };
    let profiles = [
        "client-a", "client-b", "lab", "personal", "work", "work-2", "codex",
    ];
    let models = [
        "claude-opus-4-5",
        "claude-sonnet-4-5",
        "claude-haiku-4-5",
        "gpt-5-codex",
    ];
    let mut count = 0usize;
    {
        let mut insert = tx
            .prepare(
                "INSERT INTO usage_events (account, session, ts, model, input, output, cache_r, cache_w,
                     reasoning, source, confidence, msg_uuid)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, 'transcript', 'measured', ?8)",
            )
            .unwrap();
        let start = NOW - 365 * DAY;
        for day in 0..365i64 {
            let weekday = (day + 4) % 7; // 0 = Monday
            let heavy = day % 11 == 0;
            for (p, profile) in profiles.iter().enumerate() {
                let weekend = weekday >= 5;
                let sessions = if weekend {
                    1
                } else if heavy {
                    9
                } else {
                    5
                };
                for s in 0..sessions {
                    let session = format!("{profile}-{day}-{s}");
                    let begin = start + day * DAY + (8 + next() % 10) * HOUR_MS;
                    let messages = if weekend { 20 } else { 80 + next() % 40 };
                    let model = models[if *profile == "codex" {
                        3
                    } else {
                        (s as usize + p) % 3
                    }];
                    for m in 0..messages {
                        let ts = begin + m * (90_000 + next() % 60_000);
                        let input = 2_000 + next() % 8_000;
                        insert
                            .execute(params![
                                profile,
                                session,
                                ts,
                                model,
                                input,
                                input / 5,
                                input * 12,
                                format!("{session}-{m}")
                            ])
                            .unwrap();
                        count += 1;
                    }
                }
            }
        }
    }
    tx.commit().unwrap();
    count
}

/// The best of three runs (the Mac is shared: one slow run is noise, three are not).
fn timed<T>(what: &str, budget: Duration, f: impl Fn() -> T) -> (T, Duration) {
    let mut took = Duration::MAX;
    let mut out = None;
    for _ in 0..3 {
        let start = Instant::now();
        out = Some(f());
        took = took.min(start.elapsed());
    }
    let out = out.unwrap();
    eprintln!("  {what}: {:.1} ms", took.as_secs_f64() * 1000.0);
    assert!(
        took <= budget,
        "{what} took {took:?}, over its {budget:?} budget"
    );
    (out, took)
}

/// Run it with `cargo test -p juggler-core --test usage_history -- --ignored --nocapture`.
#[test]
#[ignore = "builds a million-message store (a few seconds); the budget run"]
fn the_summaries_hold_their_budget_on_a_million_messages() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.db");
    let built = Instant::now();
    let count = large_store(&path);
    eprintln!(
        "built {count} messages in {:.1} s",
        built.elapsed().as_secs_f64()
    );
    assert!(count > 900_000, "about a million messages, not {count}");
    let storage = Storage::open(&path).unwrap();
    let history = storage.usage_history();
    let budget = Duration::from_millis(50);
    let profiles = [
        "client-a", "client-b", "lab", "personal", "work", "work-2", "codex",
    ];
    // A year of one profile is read once and kept in memory by the app
    // (features::usage_history::settled); it's timed, not held to 50 ms.
    for (name, span, limit) in [
        ("24 h", DAY, budget),
        ("7 days", 7 * DAY, budget),
        ("30 days", 30 * DAY, budget),
        ("90 days", 90 * DAY, budget),
        ("a year, read once", 366 * DAY, Duration::from_millis(400)),
    ] {
        eprintln!("{name}:");
        let mut page = Duration::ZERO;
        let mut rows = 0;
        for profile in profiles {
            let ((found, _), took) = timed(&format!("hourly {profile}"), limit, || {
                history.hourly(profile, NOW - span, NOW, 60_000).unwrap()
            });
            page += took;
            rows += found.len();
        }
        eprintln!(
            "  -> {rows} hourly rows for 7 profiles, {:.1} ms in all",
            page.as_secs_f64() * 1000.0
        );
    }
    let (first, _) = timed("first usage", budget, || history.first_ts("work").unwrap());
    assert!(first.is_some());
    let session = format!("work-{}-0", 360);
    timed("one session's turns", budget, || {
        history.session_turns("work", &session, 2_000).unwrap()
    });
    timed("one session by model", budget, || {
        history.session_by_model("work", &session).unwrap()
    });
    timed("a window's share", budget, || {
        history
            .share_since("work", &session, NOW - 7 * DAY)
            .unwrap()
    });
    timed("the settled stamp", budget, || {
        history.settled_stamp().unwrap()
    });
}

/// 0.1.5 → 0.2: a 0.1.5 database is schema 32 with the derived tables
/// beside it (no migration of their own). Opened by 0.2 it migrates to
/// pools (v33), messages (v34) and on (tasks: v36); the derived tables
/// keep their rows and their triggers are made again. A fresh database at
/// the latest schema has them too.
#[test]
fn the_derived_tables_live_on_the_latest_schema_and_survive_its_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    {
        let conn = Connection::open(&db).unwrap();
        for (index, sql) in Storage::migrations().iter().enumerate().take(32) {
            conn.execute_batch(sql).unwrap();
            conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
                .unwrap();
        }
        // What 0.1.5 left beside schema 32: a sourced reading, the
        // history's settled stamp.
        conn.execute_batch(
            "CREATE TABLE quota_sample_sources (
                 account TEXT NOT NULL, window TEXT NOT NULL, ts INTEGER NOT NULL,
                 source TEXT NOT NULL, PRIMARY KEY (account, window, ts)) WITHOUT ROWID;
             INSERT INTO quota_samples (account, window, ts, used_pct, resets_at, confidence)
                 VALUES ('work', 'five_hour', 1000, 40.0, NULL, 'measured');
             INSERT INTO quota_sample_sources VALUES ('work', 'five_hour', 1000, 'statusline');
             CREATE TABLE usage_history_epoch (id INTEGER PRIMARY KEY CHECK (id = 1),
                 token INTEGER NOT NULL, changed INTEGER NOT NULL DEFAULT 0);
             INSERT INTO usage_history_epoch VALUES (1, 7, 3);",
        )
        .unwrap();
    }
    let storage = Storage::open(&db).unwrap();
    assert!(Storage::schema_version() >= 34);
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    let kept = storage
        .usage()
        .sourced_samples_for_account("work", WindowKind::FiveHour)
        .unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].1, Some(UsageSource::Statusline));
    // The settled triggers are on the upgraded file: an old message moves
    // the stamp 0.1.5 kept.
    assert_eq!(storage.usage_history().settled_stamp().unwrap(), (7, 3));
    storage
        .usage()
        .insert_events(&[event("work", "s-old", 1_000, "opus", 10, "old-1")])
        .unwrap();
    assert_eq!(storage.usage_history().settled_stamp().unwrap(), (7, 4));
    drop(storage);
    // A fresh database at the latest schema has them from its first open.
    let fresh = Storage::open_in_memory().unwrap();
    assert_eq!(fresh.user_version().unwrap(), Storage::schema_version());
    fresh
        .usage()
        .insert_sourced_samples(&[(
            "work".to_owned(),
            WindowKind::FiveHour,
            QuotaSample {
                ts: 2_000,
                used_pct: 12.0,
                resets_at: None,
                confidence: Confidence::Measured,
            },
            UsageSource::ApiRetry,
        )])
        .unwrap();
    let read = fresh
        .usage()
        .sourced_samples_for_account("work", WindowKind::FiveHour)
        .unwrap();
    assert_eq!(read[0].1, Some(UsageSource::ApiRetry));
    assert!(fresh.usage_history().settled_stamp().is_ok());
}
