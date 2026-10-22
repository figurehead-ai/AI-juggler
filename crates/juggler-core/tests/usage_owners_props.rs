//! Who keeps a shared message (#126 re-review L-f): random trees of
//! resumed copies and forks, read in random passes and orders, always keep
//! each message where a brute-force reading of the rule over the whole
//! holder set puts it, and the account's total is every message once.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
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

/// A small deterministic generator (splitmix64), driven by proptest's seed.
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

/// Each session's messages `(key, ts)`, as its file holds them at the end.
type Tree = BTreeMap<String, Vec<(u64, i64)>>;

/// A random history: sessions write messages (sometimes in the same
/// millisecond), and a session is resumed or forked from another's state at
/// that moment (copying its messages with their times); chains reach depth
/// four or more.
fn history(dice: &mut Dice) -> Tree {
    let mut tree: Tree = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut clock = 0_i64;
    let mut next_key = 1_u64;
    let name = |dice: &mut Dice, taken: &Tree| loop {
        let id = format!("s{:02}", dice.below(60));
        if !taken.contains_key(&id) {
            return id;
        }
    };
    let first = name(dice, &tree);
    tree.insert(first.clone(), Vec::new());
    order.push(first);
    for _ in 0..(6 + dice.below(14)) {
        clock += i64::try_from(dice.below(2)).unwrap(); // 0: the same millisecond
        if dice.below(3) == 0 && order.len() < 9 {
            // Resume or fork: most often the newest session (deep chains).
            let from = if dice.below(3) == 0 {
                order[usize::try_from(dice.below(order.len() as u64)).unwrap()].clone()
            } else {
                order.last().unwrap().clone()
            };
            let copy = tree[&from].clone();
            let id = name(dice, &tree);
            tree.insert(id.clone(), copy);
            order.push(id);
        } else {
            let at = usize::try_from(dice.below(order.len() as u64)).unwrap();
            let writer = order[at].clone();
            for _ in 0..(1 + dice.below(3)) {
                tree.get_mut(&writer).unwrap().push((next_key, clock));
                next_key += 1;
            }
        }
    }
    tree
}

fn event(session: &str, key: u64, ts: i64) -> UsageEvent {
    UsageEvent {
        account: "acc".into(),
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
        msg_uuid: Some(format!("m{key}")),
    }
}

/// The rule, read straight from the whole holder set (no increments).
fn oracle(tree: &Tree) -> BTreeMap<u64, String> {
    let mut holders: BTreeMap<u64, BTreeSet<&str>> = BTreeMap::new();
    for (session, messages) in tree {
        for (key, _) in messages {
            holders.entry(*key).or_default().insert(session);
        }
    }
    let rank = |session: &str| {
        let messages = &tree[session];
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
            session.to_owned(),
        )
    };
    holders
        .iter()
        .map(|(key, sessions)| {
            let keeper = sessions.iter().min_by_key(|s| rank(s)).unwrap();
            (*key, (*keeper).to_owned())
        })
        .collect()
}

fn kept(storage: &Storage) -> BTreeMap<u64, String> {
    storage
        .usage()
        .events_for_account("acc", 0)
        .unwrap()
        .into_iter()
        .map(|row| {
            (
                row.msg_uuid.unwrap()[1..].parse().unwrap(),
                row.session.unwrap(),
            )
        })
        .collect()
}

