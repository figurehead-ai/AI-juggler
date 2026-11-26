//! The operations: seed, expand (best K per step by the kinds' rules),
//! rank (one per fact key), as of, matches, explain.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    missing_docs,
    reason = "tests: a failed step fails the test"
)]

mod common;

use common::{db, link, node, small_graph};
use gyrus::ops::{self, Need, Pattern, SeedQuery, Var, Via, Walk};
use gyrus::{Batch, NewLink, NewNode, Op};

const NOW: i64 = 2_000;

fn keys(snap: &gyrus::Snapshot, ids: impl IntoIterator<Item = gyrus::NodeId>) -> Vec<String> {
    ids.into_iter()
        .map(|id| snap.node(id).unwrap().key.to_string())
        .collect()
}

#[test]
fn seeds_come_from_the_hosts_matches_aliases_then_words() {
    let db = db();
    let graph = small_graph(&db);
    let snap = graph.snapshot().unwrap();
    let given = vec![("project:api".to_owned(), 0.8)];
    let seeds = ops::seed(
        &db.conn,
        &snap,
        graph.pack(),
        &SeedQuery {
            text: "tell the api agent about staging",
            given: &given,
            k: 5,
            ..SeedQuery::default()
        },
    )
    .unwrap();
    let found: Vec<(String, Via)> = seeds
        .iter()
        .map(|s| (snap.node(s.node).unwrap().key.to_string(), s.via))
        .collect();
    assert_eq!(found[0], ("project:api".into(), Via::Given));
    assert_eq!(found[1], ("session:web".into(), Via::Alias), "{found:?}");
    assert!(
        found.contains(&("note:1".into(), Via::Keyword)),
        "{found:?}"
    );
    // Words are never query syntax.
    let odd = ops::seed(
        &db.conn,
        &snap,
        graph.pack(),
        &SeedQuery {
            text: "\" OR * NEAR( )",
            k: 5,
            ..SeedQuery::default()
        },
    )
    .unwrap();
    assert!(
        odd.iter()
            .all(|s| s.via != Via::Keyword || snap.node(s.node).is_some())
    );
}

