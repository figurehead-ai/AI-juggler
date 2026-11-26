#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test: a failed step fails the test"
)]

use std::sync::Arc;

use gyrus::ops::{self, SeedQuery, Via, Walk};
use juggler_core::storage::memory::world::memory_key;

use super::{link, pack, resolve};
use crate::features::bot::dto::{BotRowInfo, BotRowKind, BotState};
use crate::features::bot::world::World;
use crate::features::bot::world::tests_support::row;
use crate::state::AppState;

/// 2026-11-19 12:00 UTC.
const NOW: i64 = 1_795_521_600_000;
const MIN: i64 = 60_000;

fn state() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::bare_state(dir.path().to_path_buf());
    (dir, state)
}

fn session(id: &str, title: &str, state: BotState, project: &str) -> BotRowInfo {
    BotRowInfo {
        id: id.into(),
        project: Some(project.into()),
        project_id: Some(format!("p-{project}")),
        app_id: Some("app-term".into()),
        app_label: Some("Terminal".into()),
        at_ms: Some(NOW - 5 * MIN),
        ..row(BotRowKind::Session, title, Some(state))
    }
}

/// Generic names only.
pub(super) fn world() -> World {
    World {
        now_ms: NOW,
        offset_hours: 0,
        sessions: vec![
            session("s-web", "web", BotState::AsksYou, "shop"),
            session("s-api", "agent11", BotState::Working, "shop"),
            session("s-docs", "docs", BotState::Finished, "docs-site"),
        ],
        tasks: vec![BotRowInfo {
            id: "t-1".into(),
            pool: Some("reviewers".into()),
            pool_id: Some("pool-rev".into()),
            project: Some("shop".into()),
            project_id: Some("p-shop".into()),
            ..row(
                BotRowKind::Task,
                "Review the cart",
                Some(BotState::InReview),
            )
        }],
        accounts: vec![BotRowInfo {
            id: "acc-1".into(),
            used_percent: Some(62),
            window: Some("five_hour".into()),
            ..row(BotRowKind::Account, "Work", None)
        }],
        removed: Vec::new(),
    }
}

fn sync(state: &Arc<AppState>, world: &World) {
    let storage = state.storage.lock().unwrap();
    let graph = state.world.graph().unwrap();
    let snap = state.world.ready(state, &storage);
    let batch = link::entities_batch(world, snap.as_deref(), graph.pack(), NOW);
    state.world.write(state, &storage, &batch, NOW).unwrap();
}

#[test]
fn the_pack_declares_adr_0038s_kinds_and_links() {
    let pack = pack::pack().unwrap();
    for kind in [
        "user",
        "preference",
        "behaviour",
        "project",
        "task",
        "app",
        "profile",
        "account",
        "session",
        "pool",
        "memory",
        "fact",
        "rule",
        "question",
        "topic",
        "conversation",
        "turn_summary",
    ] {
        assert!(pack.kind_index(kind).is_some(), "{kind} undeclared");
    }
    for link in [
        "about",
        "part_of",
        "runs_in",
        "uses_account",
        "prefers",
        "asked_about",
        "derived_from",
        "contradicts",
        "supersedes",
        "governs",
        "same_as",
    ] {
        assert!(pack.link_index(link).is_some(), "{link} undeclared");
    }
    // Pointer kinds keep no words in the keyword index.
    for kind in pack::ENTITY_KINDS
        .iter()
        .chain(&["memory", "turn_summary", "topic"])
    {
        let spec = pack.kind(pack.kind_index(kind).unwrap()).unwrap();
        assert!(!spec.searchable, "{kind} is searchable");
    }
    // Each kind that brings live state into a context says to read it live.
    let session = pack.kind(pack.kind_index("session").unwrap()).unwrap();
    assert!(session.questions[0].contains("live"));
}

