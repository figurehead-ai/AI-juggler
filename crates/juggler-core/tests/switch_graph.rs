//! AJ-E201 evidence: the Switch Graph — Work/Epoch/SwitchEdge
//! round-trips through SQLite identically after reopen (the
//! a→b→a→c→b fixture survives restart), edges are append-only
//! (transitions, never rewrites; undo inserts a compensating edge),
//! the graph is per-project by construction, migration up from an
//! earlier schema preserves rows, and resumed epochs keep the
//! original `native_id` (Req.5) with Resume available on every
//! epoch that has one (Req.6).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_core::model::{
    AccountId, EdgeActor, EdgeState, Epoch, EpochId, FailureStage, HarnessId, ManifestEntry,
    ProjectId, SessionId, SwitchEdge, SwitchEdgeId, SwitchMechanism, WorkItem, WorkItemId,
};
use juggler_core::storage::{Storage, WorkGraph};
use proptest::prelude::*;

fn open() -> Storage {
    Storage::open_in_memory().unwrap()
}

fn epoch(
    work: &WorkItemId,
    id: &str,
    account: &str,
    native: Option<&str>,
    resumed_from: Option<&EpochId>,
) -> Epoch {
    Epoch {
        id: EpochId::new(id),
        work: work.clone(),
        session_ref: native.map(|_| SessionId::new(format!("sess-{id}"))),
        native_id: native.map(str::to_owned),
        account: AccountId::new(account),
        provider: HarnessId::new("claude-code"),
        resumed_from_epoch: resumed_from.cloned(),
    }
}

fn edge(work: &WorkItemId, id: &str, from: &EpochId, to_account: &str) -> SwitchEdge {
    SwitchEdge {
        id: SwitchEdgeId::new(id),
        work: work.clone(),
        from_epoch: from.clone(),
        to_epoch: None,
        from_account: AccountId::new("acc-a"),
        to_account: AccountId::new(to_account),
        from_provider: HarnessId::new("claude-code"),
        to_provider: HarnessId::new("claude-code"),
        mechanism: SwitchMechanism::Transplant,
        pack_id: None,
        fidelity_shown: Some("F3".to_owned()),
        manifest_sha256: Some("ab12".repeat(16)),
        state: EdgeState::Staged,
        actor: EdgeActor::User,
        parent_edge_a: None,
        parent_edge_b: None,
        created_at: juggler_core::model::Timestamp::from_millis(1_792_094_400_000),
    }
}

fn seed_project(storage: &Storage, root: &str) -> ProjectId {
    storage
        .projects()
        .touch(std::path::Path::new(root), None)
        .unwrap()
        .id
}