#[test]
fn vectors_are_found_by_exact_cosine() {
    let db = db();
    let graph = small_graph(&db);
    let mut b = Batch::new();
    b.node(NewNode {
        vector: Some(vec![1.0, 0.0, 0.0]),
        ..node("note", "note:v1", "first")
    })
    .node(NewNode {
        vector: Some(vec![0.0, 1.0, 0.0]),
        ..node("note", "note:v2", "second")
    });
    graph.write(&db.conn, &b, NOW).unwrap();
    let near = ops::nearest(&db.conn, &[0.1, 0.9, 0.0], 2).unwrap();
    let snap = graph.snapshot().unwrap();
    assert_eq!(
        keys(&snap, near.iter().map(|(id, _)| *id)),
        vec!["note:v2", "note:v1"]
    );
    assert!((ops::cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
}

#[test]
fn a_walk_follows_the_kinds_rules_and_keeps_the_best_k_per_step() {
    let db = db();
    let graph = small_graph(&db);
    // Ten notes about web, of different strengths: a session keeps the
    // best five `about` links inwards (its rule), never an arbitrary five.
    let mut b = Batch::new();
    for i in 0..10 {
        let key = format!("note:w{i}");
        b.node(node("note", &key, "web note")).link(NewLink {
            weight: 0.1 + 0.08 * f32::from(u8::try_from(i).unwrap()),
            ..link(&key, "about", "session:web")
        });
    }
    graph.write(&db.conn, &b, NOW).unwrap();
    let snap = graph.snapshot().unwrap();
    let web = snap.id_of("session:web").unwrap();
    let seeds = [ops::Seed {
        node: web,
        relevance: 1.0,
        via: Via::Given,
    }];
    let reached = ops::expand(&snap, graph.pack(), &seeds, &Walk::new(64, NOW));
    let notes: Vec<String> = reached
        .iter()
        .filter(|r| {
            r.path.len() == 1 && r.path[0].link == graph.pack().link_index("about").unwrap()
        })
        .map(|r| snap.node(r.node).unwrap().key.to_string())
        .collect();
    assert_eq!(notes.len(), 5, "{notes:?}");
    for strong in ["note:w9", "note:w8", "note:w7", "note:1"] {
        assert!(
            notes.contains(&strong.to_owned()),
            "{strong} missing from {notes:?}"
        );
    }
    assert!(!notes.contains(&"note:w0".to_owned()));
    // The project two steps away, through part_of, and the app through runs_in.
    let all = keys(&snap, reached.iter().map(|r| r.node));
    assert!(all.contains(&"project:api".to_owned()));
    assert!(all.contains(&"app:term".to_owned()));
    // Scores fall along a path.
    let score = |key: &str| {
        reached
            .iter()
            .find(|r| snap.node(r.node).unwrap().key.as_ref() == key)
            .unwrap()
            .score
    };
    assert!(score("session:web") > score("project:api"));
    // The caps hold.
    let capped = ops::expand(&snap, graph.pack(), &seeds, &Walk::new(3, NOW));
    assert_eq!(capped.len(), 3);
    let shallow = ops::expand(
        &snap,
        graph.pack(),
        &seeds,
        &Walk {
            depth: 1,
            ..Walk::new(64, NOW)
        },
    );
    assert!(shallow.iter().all(|r| r.path.len() <= 1));
}

#[test]
fn a_rule_node_narrows_the_walk_of_what_it_governs() {
    let db = db();
    let graph = small_graph(&db);
    let mut b = Batch::new();
    b.node(NewNode {
        props: serde_json::json!({ "block": ["runs_in"] }),
        ..node("rule", "rule:no-apps", "never walk into apps from web")
    })
    .link(link("rule:no-apps", "governs", "session:web"));
    graph.write(&db.conn, &b, NOW).unwrap();
    let snap = graph.snapshot().unwrap();
    let seeds = [ops::Seed {
        node: snap.id_of("session:web").unwrap(),
        relevance: 1.0,
        via: Via::Given,
    }];
    let reached = keys(
        &snap,
        ops::expand(&snap, graph.pack(), &seeds, &Walk::new(64, NOW))
            .iter()
            .map(|r| r.node),
    );
    assert!(!reached.contains(&"app:term".to_owned()), "{reached:?}");
    assert!(reached.contains(&"project:api".to_owned()));
}

#[test]
fn as_of_reads_only_what_was_valid_then() {
    let db = db();
    let graph = small_graph(&db);
    let mut b = Batch::new();
    b.node(NewNode {
        valid_from: Some(5_000),
        ..node("note", "note:later", "said later")
    })
    .link(NewLink {
        valid_to: Some(3_000),
        ..link("note:later", "about", "session:docs")
    })
    .op(Op::Close {
        key: "note:2".into(),
        at: 4_000,
    });
    graph.write(&db.conn, &b, NOW).unwrap();
    let snap = graph.snapshot().unwrap();
    let docs = [ops::Seed {
        node: snap.id_of("session:docs").unwrap(),
        relevance: 1.0,
        via: Via::Given,
    }];
    let at = |t: i64| {
        keys(
            &snap,
            ops::expand(&snap, graph.pack(), &docs, &Walk::new(64, NOW).at(t))
                .iter()
                .map(|r| r.node),
        )
    };
    assert!(at(3_500).contains(&"note:2".to_owned()));
    assert!(!at(4_500).contains(&"note:2".to_owned()));
    assert!(
        !at(6_000).contains(&"note:later".to_owned()),
        "its link ended at 3,000"
    );
}

#[test]
fn ranking_keeps_one_node_per_fact_key() {
    let db = db();
    let graph = small_graph(&db);
    let mut b = Batch::new();
    for (i, w) in [(1, 0.9_f32), (2, 0.5)] {
        let key = format!("note:f{i}");
        b.node(NewNode {
            props: serde_json::json!({ "fact": "session:web:state" }),
            ..node("note", &key, "web is stuck")
        })
        .link(NewLink {
            weight: w,
            ..link(&key, "about", "session:web")
        });
    }
    graph.write(&db.conn, &b, NOW).unwrap();
    let snap = graph.snapshot().unwrap();
    let seeds = [ops::Seed {
        node: snap.id_of("session:web").unwrap(),
        relevance: 1.0,
        via: Via::Given,
    }];
    let reached = ops::expand(&snap, graph.pack(), &seeds, &Walk::new(64, NOW));
    let top = keys(&snap, ops::rank(&snap, &reached, 20).iter().map(|r| r.node));
    assert!(top.contains(&"note:f1".to_owned()));
    assert!(
        !top.contains(&"note:f2".to_owned()),
        "two nodes for one fact: {top:?}"
    );
    assert_eq!(ops::rank(&snap, &reached, 2).len(), 2);
}

/// ADR-0038 §3's example: sessions of the same project that share a
/// memory, across Apps.
#[test]
fn a_pattern_finds_sessions_of_one_project_sharing_a_note_across_apps() {
    let db = db();
    let graph = small_graph(&db);
    let mut b = Batch::new();
    b.node(node("note", "note:shared", "both wait on the schema"))
        .link(link("note:shared", "about", "session:web"))
        .link(link("note:shared", "about", "session:docs"));
    graph.write(&db.conn, &b, NOW).unwrap();
    let snap = graph.snapshot().unwrap();
    let pattern = Pattern {
        vars: vec![
            Var {
                kind: Some("session".into()),
            },
            Var {
                kind: Some("session".into()),
            },
            Var {
                kind: Some("project".into()),
            },
            Var {
                kind: Some("note".into()),
            },
            Var {
                kind: Some("app".into()),
            },
            Var {
                kind: Some("app".into()),
            },
        ],
        needs: vec![
            Need {
                from: 0,
                link: "part_of".into(),
                to: 2,
            },
            Need {
                from: 1,
                link: "part_of".into(),
                to: 2,
            },
            Need {
                from: 3,
                link: "about".into(),
                to: 0,
            },
            Need {
                from: 3,
                link: "about".into(),
                to: 1,
            },
            Need {
                from: 0,
                link: "runs_in".into(),
                to: 4,
            },
            Need {
                from: 1,
                link: "runs_in".into(),
                to: 5,
            },
        ],
        distinct: vec![(0, 1), (4, 5)],
        anchor: None,
        limit: 10,
        at: None,
    };
    let found = ops::matches(&snap, graph.pack(), &pattern);
    assert_eq!(found.len(), 2, "web/docs both ways round");
    for binding in &found {
        assert_eq!(snap.node(binding[3]).unwrap().key.as_ref(), "note:shared");
    }
    let one = ops::matches(
        &snap,
        graph.pack(),
        &Pattern {
            limit: 1,
            ..pattern.clone()
        },
    );
    assert_eq!(one.len(), 1);
}

#[test]
fn explain_says_why_each_node_was_chosen() {
    let db = db();
    let graph = small_graph(&db);
    let snap = graph.snapshot().unwrap();
    let seeds = [ops::Seed {
        node: snap.id_of("session:web").unwrap(),
        relevance: 1.0,
        via: Via::Alias,
    }];
    let reached = ops::expand(&snap, graph.pack(), &seeds, &Walk::new(64, NOW));
    let why = ops::explain(&snap, graph.pack(), &ops::rank(&snap, &reached, 10));
    let project = why.iter().find(|w| w.key == "project:api").unwrap();
    assert_eq!(project.seed, "session:web");
    assert_eq!(project.via, Via::Alias);
    assert_eq!(project.path, vec!["session:web -part_of-> project:api"]);
    assert_eq!(project.rule.as_deref(), Some("part_of out"));
    let web = why.iter().find(|w| w.key == "session:web").unwrap();
    assert!(web.path.is_empty() && web.rule.is_none());
    assert_eq!(
        web.questions,
        vec!["Is it running right now? Read its live state."]
    );
}
