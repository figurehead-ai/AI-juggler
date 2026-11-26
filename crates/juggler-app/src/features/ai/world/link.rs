//! What links the world model to AI Juggler's records (ADR-0038 §2):
//!
//! - **A memory record** (ADR-0037) is a `memory` node named by its
//!   subject's random id, linked `about` the entities it names; a
//!   preference's alias (the user's own name for a session) is the memory
//!   node's alias, so it is found by exactly those words and forgotten
//!   with it. Nothing moves: the record stays where it is.
//! - **The entities** (sessions, Apps, projects, pools, tasks, Accounts)
//!   are pointer nodes linked as the records relate them: a session
//!   `part_of` its project and `runs_in` its App, a task `part_of` its pool
//!   and project, a pool `part_of` its project.
//! - **A Chat turn** is a `turn_summary` node `part_of` its `topic`,
//!   `asked_about` the records its answer rested on.
//!
//! The one-time pass ([`link_existing`]) links the records written before
//! this version, in bounded batches; later records are linked as they are
//! written (Bot's remember flow, [`record_batch`]).

use std::collections::BTreeSet;

use gyrus::{Authority as GAuthority, Batch, Label, NewLink, NewNode, Snapshot, Tier as GTier};
use juggler_core::storage::Storage;
use juggler_core::storage::memory::world::{memory_key, topic_key, turn_key};
use juggler_core::storage::memory::{Authority, EntityKind, EntityRef, Kind, Record, Tier};

use crate::features::bot::dto::{BotRowInfo, BotRowKind};
use crate::features::bot::world::World;

/// The setting that says the one-time pass is done.
pub const LINKED_SETTING: &str = "world.linked_v1";
/// The most records one batch of the one-time pass links (one short hold).
pub const LINK_BATCH: u32 = 100;

/// An entity's node key.
#[must_use]
pub fn entity_key(kind: EntityKind, id: &str) -> String {
    format!("{}:{id}", kind.as_str())
}

/// A pointer node (no label, no state).
fn pointer(kind: &str, key: &str, now: i64) -> NewNode {
    NewNode {
        tier: GTier::Long,
        ..NewNode::new(kind, key, Label::none(), now)
    }
}

const fn tier_of(tier: Tier) -> GTier {
    match tier {
        Tier::Short => GTier::Short,
        Tier::Mid => GTier::Mid,
        Tier::Long => GTier::Long,
    }
}

/// What a memory record's node is made from.
#[derive(Clone, Copy, Debug)]
pub struct MemoryFacts<'a> {
    /// The subject's random id (the node's key).
    pub subject_id: &'a str,
    /// Its kind.
    pub kind: Kind,
    /// Its tier.
    pub tier: Tier,
    /// Its priority, 0..=3.
    pub priority: i64,
    /// Whose words.
    pub authority: Authority,
    /// It holds agent words.
    pub untrusted: bool,
    /// When it was said.
    pub as_of_ms: i64,
    /// The user's own name for an entity (an alias preference's label).
    pub alias: Option<&'a str>,
    /// The entities it names.
    pub entities: &'a [EntityRef],
}

/// The record's node and its links, or `None` for a record the world
/// model never holds: one with agent words (untrusted rows are never
/// recalled, ADR-0037 §1.2), or a kind that isn't the user's or the app's
/// memory (a digest).
#[must_use]
pub fn record_batch(facts: &MemoryFacts<'_>) -> Option<Batch> {
    let MemoryFacts {
        subject_id,
        kind,
        tier,
        priority,
        authority,
        untrusted,
        as_of_ms,
        alias,
        entities,
    } = *facts;
    if untrusted || matches!(kind, Kind::Digest) {
        return None;
    }
    let authority = match authority {
        Authority::User => GAuthority::User,
        Authority::App => GAuthority::App,
        Authority::Model => GAuthority::Model,
        Authority::Agent | Authority::Bot => return None,
    };
    let key = memory_key(subject_id);
    let mut batch = Batch::new();
    batch.node(NewNode {
        tier: tier_of(tier),
        priority: u8::try_from(priority.clamp(0, 3)).unwrap_or(1),
        authority,
        aliases: alias.map(|a| vec![a.to_owned()]).unwrap_or_default(),
        props: serde_json::json!({ "record": kind.as_str() }),
        ..NewNode::new("memory", &key, Label::none(), as_of_ms)
    });
    let mut seen = BTreeSet::new();
    for entity in entities {
        let node = entity_key(entity.kind, &entity.id);
        if !seen.insert(node.clone()) {
            continue;
        }
        batch
            .node(pointer(entity.kind.as_str(), &node, as_of_ms))
            .link(NewLink::new(&key, "about", &node, "record"));
    }
    Some(batch)
}

/// [`record_batch`] for a stored record and its entities.
#[must_use]
pub fn stored_batch(record: &Record, entities: &[EntityRef]) -> Option<Batch> {
    let alias = (record.kind == Kind::Preference && record.subject_key.starts_with("alias:"))
        .then_some(record.text.as_str());
    record_batch(&MemoryFacts {
        subject_id: &record.subject_id,
        kind: record.kind,
        tier: record.tier,
        priority: record.priority.as_i64(),
        authority: record.authority,
        untrusted: record.untrusted,
        as_of_ms: record.as_of_ms,
        alias,
        entities,
    })
}

