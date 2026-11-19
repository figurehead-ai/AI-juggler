//! The store and its log (ADR-0036 §2.7, ADR-0037): written only by the
//! user's commands through the watches' own writer; the engine's fires
//! and endings go to the log only.

use juggler_core::storage::Storage;
use juggler_core::storage::memory::{self, Writers};

use super::*;
use crate::features::ai::memory::RedactionSecrets;
use crate::features::watches::store::{Origin, StoreError, WatchStore};

fn installed() -> Storage {
    Storage::open_in_memory().unwrap()
}

fn verbs(storage: &Storage, id: &str) -> Vec<String> {
    let mut events = storage.ai_log().of_subject("watch", id, 100).unwrap();
    events.reverse();
    events.into_iter().map(|e| e.verb).collect()
}

fn records(storage: &Storage) -> Vec<(String, i64, String)> {
    let mut stmt = storage
        .raw_conn()
        .prepare("SELECT subject_key, version, payload FROM memory_records ORDER BY subject_key, version")
        .unwrap();
    stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn sample(id: &str) -> Watch {
    watch(
        id,
        vec![session("api")],
        vec![cond("finishes"), cond_p("stuck", Param::Minutes(10))],
        Lifetime::UntilEnds,
    )
}

#[test]
fn every_change_is_a_version_and_a_logged_event() {
    let storage = installed();
    let registry = Registry::shipped();
    let writers = Writers::for_tests();
    let store = WatchStore::new(&storage, &registry);
    let id = WatchId("w-5a1".into());
    let w = sample(&id.0);
    store
        .create(
            &writers.watches,
            &RedactionSecrets,
            &w,
            &Origin::WatchesView,
            T,
        )
        .unwrap();
    let mut edited = w.clone();
    edited.even_in_quiet_hours = true;
    let v2 = store
        .update(
            &writers.watches,
            &RedactionSecrets,
            &edited,
            &Origin::ChatTurn("turn-1".into()),
            T + 1,
        )
        .unwrap();
    assert_eq!(v2.version, 2);
    store
        .set_paused(
            &writers.watches,
            &RedactionSecrets,
            &id,
            true,
            &Origin::WatchesView,
            T + 2,
        )
        .unwrap();
    store
        .set_paused(
            &writers.watches,
            &RedactionSecrets,
            &id,
            false,
            &Origin::WatchesView,
            T + 3,
        )
        .unwrap();
    let current = store.get(&id).unwrap().unwrap();
    assert_eq!(current.version, 4);
    assert!(current.watch.even_in_quiet_hours && !current.watch.paused);
    assert_eq!(store.list().unwrap().len(), 1);
    store
        .remove(
            &writers.watches,
            &RedactionSecrets,
            &id,
            &Origin::WatchesView,
            T + 4,
        )
        .unwrap();
    assert!(
        store.list().unwrap().is_empty(),
        "a removed watch leaves the list"
    );
    assert!(
        store.get(&id).unwrap().unwrap().removed,
        "and is kept, superseded, for memory"
    );
    assert_eq!(
        verbs(&storage, &id.0),
        ["created", "edited", "paused", "resumed", "removed"]
    );
    assert!(matches!(
        store.remove(
            &writers.watches,
            &RedactionSecrets,
            &id,
            &Origin::WatchesView,
            T + 5
        ),
        Err(StoreError::NotFound)
    ));
    // The record is the user's, and holds no agent-written name.
    let record = storage.memory().current("watch:w-5a1").unwrap().unwrap();
    assert_eq!(record.authority, memory::Authority::User);
    assert!(!record.untrusted);
    assert_eq!(
        record.text,
        "Tell you when a session finishes, or looks stuck for 10 minutes. Until it ends. Even during quiet hours."
    );
}

#[test]
fn the_store_refuses_what_the_model_refuses() {
    let storage = installed();
    let registry = Registry::shipped();
    let writers = Writers::for_tests();
    let store = WatchStore::new(&storage, &registry);
    let mut bad = sample("w-5b1");
    bad.conditions = vec![cond("tests_pass")];
    assert!(matches!(
        store.create(
            &writers.watches,
            &RedactionSecrets,
            &bad,
            &Origin::WatchesView,
            T
        ),
        Err(StoreError::Invalid(WatchError::UnknownCondition(_)))
    ));
    assert!(records(&storage).is_empty());
    let good = sample("w-5b2");
    store
        .create(
            &writers.watches,
            &RedactionSecrets,
            &good,
            &Origin::WatchesView,
            T,
        )
        .unwrap();
    assert!(matches!(
        store.create(
            &writers.watches,
            &RedactionSecrets,
            &good,
            &Origin::WatchesView,
            T
        ),
        Err(StoreError::Exists)
    ));
    // Another module's writer can't write a watch.
    let mut other = sample("w-5b3");
    other.id = WatchId("w-5b3".into());
    assert!(matches!(
        store.create(
            &writers.memory,
            &RedactionSecrets,
            &other,
            &Origin::WatchesView,
            T
        ),
        Err(StoreError::Memory(memory::MemoryError::Refused(
            memory::Refusal::WrongWriter
        )))
    ));
}

#[test]
fn at_most_fifty_watches_are_active() {
    let storage = installed();
    let registry = Registry::shipped();
    let writers = Writers::for_tests();
    let store = WatchStore::new(&storage, &registry);
    for i in 0..MAX_ACTIVE {
        store
            .create(
                &writers.watches,
                &RedactionSecrets,
                &sample(&format!("w-{i:x}c")),
                &Origin::WatchesView,
                T,
            )
            .unwrap();
    }
    assert!(matches!(
        store.create(
            &writers.watches,
            &RedactionSecrets,
            &sample("w-ffc"),
            &Origin::WatchesView,
            T
        ),
        Err(StoreError::Invalid(WatchError::TooMany))
    ));
    // An ended one makes room.
    store
        .log_end(&super::super::engine::Expiry {
            watch: WatchId("w-0c".into()),
            reason: EndReason::TimeReached,
            at_ms: T + 1,
        })
        .unwrap();
    store
        .create(
            &writers.watches,
            &RedactionSecrets,
            &sample("w-ffc"),
            &Origin::WatchesView,
            T + 2,
        )
        .unwrap();
}

#[test]
fn without_the_memory_store_every_command_says_so() {
    let storage = Storage::open_in_memory().unwrap();
    // A database without V54's store (one restored from before it).
    storage
        .raw_conn()
        .execute_batch("DROP TABLE memory_records;")
        .unwrap();
    let registry = Registry::shipped();
    let writers = Writers::for_tests();
    let store = WatchStore::new(&storage, &registry);
    assert!(matches!(store.list(), Err(StoreError::Unavailable)));
    assert!(matches!(
        store.create(
            &writers.watches,
            &RedactionSecrets,
            &sample("w-6a"),
            &Origin::WatchesView,
            T
        ),
        Err(StoreError::Unavailable)
    ));
}

/// Nothing the engine does writes a watch: a whole replay's fires and
/// endings, logged, leave every record as it was; only the log grows,
/// with `fired`, `expired` and `ended`. And the engine's code holds no
/// way to: it names no store, writer or storage.
#[test]
fn no_evaluator_path_writes_a_watch() {
    let storage = installed();
    let registry = Registry::shipped();
    let writers = Writers::for_tests();
    let store = WatchStore::new(&storage, &registry);
    let replay: Replay = serde_json::from_str(include_str!("../fixtures/replay.json")).unwrap();
    for w in &replay.watches {
        store
            .create(
                &writers.watches,
                &RedactionSecrets,
                w,
                &Origin::WatchesView,
                replay.start_ms,
            )
            .unwrap();
    }
    let before = records(&storage);
    let out = run_replay(&replay);
    for fire in &out.fires {
        store.log_fire(fire).unwrap();
    }
    for expiry in &out.ended {
        store.log_end(expiry).unwrap();
    }
    assert_eq!(
        records(&storage),
        before,
        "the engine's outcome changed a watch"
    );
    let ended: Vec<String> = verbs(&storage, "w-a3");
    assert_eq!(ended, ["created", "fired", "expired"]);
    assert_eq!(
        verbs(&storage, "w-a5").last().map(String::as_str),
        Some("ended")
    );
    assert_eq!(store.fired_count(&WatchId("w-a1".into())).unwrap(), 3);
    assert_eq!(
        store
            .ended(&WatchId("w-a5".into()))
            .unwrap()
            .map(|(r, _)| r),
        Some(EndReason::TargetGone)
    );
    assert_eq!(store.active().unwrap().len(), 1, "w-a2 is left, always");
    for source in [
        include_str!("../engine.rs"),
        include_str!("../conditions/mod.rs"),
        include_str!("../catalogue.rs"),
        include_str!("../echo.rs"),
    ] {
        for word in ["KindWriter", "WatchStore", "Storage", "remember"] {
            assert!(!source.contains(word), "an evaluator file names {word}");
        }
    }
    let runtime = include_str!("../runtime.rs");
    for call in [
        "store.create(",
        "store.update(",
        "store.set_paused(",
        "store.remove(",
        "remember",
    ] {
        assert!(!runtime.contains(call), "the runtime calls {call}");
    }
}

/// A watch's ending shows why; an edit after it renews it.
#[test]
fn an_ended_watch_says_why_and_an_edit_renews_it() {
    let storage = installed();
    let registry = Registry::shipped();
    let writers = Writers::for_tests();
    let store = WatchStore::new(&storage, &registry);
    let w = sample("w-7a");
    store
        .create(
            &writers.watches,
            &RedactionSecrets,
            &w,
            &Origin::WatchesView,
            T,
        )
        .unwrap();
    store
        .log_end(&super::super::engine::Expiry {
            watch: w.id.clone(),
            reason: EndReason::TargetGone,
            at_ms: T + 5,
        })
        .unwrap();
    assert_eq!(
        store.ended(&w.id).unwrap(),
        Some((EndReason::TargetGone, T + 5))
    );
    assert!(store.active().unwrap().is_empty());
    let mut renewed = w.clone();
    renewed.targets = vec![session("web")];
    store
        .update(
            &writers.watches,
            &RedactionSecrets,
            &renewed,
            &Origin::WatchesView,
            T + 6,
        )
        .unwrap();
    assert_eq!(store.ended(&w.id).unwrap(), None);
    assert_eq!(store.active().unwrap().len(), 1);
}

/// The list is read a keyset page at a time, newest change first.
#[test]
fn the_list_is_paged() {
    let storage = installed();
    let registry = Registry::shipped();
    let writers = Writers::for_tests();
    let store = WatchStore::new(&storage, &registry);
    for (i, id) in ["w-8a", "w-8b", "w-8c"].iter().enumerate() {
        let at = T + i64::try_from(i).unwrap();
        store
            .create(
                &writers.watches,
                &RedactionSecrets,
                &sample(id),
                &Origin::WatchesView,
                at,
            )
            .unwrap();
    }
    let (first, next) = store.page(None, 2).unwrap();
    let ids: Vec<&str> = first.iter().map(|s| s.watch.id.0.as_str()).collect();
    assert_eq!(ids, ["w-8c", "w-8b"]);
    let (rest, end) = store.page(next.as_ref(), 2).unwrap();
    assert_eq!(
        rest.iter()
            .map(|s| s.watch.id.0.as_str())
            .collect::<Vec<_>>(),
        ["w-8a"]
    );
    assert_eq!(end, None);
}
