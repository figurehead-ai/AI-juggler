//! 0.3.8: a conversation copied into another account outside AI Juggler (a
//! Dev build's switch, a folder copied by hand, another Mac) repeats the
//! vendor's own call keys. Each call was billed once, so it counts once:
//! under the original for the shared history, under the copy for its own
//! new messages. Random copies across two accounts, read in random passes,
//! always keep each call where a brute-force reading of the rule over every
//! account's holders puts it, once.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeMap, BTreeSet};

use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{UsageEvent, UsageSource};
use juggler_core::storage::Storage;
use proptest::prelude::*;

/// `(account, session)`.
type Place = (String, String);
/// Each place's messages `(key, ts)`, as its file holds them at the end.
type Files = BTreeMap<Place, Vec<(u64, i64)>>;

struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

const ACCOUNTS: [&str; 2] = ["acc", "bcc"];

/// A random history over two accounts: sessions write messages, and a
/// session is copied (a resume in its own account under a new id, or a
/// file copied into the other account under the same id), copying its
/// messages with their times; copies go on writing.
fn history(dice: &mut Dice) -> Files {
    let mut files: Files = BTreeMap::new();
    let mut order: Vec<Place> = Vec::new();
    let mut clock = 0_i64;
    let mut next_key = 1_u64;
    let name = |dice: &mut Dice, files: &Files, account: &str| loop {
        let id = format!("s{:02}", dice.below(60));
        if !files.contains_key(&(account.to_owned(), id.clone())) {
            return id;
        }
    };
    let first = ("acc".to_owned(), name(dice, &files, "acc"));
    files.insert(first.clone(), Vec::new());
    order.push(first);
    for _ in 0..(6 + dice.below(14)) {
        clock += i64::try_from(dice.below(2)).unwrap();
        if dice.below(3) == 0 && order.len() < 9 {
            let from = order[usize::try_from(dice.below(order.len() as u64)).unwrap()].clone();
            let copy = files[&from].clone();
            let other = ACCOUNTS.iter().find(|a| **a != from.0).unwrap();
            let to = if dice.below(2) == 0
                && !files.contains_key(&((*other).to_owned(), from.1.clone()))
            {
                // A folder copied into the other account: the same file.
                ((*other).to_owned(), from.1.clone())
            } else {
                let account = ACCOUNTS[usize::try_from(dice.below(2)).unwrap()];
                (account.to_owned(), name(dice, &files, account))
            };
            files.insert(to.clone(), copy);
            order.push(to);
        } else {
            let writer = order[usize::try_from(dice.below(order.len() as u64)).unwrap()].clone();
            for _ in 0..(1 + dice.below(3)) {
                files.get_mut(&writer).unwrap().push((next_key, clock));
                next_key += 1;
            }
        }
    }
    files.retain(|_, messages| !messages.is_empty());
    files
}