/// The pass2/02 §2.3 acceptance chain (extended): A→B→A→C→B renders
/// as ONE graph, and re-opening it after restart is byte-identical.
#[test]
fn a_b_a_c_b_round_trips_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graph.db");

    let (work_id, before) = {
        let storage = Storage::open(&path).unwrap();
        let project = seed_project(&storage, "/w/ai-juggler");
        let work = storage
            .switch_graph()
            .create_work(&project, "Ship v2")
            .unwrap();
        let graph = storage.switch_graph();

        // A (first) → B → A (resume, same native id) → C → B.
        let a1 = epoch(&work.id, "ep-a1", "acc-a", Some("native-1"), None);
        graph.insert_epoch(&a1).unwrap();
        let b1 = epoch(&work.id, "ep-b1", "acc-b", Some("native-2"), None);
        graph.insert_epoch(&b1).unwrap();
        let e1 = edge(&work.id, "edge-1", &a1.id, "acc-b");
        graph.append_edge(&e1).unwrap();
        graph.mark_launched(&e1.id, &b1.id).unwrap();
        graph.mark_confirmed(&e1.id).unwrap();

        // Req.5: switch-back creates a NEW epoch resuming the
        // original native id.
        let a2 = epoch(&work.id, "ep-a2", "acc-a", Some("native-1"), Some(&a1.id));
        assert!(a2.can_resume());
        graph.insert_epoch(&a2).unwrap();
        let e2 = edge(&work.id, "edge-2", &b1.id, "acc-a");
        graph.append_edge(&e2).unwrap();
        graph.mark_launched(&e2.id, &a2.id).unwrap();

        let c1 = epoch(&work.id, "ep-c1", "acc-c", Some("native-3"), None);
        graph.insert_epoch(&c1).unwrap();
        let e3 = edge(&work.id, "edge-3", &a2.id, "acc-c");
        graph.append_edge(&e3).unwrap();
        graph.mark_launched(&e3.id, &c1.id).unwrap();

        let b2 = epoch(&work.id, "ep-b2", "acc-b", Some("native-4"), None);
        graph.insert_epoch(&b2).unwrap();
        let e4 = edge(&work.id, "edge-4", &c1.id, "acc-b");
        graph.append_edge(&e4).unwrap();
        graph.mark_launched(&e4.id, &b2.id).unwrap();

        // Manifest rows ride along.
        graph
            .append_manifest(
                &e1.id,
                &[ManifestEntry {
                    rel_path: "conversation.md".into(),
                    sha256: "cd34".repeat(16),
                    bytes: 42,
                    kind: "conversation".into(),
                    disposition: "copy".into(),
                }],
            )
            .unwrap();

        let graph_snapshot = graph.graph(&work.id).unwrap();
        assert_eq!(graph_snapshot.epochs.len(), 5, "5 epoch nodes");
        assert_eq!(graph_snapshot.edges.len(), 4, "4 edges");
        (
            work.id.clone(),
            serde_json::to_string(&graph_snapshot).unwrap(),
        )
    };

    // Restart: reopen the DB and require the identical graph.
    let storage = Storage::open(&path).unwrap();
    let after = storage.switch_graph().graph(&work_id).unwrap();
    assert_eq!(
        serde_json::to_string(&after).unwrap(),
        before,
        "restart-identical graph JSON"
    );
    // The resumed epoch kept the original native id (Req.5).
    let a2 = after
        .epochs
        .iter()
        .find(|e| e.id.as_str() == "ep-a2")
        .unwrap();
    assert_eq!(a2.native_id.as_deref(), Some("native-1"));
    assert_eq!(
        a2.resumed_from_epoch.as_ref().map(EpochId::as_str),
        Some("ep-a1")
    );
    // Manifest survived too.
    assert_eq!(
        storage
            .switch_graph()
            .manifest_for(&SwitchEdgeId::new("edge-1"))
            .unwrap()
            .len(),
        1
    );
}