/// Entity nodes are pointers: a key and links, never a name or a state;
/// the sync writes only what the index lacks.
#[test]
fn entities_are_pointers_linked_as_the_records_relate_them() {
    let (_dir, state) = state();
    let world = world();
    sync(&state, &world);
    let storage = state.storage.lock().unwrap();
    let snap = state.world.ready(&state, &storage).unwrap();
    let graph = state.world.graph().unwrap();
    for key in [
        "session:s-web",
        "project:p-shop",
        "app:app-term",
        "task:t-1",
        "pool:pool-rev",
        "account:acc-1",
    ] {
        let id = snap.id_of(key).unwrap_or_else(|| panic!("{key} missing"));
        let labels = storage.world().labels(&[id]).unwrap();
        assert_eq!(labels[&id], "", "{key} holds words");
    }
    let raw = storage.raw_conn();
    let props: i64 = raw
        .query_row(
            "SELECT COUNT(*) FROM gyrus_nodes WHERE props != '{}' OR label != ''",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(props, 0, "an entity node keeps state");
    let part_of = graph.pack().link_index("part_of").unwrap();
    let web = snap.id_of("session:s-web").unwrap();
    assert_eq!(
        snap.out(web, part_of)[0].other,
        snap.id_of("project:p-shop").unwrap()
    );
    drop(storage);
    // A second pass over the same world writes nothing.
    let again = {
        let storage = state.storage.lock().unwrap();
        let snap = state.world.ready(&state, &storage);
        link::entities_batch(&world, snap.as_deref(), graph.pack(), NOW)
    };
    assert!(again.is_empty(), "{:?}", again.ops);
}

/// ADR-0037's records are linked as `memory` nodes by the one-time pass;
/// nothing moves; new ones are linked as they are written.
#[test]
fn records_are_linked_as_memory_nodes_and_nothing_moves() {
    let (_dir, state) = state();
    let world = world();
    // Written before the world model existed: plain remember.
    {
        let storage = state.storage.lock().unwrap();
        let writer = juggler_core::storage::memory::Writers::for_tests().memory;
        crate::features::bot::memory::remember_in(
            &storage,
            &writer,
            &world,
            "call agent11 the api agent",
            "t1",
        )
        .unwrap()
        .unwrap();
        crate::features::bot::memory::remember_in(
            &storage,
            &writer,
            &world,
            "the release goes out on fridays",
            "t2",
        )
        .unwrap()
        .unwrap();
    }
    let before: i64 = {
        let storage = state.storage.lock().unwrap();
        storage
            .raw_conn()
            .query_row("SELECT COUNT(*) FROM memory_records", [], |r| r.get(0))
            .unwrap()
    };
    let passed = super::link_records(&state);
    assert!(passed, "the one-time pass didn't finish");
    let storage = state.storage.lock().unwrap();
    let after: i64 = storage
        .raw_conn()
        .query_row("SELECT COUNT(*) FROM memory_records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after, before, "a record moved");
    let snap = state.world.ready(&state, &storage).unwrap();
    let graph = state.world.graph().unwrap();
    let alias = storage
        .memory()
        .current("alias:session:s-api")
        .unwrap()
        .unwrap();
    let node = snap.id_of(&memory_key(&alias.subject_id)).unwrap();
    assert_eq!(snap.by_alias("The API agent"), &[node]);
    let about = graph.pack().link_index("about").unwrap();
    assert_eq!(
        snap.out(node, about)[0].other,
        snap.id_of("session:s-api").unwrap()
    );
    // Seeding by the user's own name finds the session through its memory.
    let seeds = storage
        .world()
        .seed(
            &snap,
            graph.pack(),
            &SeedQuery {
                text: "tell the api agent to double-validate",
                k: 4,
                ..SeedQuery::default()
            },
        )
        .unwrap();
    assert_eq!((seeds[0].node, seeds[0].via), (node, Via::Alias));
    let reached = ops::expand(&snap, graph.pack(), &seeds, &Walk::new(16, NOW));
    let keys: Vec<String> = reached
        .iter()
        .map(|r| snap.node(r.node).unwrap().key.to_string())
        .collect();
    assert!(keys.contains(&"session:s-api".to_owned()), "{keys:?}");
}

/// A record written now commits with its node (Bot's remember flow).
#[test]
fn bots_remember_flow_links_the_record_in_the_same_write() {
    let (_dir, state) = state();
    let world = world();
    crate::features::bot::memory::remember(&state, &world, "call web the shop agent", "t1")
        .unwrap()
        .unwrap();
    let storage = state.storage.lock().unwrap();
    let snap = state.world.ready(&state, &storage).unwrap();
    let alias = storage
        .memory()
        .current("alias:session:s-web")
        .unwrap()
        .unwrap();
    assert!(snap.id_of(&memory_key(&alias.subject_id)).is_some());
    assert!(snap.id_of("session:s-web").is_some());
}

/// The live resolvers read state now; a memory is said as the user's.
#[test]
fn nodes_resolve_to_live_lines() {
    let (_dir, state) = state();
    let mut world = world();
    crate::features::bot::memory::remember(&state, &world, "call web the shop agent", "t1")
        .unwrap()
        .unwrap();
    let storage = state.storage.lock().unwrap();
    let line = resolve::resolve(&storage, &world, "session:s-web", None).unwrap();
    assert!(line.line.contains("asks you"), "{}", line.line);
    // The state changes: the line changes; nothing was remembered of it.
    world.sessions[0].state = Some(BotState::Finished);
    let line = resolve::resolve(&storage, &world, "session:s-web", None).unwrap();
    assert!(line.line.contains("finished now"), "{}", line.line);
    assert!(resolve::resolve(&storage, &world, "session:gone", None).is_none());
    let account = resolve::resolve(&storage, &world, "account:acc-1", None).unwrap();
    assert!(account.line.contains("62% used"), "{}", account.line);
    let alias = storage
        .memory()
        .current("alias:session:s-web")
        .unwrap()
        .unwrap();
    let memory = resolve::resolve(&storage, &world, &memory_key(&alias.subject_id), None).unwrap();
    assert!(
        memory
            .line
            .starts_with("you call \u{201c}web\u{201d} \u{201c}the shop agent\u{201d}"),
        "{}",
        memory.line
    );
    assert_eq!(resolve::age_words(NOW, NOW - 3 * 86_400_000), "3 days ago");
}