fn event(account: &str, session: &str, key: u64, ts: i64) -> UsageEvent {
    UsageEvent {
        account: account.into(),
        session: Some(session.into()),
        ts,
        model: None,
        tokens: TokenCounts {
            output: key,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Measured,
        detail: juggler_core::quota::UsageDetail::default(),
        msg_uuid: Some(format!("m{key}")),
    }
}

fn events_of(files: &Files, place: &Place) -> Vec<UsageEvent> {
    files[place]
        .iter()
        .map(|(key, ts)| event(&place.0, &place.1, *key, *ts))
        .collect()
}

/// The rule, read straight from every account's holders.
fn oracle(files: &Files) -> BTreeMap<u64, Place> {
    let mut holders: BTreeMap<u64, BTreeSet<&Place>> = BTreeMap::new();
    for (place, messages) in files {
        for (key, _) in messages {
            holders.entry(*key).or_default().insert(place);
        }
    }
    let rank = |place: &Place| {
        let messages = &files[place];
        let first = messages.iter().map(|m| m.1).min().unwrap();
        let last = messages.iter().map(|m| m.1).max().unwrap();
        let own = messages
            .iter()
            .filter(|(key, _)| holders[key].len() == 1)
            .map(|m| m.1)
            .min();
        (
            first,
            std::cmp::Reverse(own.unwrap_or(i64::MAX)),
            if own.is_none() { last } else { 0 },
            place.1.clone(),
            place.0.clone(),
        )
    };
    holders
        .iter()
        .map(|(key, places)| {
            let keeper = places.iter().min_by_key(|p| rank(p)).unwrap();
            (*key, (*keeper).clone())
        })
        .collect()
}

/// Every stored call, where it's kept; a call stored twice fails.
fn kept(storage: &Storage) -> BTreeMap<u64, Place> {
    let mut out = BTreeMap::new();
    for account in ACCOUNTS {
        for row in storage.usage().events_for_account(account, 0).unwrap() {
            let key: u64 = row.msg_uuid.unwrap()[1..].parse().unwrap();
            let was = out.insert(key, (account.to_owned(), row.session.unwrap()));
            assert!(was.is_none(), "m{key} counted in two accounts");
        }
    }
    out
}

/// Each file cut into one to three slices, read in a random order.
fn passes(files: &Files, dice: &mut Dice) -> Vec<Vec<UsageEvent>> {
    let mut slices: Vec<Vec<UsageEvent>> = Vec::new();
    for place in files.keys() {
        let events = events_of(files, place);
        let mut cuts: Vec<usize> = (0..dice.below(3))
            .map(|_| usize::try_from(dice.below(events.len() as u64 + 1)).unwrap())
            .collect();
        cuts.push(0);
        cuts.push(events.len());
        cuts.sort_unstable();
        for pair in cuts.windows(2) {
            slices.push(events[pair[0]..pair[1]].to_vec());
        }
    }
    for at in (1..slices.len()).rev() {
        let other = usize::try_from(dice.below(at as u64 + 1)).unwrap();
        slices.swap(at, other);
    }
    let mut batches = Vec::new();
    let mut rest = slices.as_slice();
    while !rest.is_empty() {
        let take = (1 + usize::try_from(dice.below(3)).unwrap()).min(rest.len());
        batches.push(rest[..take].concat());
        rest = &rest[take..];
    }
    batches
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn copies_across_accounts_count_once_where_the_rule_says(seed in any::<u64>()) {
        let mut dice = Dice(seed);
        let files = history(&mut dice);
        let want = oracle(&files);
        let total: u64 = want.keys().sum();

        let storage = Storage::open_in_memory().unwrap();
        for batch in passes(&files, &mut dice) {
            storage.usage().store_events(&batch).unwrap();
        }
        let got = kept(&storage);
        prop_assert_eq!(&got, &want, "files {:?}", files);
        prop_assert_eq!(got.keys().sum::<u64>(), total);

        // A re-read of every file, a few at a time: never a call twice or
        // missing, and the same keepers after.
        storage.usage().forget_standings().unwrap();
        let places: Vec<&Place> = files.keys().collect();
        for chunk in places.chunks(1 + usize::try_from(dice.below(3)).unwrap()) {
            let keys: Vec<(String, UsageSource, String)> = chunk
                .iter()
                .map(|p| (p.0.clone(), UsageSource::Transcript, p.1.clone()))
                .collect();
            let events: Vec<UsageEvent> =
                chunk.iter().flat_map(|p| events_of(&files, p)).collect();
            storage.usage().replace_session_events(&keys, &events).unwrap();
            prop_assert_eq!(kept(&storage).keys().sum::<u64>(), total, "while re-reading");
        }
        storage.usage().settle_copies().unwrap();
        prop_assert_eq!(kept(&storage), want, "after the re-read");
        prop_assert!(!storage.usage().derived_dirty().unwrap());
    }
}

/// `(account, session, messages)`.
type FileSpec<'a> = (&'a str, &'a str, &'a [(u64, i64)]);