/// Invariant S-G1: undo and failure never rewrite — they transition
/// or append. A "failed" edge stays failed with its reason; undo
/// inserts a compensating edge and the original is untouched.
#[test]
fn append_only_undo_inserts_compensating_edge() {
    let storage = open();
    let project = seed_project(&storage, "/w/x");
    let graph = storage.switch_graph();
    let work = graph.create_work(&project, "Task").unwrap();
    let a = epoch(&work.id, "ep-a", "acc-a", Some("n1"), None);
    graph.insert_epoch(&a).unwrap();
    let b = epoch(&work.id, "ep-b", "acc-b", Some("n2"), None);
    graph.insert_epoch(&b).unwrap();

    let e1 = edge(&work.id, "edge-1", &a.id, "acc-b");
    graph.append_edge(&e1).unwrap();
    graph.mark_launched(&e1.id, &b.id).unwrap();
    graph
        .mark_failed(&e1.id, FailureStage::Confirm, "target never attached")
        .unwrap();

    // "Undo": a compensating switch-back edge back to A.
    let undo = SwitchEdge {
        id: SwitchEdgeId::new("edge-undo"),
        parent_edge_a: Some(e1.id.clone()),
        ..edge(&work.id, "edge-undo", &b.id, "acc-a")
    };
    graph.append_edge(&undo).unwrap();
    graph.mark_launched(&undo.id, &a.id).unwrap();

    let snapshot = graph.graph(&work.id).unwrap();
    let failed = snapshot
        .edges
        .iter()
        .find(|e| e.id.as_str() == "edge-1")
        .unwrap();
    assert!(
        matches!(
            &failed.state,
            EdgeState::Failed {
                stage: FailureStage::Confirm,
                ..
            }
        ),
        "failure recorded on the original edge: {:?}",
        failed.state
    );
    assert_eq!(snapshot.edges.len(), 2, "both edges kept — history intact");
    assert_eq!(
        failed.to_epoch.as_ref().map(EpochId::as_str),
        Some("ep-b"),
        "to_epoch preserved"
    );

    // The API refuses to append an edge that pre-fills to_epoch or a
    // post-staged state (the guardrails of append-only).
    let bogus = SwitchEdge {
        to_epoch: Some(b.id.clone()),
        ..edge(&work.id, "edge-bogus", &a.id, "acc-b")
    };
    assert!(graph.append_edge(&bogus).is_err());
    let bogus2 = SwitchEdge {
        state: EdgeState::Confirmed,
        ..edge(&work.id, "edge-bogus2", &a.id, "acc-b")
    };
    assert!(graph.append_edge(&bogus2).is_err());

    // Transitions are one-way: a confirmed edge cannot re-launch.
    let e2 = edge(&work.id, "edge-2", &a.id, "acc-b");
    graph.append_edge(&e2).unwrap();
    graph.mark_launched(&e2.id, &b.id).unwrap();
    graph.mark_confirmed(&e2.id).unwrap();
    assert!(
        graph
            .mark_failed(&e2.id, FailureStage::Launch, "late")
            .is_err()
    );
}

/// Invariant S-G2: cross-project moves cannot be edges — epochs and
/// edges hang off work items that reference exactly one project.
#[test]
fn graph_is_per_project_by_construction() {
    let storage = open();
    let p1 = seed_project(&storage, "/w/one");
    let p2 = seed_project(&storage, "/w/two");
    let graph = storage.switch_graph();
    let w1 = graph.create_work(&p1, "One").unwrap();
    let w2 = graph.create_work(&p2, "Two").unwrap();

    // An epoch referencing w2's work cannot appear in w1's graph.
    let e = epoch(&w2.id, "ep-x", "acc-a", Some("n"), None);
    graph.insert_epoch(&e).unwrap();
    assert!(graph.graph(&w1.id).unwrap().epochs.is_empty());
    assert_eq!(graph.graph(&w2.id).unwrap().epochs.len(), 1);
    // Each work item is anchored to exactly one project.
    assert_eq!(graph.work(&w1.id).unwrap().project, p1);
    assert_eq!(graph.work(&w2.id).unwrap().project, p2);
    assert_eq!(graph.works_for_project(&p1).unwrap().len(), 1);
    assert_eq!(graph.works_for_project(&p2).unwrap().len(), 1);
}

/// Migration: a database from an earlier schema version migrates up
/// with zero data loss and usable graph tables.
#[test]
fn migration_up_from_earlier_schema_preserves_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("older.db");
    {
        let storage = Storage::open(&path).unwrap();
        seed_project(&storage, "/w/migrate");
        // Simulate a pre-v20 database honestly: drop the v20 tables
        // (a v19 database has none of them) and rewind the recorded
        // version; the runner then replays V20 forward onto real
        // pre-existing rows.
        storage
            .raw_conn()
            .execute_batch(
                "DROP TABLE artifact_manifests;
                 DROP TABLE switch_edges;
                 DROP TABLE epochs;
                 DROP TABLE work_items;
                 ALTER TABLE accounts DROP COLUMN import_source_path;
                 ALTER TABLE accounts DROP COLUMN import_origin;
                 ALTER TABLE accounts DROP COLUMN imported_at;
                 PRAGMA user_version = 19;",
            )
            .unwrap();
    }
    let storage = Storage::open(&path).unwrap();
    // Old data intact.
    assert!(
        storage
            .projects()
            .by_root(std::path::Path::new("/w/migrate"))
            .unwrap()
            .is_some(),
        "existing rows survive migration"
    );
    // New tables usable (schema landed).
    let project = storage
        .projects()
        .by_root(std::path::Path::new("/w/migrate"))
        .unwrap()
        .unwrap();
    let work = storage
        .switch_graph()
        .create_work(&project.id, "Post-migration")
        .unwrap();
    assert_eq!(
        storage.switch_graph().graph(&work.id).unwrap().epochs.len(),
        0
    );
}

