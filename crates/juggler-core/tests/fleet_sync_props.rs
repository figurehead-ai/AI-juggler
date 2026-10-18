//! AJ-E708 quality gates for the Fleet outbox and replica:
//! - shuffled and duplicated delivery converges to the origin's latest
//!   state per record;
//! - a console killed after any prefix of a catch-up resumes from its
//!   durable high-water mark and loses zero records;
//! - a peer clock skewed by up to ±10 minutes never changes the order —
//!   the sequence wins, the wall clock is display-only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::BTreeMap;

use juggler_core::storage::{Storage, SyncEntry};
use proptest::prelude::*;

const ORIGIN: &str = "mac-a";
const EPOCH: &str = "epoch-1";

/// Edits on the origin: (record index, payload byte, wall-clock skew ms).
fn edits() -> impl Strategy<Value = Vec<(u8, u8, i64)>> {
    prop::collection::vec((0_u8..8, any::<u8>(), -600_000_i64..=600_000), 1..120)
}

/// Runs `edits` through a node's outbox; returns the outbox (oldest
/// first) and the expected latest payload per record.
fn origin_outbox(edits: &[(u8, u8, i64)]) -> (Vec<SyncEntry>, BTreeMap<String, Vec<u8>>) {
    let node = Storage::open_in_memory().unwrap();
    let sync = node.fleet_sync();
    let mut expected = BTreeMap::new();
    for (index, (record, value, skew)) in edits.iter().enumerate() {
        let id = format!("inst-{record}");
        let payload = vec![*record, *value, u8::try_from(index % 251).unwrap()];
        // The wall clock jumps around; only the sequence is trusted.
        let wall = 1_000_000_000 + i64::try_from(index).unwrap() * 1_000 + skew;
        sync.outbox_append("instance", &id, &payload, wall).unwrap();
        expected.insert(id, payload);
    }
    (sync.outbox_since(0, 100_000).unwrap(), expected)
}

fn replica_state(storage: &Storage) -> BTreeMap<String, Vec<u8>> {
    storage
        .fleet_sync()
        .replica_list(ORIGIN, "instance")
        .unwrap()
        .into_iter()
        .map(|(entry, _)| (entry.record_id, entry.payload))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn shuffled_duplicated_delivery_converges(
        edits in edits(),
        order in Just(()).prop_flat_map(|()| prop::collection::vec(any::<prop::sample::Index>(), 0..200)),
    ) {
        let (outbox, expected) = origin_outbox(&edits);
        // Every delta once, plus random duplicates, in a random order.
        let mut delivery: Vec<&SyncEntry> = outbox.iter().collect();
        delivery.extend(order.iter().map(|index| index.get(&outbox)));
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64 ^ u64::try_from(delivery.len()).unwrap();
        for i in (1..delivery.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let j = usize::try_from(seed % u64::try_from(i + 1).unwrap()).unwrap();
            delivery.swap(i, j);
        }
        let console = Storage::open_in_memory().unwrap();
        console.fleet_sync().replica_set_epoch(ORIGIN, EPOCH).unwrap();
        for entry in delivery {
            console.fleet_sync().replica_apply(ORIGIN, entry, 1).unwrap();
        }
        prop_assert_eq!(replica_state(&console), expected);
        // Replaying the whole batch again changes nothing.
        for entry in &outbox {
            prop_assert!(!console.fleet_sync().replica_apply(ORIGIN, entry, 2).unwrap());
        }
    }

    #[test]
    fn a_kill_after_any_prefix_resumes_and_loses_nothing(
        edits in edits(),
        cut in any::<prop::sample::Index>(),
    ) {
        let (outbox, expected) = origin_outbox(&edits);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("console.db");
        let applied = cut.index(outbox.len() + 1);
        {
            let console = Storage::open(&path).unwrap();
            console.fleet_sync().replica_set_epoch(ORIGIN, EPOCH).unwrap();
            for entry in &outbox[..applied] {
                console.fleet_sync().replica_apply(ORIGIN, entry, 1).unwrap();
            }
            // Killed here: the process goes, nothing else is flushed.
        }
        let console = Storage::open(&path).unwrap();
        let sync = console.fleet_sync();
        sync.replica_set_epoch(ORIGIN, EPOCH).unwrap();
        let mark = sync.replica_high_water(ORIGIN, EPOCH).unwrap();
        let resumed: Vec<&SyncEntry> = outbox.iter().filter(|entry| entry.seq > mark).collect();
        prop_assert_eq!(
            resumed.len(),
            outbox.len() - applied,
            "the catch-up is exactly what the kill cut off"
        );
        for entry in resumed {
            sync.replica_apply(ORIGIN, entry, 2).unwrap();
        }
        prop_assert_eq!(replica_state(&console), expected);
    }

    #[test]
    fn a_skewed_clock_never_reorders(
        edits in edits(),
    ) {
        let (outbox, expected) = origin_outbox(&edits);
        // Deliver in wall-clock order (what trusting a skewed clock
        // would do): the replica still ends at each record's highest
        // sequence.
        let mut by_wall: Vec<&SyncEntry> = outbox.iter().collect();
        by_wall.sort_by_key(|entry| entry.wall_ms);
        let console = Storage::open_in_memory().unwrap();
        console.fleet_sync().replica_set_epoch(ORIGIN, EPOCH).unwrap();
        for entry in by_wall {
            console.fleet_sync().replica_apply(ORIGIN, entry, 1).unwrap();
        }
        prop_assert_eq!(replica_state(&console), expected);
    }
}
