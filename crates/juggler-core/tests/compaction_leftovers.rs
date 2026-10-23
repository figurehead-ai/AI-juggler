//! P21's leftovers: a Codex compaction row an older build stored for
//! another thread's model call, in a session whose rollout is gone. Only
//! such a row goes; a gone file's other rows, and every row of a file still
//! there, stay.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{UsageEvent, UsageSource};
use juggler_core::storage::Storage;

const PARENT: &str = "00000000-0000-4000-8000-00000000000a";
const GONE: &str = "00000000-0000-4000-8000-00000000000b";
const GONE_REAL: &str = "00000000-0000-4000-8000-00000000000c";
const HERE: &str = "00000000-0000-4000-8000-00000000000d";

fn row(session: &str, key: &str, input: u64, output: u64) -> UsageEvent {
    UsageEvent {
        account: "acc".into(),
        session: Some(session.into()),
        ts: 1_792_576_801_000,
        model: None,
        tokens: TokenCounts {
            input,
            output,
            ..Default::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Rollout,
        confidence: Confidence::Measured,
        msg_uuid: Some(key.into()),
    }
}

fn stored(storage: &Storage) -> BTreeMap<String, u64> {
    storage
        .usage()
        .events_for_account("acc", 0)
        .unwrap()
        .into_iter()
        .map(|event| (event.msg_uuid.unwrap(), event.tokens.input))
        .collect()
}

#[test]
fn only_a_gone_sessions_copy_of_a_stored_call_goes() {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .usage()
        .store_events(&[
            // The call, counted where it was written.
            row(PARENT, &format!("{PARENT}/tp/r-compact"), 136_658, 74),
            // An older build's copy of it, in a thread whose rollout is gone.
            row(
                GONE,
                &format!("{GONE}/compacted/1792576804000/1"),
                136_658,
                74,
            ),
            // A real compaction of a gone file: no record has its tokens.
            row(
                GONE_REAL,
                &format!("{GONE_REAL}/compacted/1792576805000/1"),
                900,
                3,
            ),
            // The same tokens as the call, but its file is still here.
            row(
                HERE,
                &format!("{HERE}/compacted/1792576806000/1"),
                136_658,
                74,
            ),
            // Same input, other classes differ: not the call.
            row(
                GONE,
                &format!("{GONE}/compacted/1792576807000/2"),
                136_658,
                75,
            ),
            // A gone thread's own record, even with the same tokens: only
            // compaction rows are ever looked at.
            row(GONE, &format!("{GONE}/t1/r-t1"), 136_658, 74),
            // A live thread's real compaction (no record has its tokens).
            row(HERE, &format!("{HERE}/compacted/1792576808000/2"), 500, 1),
        ])
        .unwrap();
    let present = BTreeSet::from([PARENT.to_owned(), HERE.to_owned()]);
    let gone = storage
        .usage()
        .forget_duplicated_compactions("acc", &present)
        .unwrap();
    assert_eq!(gone, 1);
    // Nothing else goes: every other row, by key, is still there.
    let keys: BTreeSet<String> = stored(&storage).into_keys().collect();
    let kept = BTreeSet::from([
        format!("{PARENT}/tp/r-compact"),
        format!("{GONE_REAL}/compacted/1792576805000/1"),
        format!("{HERE}/compacted/1792576806000/1"),
        format!("{GONE}/compacted/1792576807000/2"),
        format!("{GONE}/t1/r-t1"),
        format!("{HERE}/compacted/1792576808000/2"),
    ]);
    assert_eq!(keys, kept);
    // Again: nothing more to do.
    assert_eq!(
        storage
            .usage()
            .forget_duplicated_compactions("acc", &present)
            .unwrap(),
        0
    );
}