fn files(list: &[FileSpec]) -> Files {
    list.iter()
        .map(|(account, session, messages)| {
            (
                ((*account).to_owned(), (*session).to_owned()),
                messages.to_vec(),
            )
        })
        .collect()
}

fn store_all(storage: &Storage, files: &Files) {
    for place in files.keys() {
        storage
            .usage()
            .store_events(&events_of(files, place))
            .unwrap();
    }
}

fn total_of(storage: &Storage, account: &str) -> u64 {
    storage
        .usage()
        .events_for_account(account, 0)
        .unwrap()
        .iter()
        .map(|row| row.tokens.output)
        .sum()
}

/// The 2026-11-04 finding: a conversation copied by hand into a second
/// account, with no switch recorded, counted under both. It counts once,
/// under the original; the copy's own new messages under the copy's
/// account. A re-read changes nothing.
#[test]
fn a_copy_in_another_account_counts_once_and_its_own_messages_count_there() {
    // The original wrote 1..3, was copied (same file) into bcc, and only
    // the copy went on (4, 5).
    let history = files(&[
        ("acc", "s1", &[(1, 0), (2, 1), (3, 2)]),
        ("bcc", "s1", &[(1, 0), (2, 1), (3, 2), (4, 10), (5, 11)]),
    ]);
    let storage = Storage::open_in_memory().unwrap();
    store_all(&storage, &history);
    assert_eq!(total_of(&storage, "acc"), 1 + 2 + 3, "the shared history");
    assert_eq!(total_of(&storage, "bcc"), 4 + 5, "the copy's own");
    // Read in the other order: the same.
    let other = Storage::open_in_memory().unwrap();
    for place in history.keys().rev() {
        other
            .usage()
            .store_events(&events_of(&history, place))
            .unwrap();
    }
    assert_eq!(total_of(&other, "acc"), 6);
    assert_eq!(total_of(&other, "bcc"), 9);
    // A re-read of both files changes nothing.
    storage.usage().forget_standings().unwrap();
    for place in history.keys() {
        storage
            .usage()
            .replace_session_events(
                &[(place.0.clone(), UsageSource::Transcript, place.1.clone())],
                &events_of(&history, place),
            )
            .unwrap();
    }
    storage.usage().settle_copies().unwrap();
    assert_eq!(total_of(&storage, "acc"), 6);
    assert_eq!(total_of(&storage, "bcc"), 9);
}

/// What 0.3.7 left in the store: the same calls stored under both
/// accounts. Settling after the re-read keeps one, under the original,
/// even when a file is gone (no re-read reaches it).
#[test]
fn copies_two_accounts_stored_before_are_settled_into_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&path).unwrap();
    store_all(
        &storage,
        &files(&[("acc", "s1", &[(1, 0), (2, 1)]), ("acc", "s2", &[(3, 5)])]),
    );
    // 0.3.7 stored the copy's rows in bcc as well, with its holders.
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "INSERT INTO usage_events (account, session, ts, model, input, output, cache_r, cache_w,
                 reasoning, credits, usd_est, source, confidence, msg_uuid)
             SELECT 'bcc', session, ts, model, input, output, cache_r, cache_w, reasoning,
                 credits, usd_est, source, confidence, msg_uuid
             FROM usage_events WHERE account = 'acc' AND session = 's1';
             INSERT INTO usage_holds (account, msg, session, ts)
             SELECT 'bcc', msg, session, ts FROM usage_holds
             WHERE account = 'acc' AND session = (SELECT id FROM usage_keys WHERE key = 's1');
             INSERT INTO usage_events (account, session, ts, output, source, confidence, msg_uuid)
             VALUES ('bcc', 's1', 9, 4, 'transcript', 'measured', 'm4');
             INSERT OR IGNORE INTO usage_keys (key) VALUES ('m4'), ('s1');
             INSERT INTO usage_holds (account, msg, session, ts)
             SELECT 'bcc', m.id, s.id, 9 FROM usage_keys m, usage_keys s
             WHERE m.key = 'm4' AND s.key = 's1';",
        )
        .unwrap();
    }
    assert_eq!(total_of(&storage, "bcc"), 1 + 2 + 4, "counted twice");
    storage.usage().forget_standings().unwrap();
    assert_eq!(storage.usage().settle_copies().unwrap(), 2);
    assert_eq!(total_of(&storage, "acc"), 1 + 2 + 3);
    assert_eq!(total_of(&storage, "bcc"), 4, "only its own");
    assert_eq!(storage.usage().settle_copies().unwrap(), 2, "settled again");
    assert_eq!(total_of(&storage, "acc"), 6, "the same");
}

