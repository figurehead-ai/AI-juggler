//! The idle pass: tiers, decay, archive, folds, duplicates, weak links,
//! in bounded batches.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    missing_docs,
    reason = "tests: a failed step fails the test"
)]

mod common;

use common::{db, link, node};
use gyrus::dream::{self, DreamOpts};
use gyrus::{
    Authority, Batch, Fold, Graph, KindSpec, LinkSpec, NewLink, NewNode, Pack, Tier, TierRules,
    WalkRule,
};

const DAY: i64 = 86_400_000;
const NOW: i64 = 400 * DAY;

fn pack() -> Pack {
    let tiers = TierRules {
        promote_short_after: 2,
        promote_mid_after: 5,
        demote_after_days: 30.0,
        archive_after_days: Some(60.0),
    };
    let keep = TierRules {
        archive_after_days: None,
        ..tiers
    };
    Pack::builder()
        .kind(
            KindSpec::new("note")
                .half_life(30.0)
                .tiers(tiers)
                .mergeable()
                .walk(WalkRule::out("about", 1, 4)),
        )
        .kind(KindSpec::new("session").tiers(keep).not_searchable())
        .kind(KindSpec::new("turn").tiers(tiers).not_searchable())
        .kind(KindSpec::new("topic").tiers(keep).not_searchable())
        .link(LinkSpec::new("about", &["note"], &["session"]))
        .link(LinkSpec::new("part_of", &["turn"], &["topic"]))
        .link(LinkSpec::new("same_as", &["note"], &["note"]))
        .same_as("same_as")
        .fold(Fold {
            child: "turn".into(),
            link: "part_of".into(),
            parent: "topic".into(),
            after_days: 14.0,
        })
        .build()
        .unwrap()
}

fn at(n: NewNode, days_ago: i64) -> NewNode {
    NewNode {
        as_of: NOW - days_ago * DAY,
        ..n
    }
}

fn dreamed(graph: &Graph, conn: &rusqlite::Connection) -> dream::DreamCounts {
    let snap = graph.snapshot().unwrap();
    let planned = dream::plan(conn, &snap, graph.pack(), &DreamOpts::new(NOW, 500)).unwrap();
    drop(snap);
    graph.write(conn, &planned.batch, NOW).unwrap();
    planned.counts
}

fn entry(graph: &Graph, key: &str) -> gyrus::NodeEntry {
    let snap = graph.snapshot().unwrap();
    snap.node(snap.id_of(key).unwrap()).unwrap().clone()
}

#[test]
fn use_promotes_idleness_decays_then_archives_but_never_what_the_user_stated() {
    let db = db();
    let graph = Graph::new(pack());
    graph.build(&db.conn).unwrap();
    let mut b = Batch::new();
    b.node(node("session", "session:web", ""))
        .node(at(node("note", "note:used", "used often"), 1))
        .node(at(
            NewNode {
                priority: 1,
                ..node("note", "note:idle", "nobody reads this")
            },
            40,
        ))
        .node(at(
            NewNode {
                priority: 0,
                ..node("note", "note:stale", "long forgotten")
            },
            90,
        ))
        .node(at(
            NewNode {
                priority: 0,
                tier: Tier::Long,
                authority: Authority::User,
                ..node("note", "note:stated", "the user said this")
            },
            300,
        ));
    graph.write(&db.conn, &b, NOW).unwrap();
    let used = graph.snapshot().unwrap().id_of("note:used").unwrap();
    graph.touch(&[used, used, used]);
    graph.flush_uses(&db.conn, NOW).unwrap();
    let counts = dreamed(&graph, &db.conn);
    assert_eq!(entry(&graph, "note:used").tier, Tier::Mid, "{counts:?}");
    assert_eq!(entry(&graph, "note:idle").priority, 0);
    assert!(entry(&graph, "note:stale").archived);
    let stated = entry(&graph, "note:stated");
    assert!(
        !stated.archived && stated.tier == Tier::Long,
        "the user's own was archived"
    );
    assert!(counts.promoted >= 1 && counts.decayed >= 1 && counts.archived == 1);
    // An archived node is out of walks; a use brings it back.
    let stale = graph.snapshot().unwrap().id_of("note:stale").unwrap();
    graph.touch(&[stale]);
    graph.flush_uses(&db.conn, NOW).unwrap();
    assert!(!entry(&graph, "note:stale").archived);
}

