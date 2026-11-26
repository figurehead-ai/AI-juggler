#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test: a failed step fails the test"
)]

use super::*;
use crate::storage::memory::world as w;
use gyrus::{Batch, Graph, KindSpec, Label, LinkSpec, NewLink, NewNode, Pack};

fn pack() -> Pack {
    Pack::builder()
        .kind(KindSpec::new("memory"))
        .kind(KindSpec::new("session").not_searchable())
        .kind(KindSpec::new("turn_summary").not_searchable())
        .kind(KindSpec::new("topic").not_searchable())
        .link(LinkSpec::new("about", &["memory"], &["session"]))
        .link(LinkSpec::new("part_of", &["turn_summary"], &["topic"]))
        .build()
        .unwrap()
}

fn graph(storage: &Storage) -> Graph {
    let graph = Graph::new(pack());
    graph.build(&storage.conn).unwrap();
    graph
}

fn world_writer() -> KindWriter {
    Writers::for_tests().world
}

fn sessions(n: usize) -> Batch {
    let mut b = Batch::new();
    for i in 0..n {
        b.node(NewNode::new(
            "session",
            &format!("session:s{i}"),
            Label::none(),
            NOW,
        ));
    }
    b
}

fn generation(storage: &Storage) -> i64 {
    gyrus::store::generation(&storage.conn).unwrap()
}

#[test]
fn v56_holds_the_world_model_tables() {
    let storage = installed_storage();
    assert!(w::installed(&storage.conn).unwrap());
    assert_eq!(Storage::schema_version(), 56);
    assert_eq!(Storage::migrations()[55], gyrus::SCHEMA_V1);
}

/// Every world write passes the gate: the writer, the pack, no secrets;
/// a refusal writes nothing but its log event, which names no words.
#[test]
fn the_world_gate_refuses_and_logs_without_the_words() {
    let storage = installed_storage();
    let graph = graph(&storage);
    let store = storage.memory();
    let before = generation(&storage);
    let watches = Writers::for_tests().watches;
    let refused = |result: Result<gyrus::Pending, MemoryError>| match result {
        Err(MemoryError::Refused(why)) => why,
        other => panic!("not refused: {other:?}"),
    };
    assert_eq!(
        refused(store.write_world(&watches, &Secrets, &graph, &sessions(1), NOW)),
        Refusal::WrongWriter
    );
    let mut undeclared = sessions(1);
    undeclared.link(NewLink::new("session:s0", "about", "session:s0", "test"));
    assert_eq!(
        refused(store.write_world(&world_writer(), &Secrets, &graph, &undeclared, NOW)),
        Refusal::World(gyrus::Refusal::Key)
    );
    let mut planet = Batch::new();
    planet.node(NewNode::new("planet", "planet:x", Label::none(), NOW));
    assert_eq!(
        refused(store.write_world(&world_writer(), &Secrets, &graph, &planet, NOW)),
        Refusal::World(gyrus::Refusal::UndeclaredKind)
    );
    let mut secret = Batch::new();
    secret.node(NewNode::new(
        "memory",
        "memory:x",
        Label::Trusted("my key is sk-qzxsecretcanary".into()),
        NOW,
    ));
    assert_eq!(
        refused(store.write_world(&world_writer(), &Secrets, &graph, &secret, NOW)),
        Refusal::Secret
    );
    // A link to a node that isn't there is refused in the transaction.
    let mut dangling = Batch::new();
    dangling.link(NewLink::new("memory:none", "about", "session:none", "test"));
    assert_eq!(
        refused(store.write_world(&world_writer(), &Secrets, &graph, &dangling, NOW)),
        Refusal::World(gyrus::Refusal::NoSuchNode)
    );
    assert_eq!(
        generation(&storage),
        before,
        "a refused batch wrote something"
    );
    let logged = count(
        &storage,
        "SELECT COUNT(*) FROM ai_log WHERE verb = 'refused' AND payload LIKE '%\"world\"%'",
    );
    assert_eq!(logged, 5);
    assert_eq!(
        count(
            &storage,
            "SELECT COUNT(*) FROM ai_log WHERE payload LIKE '%qzxsecretcanary%'"
        ),
        0
    );
    // An accepted batch: written, its counts logged, published.
    store
        .write_world(&world_writer(), &Secrets, &graph, &sessions(3), NOW)
        .unwrap();
    assert_eq!(graph.snapshot().unwrap().len(), 3);
    assert!(graph.check(&storage.conn).unwrap());
    assert_eq!(
        count(
            &storage,
            "SELECT COUNT(*) FROM ai_log WHERE verb = 'linked'"
        ),
        1
    );
}

