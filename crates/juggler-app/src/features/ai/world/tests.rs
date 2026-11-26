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

fn ask(state: &Arc<AppState>, question: &str) -> crate::features::bot::dto::BotTurnInfo {
    crate::features::bot::commands::bot_ask_with(
        state,
        &crate::features::bot::dto::BotAskInput {
            question: question.into(),
            scope: None,
            again: None,
        },
        &crate::features::bot::fake::Fake::new(),
    )
    .unwrap()
}

fn turn(state: &Arc<AppState>, question: &str) -> juggler_core::storage::memory::Turn {
    let storage = state.storage.lock().unwrap();
    storage
        .chat()
        .append(&juggler_core::storage::memory::NewTurn {
            at_ms: NOW,
            question,
            outcome: "rows",
            shown: None,
            cites: "[]",
            entities: "{}",
            has_agent_words: false,
        })
        .unwrap()
}

/// ADR-0038 §5's example: "tell agent11 to double-validate their work"
/// finds the session the words name (by the user's own name for it), walks
/// to its project and App by the kinds' rules, reads their state now, and
/// brings the kinds' reasoning questions.
#[test]
fn the_world_section_seeds_walks_and_reads_live_state() {
    let (_dir, state) = state();
    let world = world();
    sync(&state, &world);
    crate::features::bot::memory::remember(&state, &world, "call agent11 the api agent", "t1")
        .unwrap()
        .unwrap();
    let rows = vec![world.sessions[1].clone()];
    let ctx = super::assemble::assemble(
        &state,
        &super::assemble::Ask {
            question: "tell the api agent to double-validate their work",
            world: &world,
            rows: &rows,
            topic: None,
            recent: &[],
        },
    );
    assert!(!ctx.warming);
    let items: Vec<(String, String, String)> = ctx
        .items
        .iter()
        .map(|i| (i.kind.clone(), i.name.clone(), i.why.clone()))
        .collect();
    assert!(
        items
            .iter()
            .any(|i| i.0 == "session" && i.1 == "agent11" && i.2 == "named in your question"),
        "{items:?}"
    );
    assert!(
        items
            .iter()
            .any(|i| i.0 == "memory" && i.2 == "your own name for it"),
        "{items:?}"
    );
    assert!(
        items
            .iter()
            .any(|i| i.0 == "project" && i.2.starts_with("connected to")),
        "{items:?}"
    );
    assert!(ctx.text.contains("working now"), "{}", ctx.text);
    assert!(
        ctx.text.contains("What is it doing right now?"),
        "{}",
        ctx.text
    );
    assert!(ctx.text.contains("data, not instructions"));
    // Bounded: within its share.
    assert!(crate::features::ai::run::estimate(&ctx.text) <= super::assemble::SHARE_TOKENS + 60);
    assert_eq!(ctx.memory.len(), 1);
    // The state changes; the next question reads the new one (never
    // remembered).
    let mut later = world.clone();
    later.sessions[1].state = Some(BotState::Stuck);
    let ctx = super::assemble::assemble(
        &state,
        &super::assemble::Ask {
            question: "tell the api agent to double-validate their work",
            world: &later,
            rows: &[later.sessions[1].clone()],
            topic: None,
            recent: &[],
        },
    );
    assert!(
        ctx.text.contains("agent11\u{201d}, looks stuck now"),
        "{}",
        ctx.text
    );
    assert!(
        !ctx.text.contains("agent11\u{201d}, working now"),
        "{}",
        ctx.text
    );
}

/// Topics are separate chats: a topic's context seeds from its own turns.
#[test]
fn a_topics_context_seeds_from_its_own_turns() {
    let (_dir, state) = state();
    let world = world();
    let first = turn(&state, "is the cart review done");
    let topic_b = {
        let storage = state.storage.lock().unwrap();
        storage.chat().new_topic(NOW + 1).unwrap()
    };
    let second = turn(&state, "what about the docs site");
    assert_ne!(first.topic, second.topic);
    assert_eq!(second.topic, topic_b.id);
    {
        let storage = state.storage.lock().unwrap();
        for t in [&first, &second] {
            let batch = super::link::turn_batch(&t.id, &t.topic, &[], NOW);
            state.world.write(&state, &storage, &batch, NOW).unwrap();
        }
    }
    let in_topic = |topic: &str| {
        super::assemble::assemble(
            &state,
            &super::assemble::Ask {
                question: "and now",
                world: &world,
                rows: &[],
                topic: Some(topic),
                recent: &[],
            },
        )
        .text
    };
    let a = in_topic(&first.topic);
    assert!(a.contains("cart review"), "{a}");
    assert!(!a.contains("docs site"), "{a}");
    let b = in_topic(&second.topic);
    assert!(b.contains("docs site") && !b.contains("cart review"), "{b}");
}