fn has_link(snap: Option<&Snapshot>, pack: &gyrus::Pack, src: &str, link: &str, dst: &str) -> bool {
    let Some(snap) = snap else {
        return false;
    };
    let (Some(s), Some(d), Some(l)) = (snap.id_of(src), snap.id_of(dst), pack.link_index(link))
    else {
        return false;
    };
    snap.out(s, l).iter().any(|e| e.other == d)
}

/// The entities the records hold now and how they relate, as the writes
/// the index lacks (nothing for what it already has, so a pass over an
/// unchanged world writes nothing).
#[must_use]
pub fn entities_batch(
    world: &World,
    snap: Option<&Snapshot>,
    pack: &gyrus::Pack,
    now: i64,
) -> Batch {
    let mut batch = Batch::new();
    let mut nodes: BTreeSet<String> = BTreeSet::new();
    let mut node = |batch: &mut Batch, kind: &str, key: String| {
        let known = snap.is_some_and(|s| s.id_of(&key).is_some());
        if !known && nodes.insert(key.clone()) {
            batch.node(pointer(kind, &key, now));
        }
        key
    };
    let mut links: Vec<(String, &str, String)> = Vec::new();
    let rows: Vec<&BotRowInfo> = world
        .sessions
        .iter()
        .chain(&world.tasks)
        .chain(&world.accounts)
        .collect();
    for row in rows {
        let own = match row.kind {
            BotRowKind::Session => node(&mut batch, "session", format!("session:{}", row.id)),
            BotRowKind::Task => node(&mut batch, "task", format!("task:{}", row.id)),
            BotRowKind::Account => {
                node(&mut batch, "account", format!("account:{}", row.id));
                continue;
            }
        };
        if let Some(project) = &row.project_id {
            let p = node(&mut batch, "project", format!("project:{project}"));
            links.push((own.clone(), "part_of", p));
        }
        if row.kind == BotRowKind::Session
            && let Some(app) = &row.app_id
        {
            let a = node(&mut batch, "app", format!("app:{app}"));
            links.push((own.clone(), "runs_in", a));
        }
        if row.kind == BotRowKind::Task
            && let Some(pool) = &row.pool_id
        {
            let p = node(&mut batch, "pool", format!("pool:{pool}"));
            links.push((own.clone(), "part_of", p));
        }
    }
    let mut seen = BTreeSet::new();
    for (src, link, dst) in links {
        if seen.insert((src.clone(), link, dst.clone())) && !has_link(snap, pack, &src, link, &dst)
        {
            batch.link(NewLink::new(&src, link, &dst, "records"));
        }
    }
    batch
}

/// A Chat turn's nodes: the turn, its topic, and what it was about (the
/// records its answer rested on, by kind and id).
#[must_use]
pub fn turn_batch(
    turn_id: &str,
    topic_id: &str,
    about: &[(BotRowKind, String)],
    now: i64,
) -> Batch {
    let mut batch = Batch::new();
    let turn = turn_key(turn_id);
    let topic = topic_key(topic_id);
    batch
        .node(NewNode {
            tier: GTier::Long,
            ..NewNode::new("topic", &topic, Label::none(), now)
        })
        .node(NewNode::new("turn_summary", &turn, Label::none(), now))
        .link(NewLink::new(&turn, "part_of", &topic, "turn"));
    let mut seen = BTreeSet::new();
    for (kind, id) in about.iter().take(8) {
        let (kind, key) = match kind {
            BotRowKind::Session => ("session", format!("session:{id}")),
            BotRowKind::Task => ("task", format!("task:{id}")),
            BotRowKind::Account => ("account", format!("account:{id}")),
        };
        if seen.insert(key.clone()) {
            batch.node(pointer(kind, &key, now)).link(NewLink::new(
                &turn,
                "asked_about",
                &key,
                "turn",
            ));
        }
    }
    batch
}

/// The current records the one-time pass links: trusted only, at most
/// `limit` after the cursor, with their entities.
fn unlinked(storage: &Storage, after: i64, limit: u32) -> Vec<(i64, Record, Vec<EntityRef>)> {
    let store = storage.memory();
    store
        .current_page(after, limit)
        .unwrap_or_default()
        .into_iter()
        .map(|(seq, record)| {
            let entities = store.entities(&record.id).unwrap_or_default();
            (seq, record, entities)
        })
        .collect()
}

/// One batch of the one-time pass: the records after `after` linked as
/// `memory` nodes. Returns the next cursor, `None` when every record is
/// linked (the pass is then marked done).
pub fn link_existing(
    storage: &Storage,
    graph: &gyrus::Graph,
    writer: &juggler_core::storage::memory::KindWriter,
    after: i64,
    now: i64,
) -> Option<i64> {
    let rows = unlinked(storage, after, LINK_BATCH);
    let mut batch = Batch::new();
    let mut last = after;
    for (seq, record, entities) in &rows {
        last = *seq;
        if let Some(part) = stored_batch(record, entities) {
            batch.ops.extend(part.ops);
        }
    }
    if !batch.is_empty() {
        let _ = storage.memory().write_world(
            writer,
            &crate::features::ai::memory::RedactionSecrets,
            graph,
            &batch,
            now,
        );
    }
    if rows.len() < usize::try_from(LINK_BATCH).unwrap_or(usize::MAX) {
        let _ = storage.app_settings().set(LINKED_SETTING, "1");
        return None;
    }
    Some(last)
}