/// Each file cut into one to three slices, the slices read in a random
/// order, a random number of them per pass.
fn passes(tree: &Tree, dice: &mut Dice) -> Vec<Vec<UsageEvent>> {
    let mut slices: Vec<Vec<UsageEvent>> = Vec::new();
    for (session, messages) in tree {
        let events: Vec<UsageEvent> = messages
            .iter()
            .map(|(key, ts)| event(session, *key, *ts))
            .collect();
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
    fn any_tree_read_in_any_passes_keeps_what_the_rule_says(seed in any::<u64>()) {
        let mut dice = Dice(seed);
        let tree = history(&mut dice);
        let want = oracle(&tree);
        let total: u64 = want.keys().sum();

        let storage = Storage::open_in_memory().unwrap();
        for batch in passes(&tree, &mut dice) {
            storage.usage().store_events(&batch).unwrap();
        }
        let got = kept(&storage);
        prop_assert_eq!(&got, &want, "tree {:?}", tree);
        prop_assert_eq!(got.keys().sum::<u64>(), total);

        // A fresh re-read of every session, a few at a time.
        let sessions: Vec<&String> = tree.keys().collect();
        for chunk in sessions.chunks(1 + usize::try_from(dice.below(3)).unwrap()) {
            let keys: Vec<(String, UsageSource, String)> = chunk
                .iter()
                .map(|s| ("acc".to_owned(), UsageSource::Transcript, (*s).clone()))
                .collect();
            let events: Vec<UsageEvent> = chunk
                .iter()
                .flat_map(|s| tree[*s].iter().map(|(key, ts)| event(s, *key, *ts)))
                .collect();
            storage.usage().replace_session_events(&keys, &events).unwrap();
            let after: u64 = kept(&storage).keys().sum();
            prop_assert_eq!(after, total, "the total while re-reading");
        }
        prop_assert_eq!(kept(&storage), want, "after the re-read");
    }
}

/// The real upgrade: a version-2 store (no holders, a copy credited with its
/// original's history), re-read a chunk at a time with one session's file
/// gone. Every present session ends with its own; the missing one keeps
/// what it had; the total never changes.
#[test]
fn a_version_two_store_re_read_in_chunks_with_a_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&path).unwrap();
    let files: Tree = [
        ("o".to_owned(), vec![(1, 0), (2, 1)]),
        ("c".to_owned(), vec![(1, 0), (2, 1), (3, 2), (4, 3)]),
        (
            "d".to_owned(),
            vec![(1, 0), (2, 1), (3, 2), (4, 3), (5, 4), (6, 5)],
        ),
        ("m".to_owned(), vec![(7, 6), (8, 7)]),
    ]
    .into_iter()
    .collect();
    for (session, messages) in &files {
        let events: Vec<UsageEvent> = messages
            .iter()
            .map(|(key, ts)| event(session, *key, *ts))
            .collect();
        storage.usage().store_events(&events).unwrap();
    }
    // Make it version 2's: no holders, no standings, and the copy credited
    // with the shared history.
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "DELETE FROM usage_holders; DELETE FROM usage_spans;
             UPDATE usage_events SET session = 'c' WHERE msg_uuid IN ('m1', 'm2', 'm3', 'm4');",
        )
        .unwrap();
    }
    let total: u64 = kept(&storage).keys().sum();
    assert_eq!(total, 36);
    // The re-read: m's file is gone; the rest in two chunks.
    for chunk in [vec!["o"], vec!["c", "d"]] {
        let keys: Vec<(String, UsageSource, String)> = chunk
            .iter()
            .map(|s| ("acc".to_owned(), UsageSource::Transcript, (*s).to_owned()))
            .collect();
        let events: Vec<UsageEvent> = chunk
            .iter()
            .flat_map(|s| files[*s].iter().map(|(key, ts)| event(s, *key, *ts)))
            .collect();
        storage
            .usage()
            .replace_session_events(&keys, &events)
            .unwrap();
        assert_eq!(kept(&storage).keys().sum::<u64>(), total, "after {chunk:?}");
    }
    let mut by_session: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for (key, session) in kept(&storage) {
        by_session.entry(session).or_default().push(key);
    }
    let want: BTreeMap<String, Vec<u64>> = [
        ("o", vec![1, 2]),
        ("c", vec![3, 4]),
        ("d", vec![5, 6]),
        ("m", vec![7, 8]),
    ]
    .into_iter()
    .map(|(s, keys)| (s.to_owned(), keys))
    .collect();
    assert_eq!(by_session, want);
}