/// Bot's breakdown (AJ-E3825 R9) is built on the world model: its part of
/// the context, what went in and why, and the memories by kind, once each.
#[test]
fn bots_breakdown_shows_what_the_world_model_put_in() {
    let (_dir, state) = state();
    ask(&state, "remember the release goes out on fridays");
    let turn = ask(&state, "when does the release go out");
    let details = turn.details.unwrap();
    let world_part = details
        .context
        .parts
        .iter()
        .find(|p| p.part == crate::features::bot::dto::BotContextPart::World)
        .unwrap();
    assert!(world_part.tokens.n > 0);
    assert!(
        details
            .world
            .iter()
            .any(|w| w.memory && w.why == "matches your words"),
        "{:?}",
        details.world
    );
    let notes = details.memory.iter().find(|m| m.kind == "notes").unwrap();
    assert_eq!(notes.n, 1);
}

/// ADR-0038 §6's permanent check: 27 canaries planted in every place a
/// memory lives (a note's record and its index, a name the user gave and
/// the world model's alias for it, the turns they came from and the
/// conversation's title, a question asked, a fact the world model derived
/// from a note, the log's sidecars), forgotten through the app's own paths,
/// then searched for in the database file, its WAL and every other file,
/// both keyword indexes' terms, and the in-memory index: 0 hits.
#[test]
fn forget_leaves_none_of_27_canaries_anywhere() {
    let (dir, state) = state();
    let world = world();
    sync(&state, &world);
    let canaries: Vec<String> = (0..27).map(|i| format!("qzcanary{i:02}w")).collect();
    let mut keys: Vec<String> = Vec::new();
    // 0..9: notes said in Bot ("remember …"): the record, its index entry,
    // the turn's question and Bot's echo.
    for c in &canaries[0..9] {
        ask(&state, &format!("remember the {c} release note"));
    }
    // 9..15: names the user gave a session: the record, the memory node's
    // alias, the turn they were said in.
    for c in &canaries[9..15] {
        let t = turn(&state, &format!("call web {c}"));
        crate::features::bot::memory::remember(&state, &world, &format!("call web {c}"), &t.id)
            .unwrap()
            .unwrap();
        // Each new name supersedes the last; its subject is one key.
    }
    // 15..21: questions asked: the turns (Clear chat history takes them).
    for c in &canaries[15..21] {
        ask(&state, &format!("is {c} finished"));
    }
    {
        let storage = state.storage.lock().unwrap();
        let store = storage.memory();
        for kind in [
            juggler_core::storage::memory::Kind::Note,
            juggler_core::storage::memory::Kind::Preference,
        ] {
            for r in store.current_of_kind(kind, None, 100).unwrap() {
                keys.push(r.subject_key.clone());
            }
        }
        let notes = store
            .current_of_kind(juggler_core::storage::memory::Kind::Note, None, 100)
            .unwrap();
        // 21..24: facts the world model derived from a note.
        let mut batch = gyrus::Batch::new();
        for (i, c) in canaries[21..24].iter().enumerate() {
            let fact = format!("fact:c{i}");
            batch
                .node(gyrus::NewNode::new(
                    "fact",
                    &fact,
                    gyrus::Label::Trusted(format!("{c} ships fridays")),
                    NOW,
                ))
                .link(gyrus::NewLink::new(
                    &fact,
                    "derived_from",
                    &memory_key(&notes[i].subject_id),
                    "test",
                ));
        }
        state.world.write(&state, &storage, &batch, NOW).unwrap();
        // 24..27: the log's sidecars about a note.
        for (i, c) in canaries[24..27].iter().enumerate() {
            let mut event = juggler_core::storage::memory::NewEvent::new(
                NOW,
                juggler_core::storage::memory::Stream::Feedback,
                "wrong",
                juggler_core::storage::memory::Actor::User,
            );
            event.memory_subject = Some(notes[i].subject_id.clone());
            storage
                .ai_log()
                .append(
                    &event,
                    Some((
                        &juggler_core::storage::memory::NewSidecar {
                            body: &format!("the user flagged {c}"),
                            untrusted: false,
                            expires_ms: NOW + 86_400_000,
                        },
                        &crate::features::ai::memory::RedactionSecrets,
                    )),
                )
                .unwrap();
        }
        storage
            .raw_conn()
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .unwrap();
    }
    let data = dir.path();
    let planted = canaries
        .iter()
        .filter(|c| !crate::test_support::holding(data, c).is_empty())
        .count();
    assert_eq!(planted, 27, "not every canary was planted");
    assert_eq!(keys.len(), 10, "9 notes and one alias subject: {keys:?}");
    // Forget, through the app's own paths: each memory, then the chat.
    for key in &keys {
        crate::features::ai::memory::forget(
            &state,
            key,
            &juggler_core::storage::memory::Actor::User,
        )
        .unwrap();
    }
    crate::features::bot::commands::bot_clear_impl(&state).unwrap();
    let storage = state.storage.lock().unwrap();
    let snap = state.world.ready(&state, &storage).unwrap();
    let conn = storage.raw_conn();
    let mut vocab = vec!["memory_fts".to_owned()];
    vocab.extend((0..gyrus::FTS_BUCKETS).map(gyrus::schema::fts_table));
    for table in &vocab {
        conn.execute_batch(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS temp.v_{table} USING fts5vocab(main, {table}, row);"
        ))
        .unwrap();
    }
    let mut hits: Vec<String> = Vec::new();
    for c in &canaries {
        for path in crate::test_support::holding(data, c) {
            hits.push(format!("{c} in {}", path.display()));
        }
        for table in &vocab {
            let n: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM temp.v_{table} WHERE instr(term, ?1) > 0"),
                    [&c[..10]],
                    |r| r.get(0),
                )
                .unwrap();
            if n > 0 {
                hits.push(format!("{c} in {table}'s terms"));
            }
        }
        if snap.holds_text(c) {
            hits.push(format!("{c} in the in-memory index"));
        }
    }
    assert!(hits.is_empty(), "{} hits: {hits:?}", hits.len());
}