/// A record and its world node commit together (ADR-0038 §1): a batch
/// the world gate refuses takes the record with it.
#[test]
fn a_record_and_its_world_node_commit_together() {
    let storage = installed_storage();
    let graph = graph(&storage);
    let store = storage.memory();
    store
        .write_world(&world_writer(), &Secrets, &graph, &sessions(1), NOW)
        .unwrap();
    let rec = record("note:web", Kind::Note, "web deploys need the staging flag");
    let link = |written: &Remembered| {
        let mut b = Batch::new();
        b.node(NewNode::new(
            "memory",
            &w::memory_key(&written.subject_id),
            Label::none(),
            NOW,
        ))
        .link(NewLink::new(
            &w::memory_key(&written.subject_id),
            "about",
            "session:s0",
            "test",
        ));
        b
    };
    let written = store
        .remember_linked(&all(), &Secrets, &rec, &graph, &link, NOW)
        .unwrap();
    let snap = graph.snapshot().unwrap();
    let node = snap.id_of(&w::memory_key(&written.subject_id)).unwrap();
    assert_eq!(snap.out(node, 0).len(), 1);
    assert!(graph.check(&storage.conn).unwrap());
    // Refused: a link to a session that isn't there; the record isn't kept.
    let other = record("note:docs", Kind::Note, "docs wait on review");
    let bad = |written: &Remembered| {
        let mut b = link(written);
        b.link(NewLink::new(
            &w::memory_key(&written.subject_id),
            "about",
            "session:gone",
            "test",
        ));
        b
    };
    let refused = store.remember_linked(&all(), &Secrets, &other, &graph, &bad, NOW);
    assert!(matches!(
        refused,
        Err(MemoryError::Refused(Refusal::World(
            gyrus::Refusal::NoSuchNode
        )))
    ));
    assert!(store.current("note:docs").unwrap().is_none());
}

/// A forget reaches the world model in the same transaction: the
/// subject's `memory` node and its links go, then the in-memory index.
#[test]
fn forget_reaches_the_world_model() {
    let storage = installed_storage();
    let graph = graph(&storage);
    let store = storage.memory();
    store
        .write_world(&world_writer(), &Secrets, &graph, &sessions(1), NOW)
        .unwrap();
    let rec = record("note:web", Kind::Note, "web deploys need the staging flag");
    let written = store
        .remember_linked(
            &all(),
            &Secrets,
            &rec,
            &graph,
            &|w: &Remembered| {
                let mut b = Batch::new();
                b.node(NewNode {
                    aliases: vec!["staging flag".into()],
                    ..NewNode::new("memory", &w::memory_key(&w.subject_id), Label::none(), NOW)
                })
                .link(NewLink::new(
                    &w::memory_key(&w.subject_id),
                    "about",
                    "session:s0",
                    "t",
                ));
                b
            },
            NOW,
        )
        .unwrap();
    let gone = store
        .forget_with_world("note:web", &Actor::User, NOW, Some(&graph))
        .unwrap();
    assert_eq!(gone.world_nodes, 1);
    let snap = graph.snapshot().unwrap();
    assert!(snap.id_of(&w::memory_key(&written.subject_id)).is_none());
    assert!(!snap.holds_text("staging"));
    assert!(graph.check(&storage.conn).unwrap(), "the index lags SQLite");
    assert_eq!(count(&storage, "SELECT COUNT(*) FROM gyrus_aliases"), 0);
    // Without the graph the index finds itself behind and rebuilds.
    let again = record("note:docs", Kind::Note, "docs wait on review");
    store
        .remember_linked(
            &all(),
            &Secrets,
            &again,
            &graph,
            &|w: &Remembered| {
                let mut b = Batch::new();
                b.node(NewNode::new(
                    "memory",
                    &w::memory_key(&w.subject_id),
                    Label::none(),
                    NOW,
                ));
                b
            },
            NOW,
        )
        .unwrap();
    store.forget("note:docs", &Actor::User, NOW).unwrap();
    assert!(!graph.check(&storage.conn).unwrap());
    assert!(graph.ensure(&storage.conn).unwrap());
    assert_eq!(graph.snapshot().unwrap().len(), 1);
}

/// Clear chat history takes the turns' and topics' world nodes with them;
/// AI Juggler's own copies keep no world model at all.
#[test]
fn clearing_and_own_copies_leave_no_world_nodes() {
    let storage = installed_storage();
    let graph = graph(&storage);
    let mut b = sessions(1);
    b.node(NewNode::new(
        "topic",
        &w::topic_key("c1"),
        Label::none(),
        NOW,
    ))
    .node(NewNode::new(
        "turn_summary",
        &w::turn_key("t1"),
        Label::none(),
        NOW,
    ))
    .link(NewLink::new(
        &w::turn_key("t1"),
        "part_of",
        &w::topic_key("c1"),
        "t",
    ));
    storage
        .memory()
        .write_world(&world_writer(), &Secrets, &graph, &b, NOW)
        .unwrap();
    storage.chat().clear().unwrap();
    assert_eq!(count(&storage, "SELECT COUNT(*) FROM gyrus_nodes"), 1);
    assert_eq!(count(&storage, "SELECT COUNT(*) FROM gyrus_links"), 0);
    scrub_snapshot(&storage.conn, Destination::OwnCopy).unwrap();
    assert_eq!(count(&storage, "SELECT COUNT(*) FROM gyrus_nodes"), 0);
}