/// Codex's keys name their own thread, so its accounts keep no holders:
/// a rollout copied into another account counts once (where it was stored
/// first), and the copy's own new turns count there.
#[test]
fn a_codex_rollout_copied_into_another_account_counts_once() {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .usage()
        .set_unshared_accounts(&["acc".to_owned(), "bcc".to_owned()].into())
        .unwrap();
    let codex = |account: &str, key: &str, output: u64| UsageEvent {
        source: UsageSource::Rollout,
        msg_uuid: Some(key.to_owned()),
        tokens: TokenCounts {
            output,
            ..TokenCounts::default()
        },
        ..event(account, "t1", 0, 0)
    };
    storage
        .usage()
        .store_events(&[codex("acc", "t1/u1/r1", 10), codex("acc", "t1/u2/r2", 20)])
        .unwrap();
    storage
        .usage()
        .store_events(&[
            codex("bcc", "t1/u1/r1", 10),
            codex("bcc", "t1/u2/r2", 20),
            codex("bcc", "t1/u3/r3", 5),
        ])
        .unwrap();
    assert_eq!(total_of(&storage, "acc"), 30);
    assert_eq!(total_of(&storage, "bcc"), 5);
    // A re-read of the copy changes nothing.
    storage
        .usage()
        .replace_session_events(
            &[("bcc".to_owned(), UsageSource::Rollout, "t1".to_owned())],
            &[
                codex("bcc", "t1/u1/r1", 10),
                codex("bcc", "t1/u2/r2", 20),
                codex("bcc", "t1/u3/r3", 5),
            ],
        )
        .unwrap();
    assert_eq!(total_of(&storage, "acc"), 30);
    assert_eq!(total_of(&storage, "bcc"), 5);
}

/// One key from another reader is never taken for the same call.
#[test]
fn another_readers_row_is_never_merged() {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .usage()
        .store_events(&[event("acc", "s1", 1, 0)])
        .unwrap();
    let other = UsageEvent {
        source: UsageSource::Rollout,
        ..event("bcc", "s9", 1, 0)
    };
    storage.usage().store_events(&[other]).unwrap();
    storage.usage().settle_copies().unwrap();
    assert_eq!(total_of(&storage, "acc"), 1);
    assert_eq!(total_of(&storage, "bcc"), 1);
}

/// QA on a copy of a real store (never the app's own file): settles the
/// copies two accounts hold, as 0.3.8's one-time re-read ends, and says
/// how many. Ignored, and a no-op unless `AIJ_QA_COPIES_DB` names the copy.
#[test]
#[ignore = "QA on a copy of a real store; needs AIJ_QA_COPIES_DB"]
fn settle_the_copies_in_a_copy_of_a_real_store() {
    let Some(path) = std::env::var_os("AIJ_QA_COPIES_DB") else {
        eprintln!("AIJ_QA_COPIES_DB not set: nothing to do");
        return;
    };
    let path = std::path::PathBuf::from(path);
    assert!(
        !path
            .to_string_lossy()
            .contains("Application Support/AI Juggler"),
        "a copy only, never the app's own store"
    );
    let storage = Storage::open(&path).unwrap();
    let started = std::time::Instant::now();
    storage.usage().forget_standings().unwrap();
    let settled = storage.usage().settle_copies().unwrap();
    println!("settled {settled} calls in {:?}", started.elapsed());
}