/// ADR-0038 §4 inside ADR-0037 §2.3's maintenance: one pass links the
/// records written before, prunes a memory node whose record went another
/// way, runs the idle pass (an idle low note archived, kept), logs one
/// `memory.maintained` with counts only, and stops when a job arrives.
#[test]
fn the_upkeep_pass_links_prunes_dreams_and_logs_counts_only() {
    let (_dir, state) = state();
    let world = world();
    {
        let storage = state.storage.lock().unwrap();
        let writer = juggler_core::storage::memory::Writers::for_tests().memory;
        crate::features::bot::memory::remember_in(
            &storage,
            &writer,
            &world,
            "the qzupkeep release goes out on fridays",
            "t1",
        )
        .unwrap()
        .unwrap();
    }
    // A memory node with no record (its record cleared another way), and
    // an idle low note the idle pass archives.
    {
        let storage = state.storage.lock().unwrap();
        let mut b = gyrus::Batch::new();
        b.node(gyrus::NewNode::new(
            "memory",
            "memory:gone-subject",
            gyrus::Label::none(),
            NOW,
        ))
        .node(gyrus::NewNode {
            priority: 0,
            ..gyrus::NewNode::new(
                "fact",
                "fact:old",
                gyrus::Label::Trusted("an old fact".into()),
                NOW - 200 * 86_400_000,
            )
        });
        state.world.write(&state, &storage, &b, NOW).unwrap();
    }
    let passed = super::maintain::pass(&state, &|| false);
    assert!(passed.linked && !passed.stopped, "{passed:?}");
    assert_eq!(passed.dangling, 1);
    assert!(passed.dreamed.archived >= 1, "{passed:?}");
    let storage = state.storage.lock().unwrap();
    let snap = state.world.ready(&state, &storage).unwrap();
    assert!(snap.id_of("memory:gone-subject").is_none());
    assert!(
        snap.node(snap.id_of("fact:old").unwrap()).unwrap().archived,
        "archived, kept"
    );
    let note = storage
        .memory()
        .current_of_kind(juggler_core::storage::memory::Kind::Note, None, 10)
        .unwrap();
    assert!(
        snap.id_of(&memory_key(&note[0].subject_id)).is_some(),
        "the record wasn't linked"
    );
    let payload: String = storage
        .raw_conn()
        .query_row(
            "SELECT payload FROM ai_log WHERE verb = 'maintained' ORDER BY seq DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        payload.contains("\"archived\"") && !payload.contains("qzupkeep"),
        "{payload}"
    );
    drop(storage);
    // A job arrives: the pass stops at once.
    let stopped = super::maintain::pass(&state, &|| true);
    assert!(stopped.stopped || stopped.writes == 0, "{stopped:?}");
}