#[test]
fn turns_fold_into_their_topic_once_idle() {
    let db = db();
    let graph = Graph::new(pack());
    graph.build(&db.conn).unwrap();
    let mut b = Batch::new();
    b.node(node("topic", "topic:a", ""))
        .node(at(node("turn", "turn:old", ""), 20))
        .node(at(node("turn", "turn:new", ""), 1))
        .link(link("turn:old", "part_of", "topic:a"))
        .link(link("turn:new", "part_of", "topic:a"));
    graph.write(&db.conn, &b, NOW).unwrap();
    let counts = dreamed(&graph, &db.conn);
    assert_eq!(counts.folded, 1);
    assert!(entry(&graph, "turn:old").archived);
    assert!(!entry(&graph, "turn:new").archived);
    assert_eq!(entry(&graph, "topic:a").folded, 1);
}

#[test]
fn duplicates_are_proposed_then_merged_and_never_an_agents() {
    let db = db();
    let graph = Graph::new(pack());
    graph.build(&db.conn).unwrap();
    let mut b = Batch::new();
    b.node(node("session", "session:web", ""));
    for key in ["note:a", "note:b"] {
        b.node(at(
            node("note", key, "Web deploys need the staging flag"),
            1,
        ))
        .link(link(key, "about", "session:web"));
    }
    b.node(at(
        NewNode {
            authority: Authority::Agent,
            label: gyrus::Label::Untrusted("web deploys need the staging flag".into()),
            ..node("note", "note:agent", "")
        },
        1,
    ))
    .link(link("note:agent", "about", "session:web"));
    graph.write(&db.conn, &b, NOW).unwrap();
    let first = dreamed(&graph, &db.conn);
    assert_eq!(first.proposed, 1, "{first:?}");
    let snap = graph.snapshot().unwrap();
    let same = graph.pack().link_index("same_as").unwrap();
    let a = snap.id_of("note:a").unwrap();
    assert_eq!(snap.out(a, same).len(), 1, "the agent's note was proposed");
    drop(snap);
    let second = dreamed(&graph, &db.conn);
    assert_eq!(second.merged, 1, "{second:?}");
    let snap = graph.snapshot().unwrap();
    let left: Vec<bool> = ["note:a", "note:b"]
        .iter()
        .map(|k| snap.id_of(k).is_some())
        .collect();
    assert_eq!(
        left.iter().filter(|x| **x).count(),
        1,
        "one of the two is kept"
    );
    assert!(
        snap.id_of("note:agent").is_some(),
        "agent text is never merged"
    );
}

#[test]
fn weak_links_between_idle_nodes_are_pruned() {
    let db = db();
    let graph = Graph::new(pack());
    graph.build(&db.conn).unwrap();
    let mut b = Batch::new();
    b.node(at(node("session", "session:web", ""), 50))
        .node(at(node("note", "note:weak", "faint"), 50))
        .node(at(node("note", "note:strong", "clear"), 50))
        .link(NewLink {
            weight: 0.01,
            ..link("note:weak", "about", "session:web")
        })
        .link(link("note:strong", "about", "session:web"));
    graph.write(&db.conn, &b, NOW).unwrap();
    let counts = dreamed(&graph, &db.conn);
    assert_eq!(counts.pruned, 1);
    let snap = graph.snapshot().unwrap();
    let about = graph.pack().link_index("about").unwrap();
    assert_eq!(snap.out(snap.id_of("note:weak").unwrap(), about).len(), 0);
    assert_eq!(snap.out(snap.id_of("note:strong").unwrap(), about).len(), 1);
}

#[test]
fn a_pass_is_bounded_and_carries_on_from_its_cursor() {
    let db = db();
    let graph = Graph::new(pack());
    graph.build(&db.conn).unwrap();
    let mut b = Batch::new();
    for i in 0..50 {
        b.node(at(node("note", &format!("note:{i}"), "idle"), 40));
    }
    graph.write(&db.conn, &b, NOW).unwrap();
    let snap = graph.snapshot().unwrap();
    let first = dream::plan(&db.conn, &snap, graph.pack(), &DreamOpts::new(NOW, 10)).unwrap();
    assert_eq!(first.batch.ops.len(), 10);
    let next = first.next.unwrap();
    let rest = dream::plan(
        &db.conn,
        &snap,
        graph.pack(),
        &DreamOpts {
            after: next,
            ..DreamOpts::new(NOW, 1_000)
        },
    )
    .unwrap();
    assert!(rest.next.is_none());
    assert_eq!(first.batch.ops.len() + rest.batch.ops.len(), 50);
}