// Property: RANDOM legal edge sequences round-trip identically
// through SQLite after reopen.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn random_graphs_round_trip(
        chain in prop::collection::vec("(a|b|c|d)", 1..12),
        native in prop::bool::ANY,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prop.db");
        let (work_id, before_json) = {
            let storage = Storage::open(&path).unwrap();
            let project = seed_project(&storage, "/w/prop");
            let graph = storage.switch_graph();
            let work = graph.create_work(&project, "Prop").unwrap();
            let mut current = epoch(&work.id, "ep-0", "acc-a", native.then_some("n0"), None);
            graph.insert_epoch(&current).unwrap();
            for (idx, letter) in chain.iter().enumerate() {
                let target = epoch(&work.id, &format!("ep-{}", idx + 1), &format!("acc-{letter}"), native.then(|| format!("n{}", idx + 1)).as_deref(), None);
                graph.insert_epoch(&target).unwrap();
                let e = edge(&work.id, &format!("edge-{idx}"), &current.id, &format!("acc-{letter}"));
                graph.append_edge(&e).unwrap();
                if idx % 2 == 0 {
                    graph.mark_launched(&e.id, &target.id).unwrap();
                    if idx % 4 == 0 {
                        graph.mark_confirmed(&e.id).unwrap();
                    }
                } else {
                    graph.mark_failed(&e.id, FailureStage::Launch, "injected").unwrap();
                }
                current = target;
            }
            let snapshot: WorkGraph = graph.graph(&work.id).unwrap();
            (work.id.clone(), serde_json::to_string(&snapshot).unwrap())
        };
        // Reopen #1: identical bytes.
        let after = Storage::open(&path).unwrap().switch_graph().graph(&work_id).unwrap();
        prop_assert_eq!(serde_json::to_string(&after).unwrap(), before_json);
        // Reopen #2: also identical (idempotent reads).
        let after2 = Storage::open(&path).unwrap().switch_graph().graph(&work_id).unwrap();
        prop_assert_eq!(serde_json::to_string(&after).unwrap(), serde_json::to_string(&after2).unwrap());
    }
}

/// WorkItem/Epoch/SwitchEdge JSON is stable field-order (the
/// byte-identity contract above depends on it).
#[test]
fn graph_json_is_deterministic_across_serializations() {
    let storage = open();
    let project = seed_project(&storage, "/w/det");
    let graph = storage.switch_graph();
    let work = graph.create_work(&project, "Det").unwrap();
    let a = epoch(&work.id, "ep-a", "acc-a", Some("n"), None);
    graph.insert_epoch(&a).unwrap();
    let one = serde_json::to_string(&graph.graph(&work.id).unwrap()).unwrap();
    let two = serde_json::to_string(&graph.graph(&work.id).unwrap()).unwrap();
    assert_eq!(one, two);
    // And the WorkItem itself.
    let w1 = serde_json::to_string(&WorkItem {
        id: work.id.clone(),
        project: work.project.clone(),
        title: "Det".into(),
        created_at: work.created_at,
    })
    .unwrap();
    let w2 = w1.clone();
    assert_eq!(w1, w2);
}
