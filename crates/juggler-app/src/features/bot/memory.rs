//! "Remember …" and "What Bot remembers" (AJ-E3819's UI part, ADR-0037
//! §2.4 and §9): only what the user said, handled by code, no model.
//!
//! - **`call X Y`** keeps Y as the user's name for the record X (an
//!   `alias` preference naming it by id); **`X matters most`** keeps X as
//!   what matters (a `focus` preference); anything else is a `note`, the
//!   user's own words.
//! - **Used in later questions:** a question that says a name the user
//!   gave is read as the record's own name, and the answer's "Looked at"
//!   names the memory (its receipt), with Forget.
//! - **Forget** goes through the app's one forget path
//!   ([`crate::features::ai::memory::forget`]): every version, the
//!   message it came from cleared, the staged backup dropped, and the
//!   sheet shows `FORGET_AND_BACKUPS` word for word.

use std::sync::Arc;

use juggler_core::storage::memory::{
    Actor, Authority, EntityKind, EntityRef, Kind, MemoryError, NewRecord, Priority, Record,
    RecordText, Refusal, Scope, SecretScan, SettingLabel, SourceKind, Tier,
};
use serde::Serialize;

use super::dto::{BotMemoryInfo, BotRowInfo, BotRowKind};
use super::world::World;
use crate::error::{AppError, AppResult};
use crate::features::ai::memory::{FORGET_AND_BACKUPS, RedactionSecrets, memory_error};
use crate::state::AppState;

/// What "What Bot remembers" shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotMemoriesInfo {
    /// The user's names, focus and notes, newest change first.
    pub items: Vec<BotMemoryInfo>,
    /// The forget sheet's backup words, word for word.
    pub forget_and_backups: String,
    /// Watches kept (they are listed in Watches).
    pub watches: u32,
    /// Questions in the conversation.
    pub questions: u32,
}

/// The most memories listed at once (the page is never loaded all).
const LISTED: u32 = 200;

/// A record named in "call X Y" or "X matters most".
struct Named {
    kind: EntityKind,
    id: String,
    name: String,
}

/// The records a name may be: sessions, projects and pools, by their
/// names, the longest first (so "Fix the login redirect" wins over "Fix").
fn named_records(world: &World) -> Vec<Named> {
    let mut out: Vec<Named> = world
        .sessions
        .iter()
        .filter(|r| !r.title.is_empty())
        .map(|r| Named {
            kind: EntityKind::Session,
            id: r.id.clone(),
            name: r.title.clone(),
        })
        .collect();
    for row in world.sessions.iter().chain(&world.tasks) {
        if let (Some(id), Some(name)) = (&row.project_id, &row.project)
            && !out
                .iter()
                .any(|n| n.kind == EntityKind::Project && &n.id == id)
        {
            out.push(Named {
                kind: EntityKind::Project,
                id: id.clone(),
                name: name.clone(),
            });
        }
        if row.kind == BotRowKind::Task
            && let (Some(id), Some(name)) = (&row.pool_id, &row.pool)
            && !out
                .iter()
                .any(|n| n.kind == EntityKind::Pool && &n.id == id)
        {
            out.push(Named {
                kind: EntityKind::Pool,
                id: id.clone(),
                name: name.clone(),
            });
        }
    }
    out.sort_by_key(|n| std::cmp::Reverse(n.name.chars().count()));
    out
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// A stable short key for a note's words.
fn note_key(words: &str) -> String {
    let normal: String = words
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in normal.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("note:{hash:016x}")
}

/// What "Remember …" keeps, before it is written.
fn record_for(world: &World, words: &str, turn: &str, now_ms: i64) -> NewRecord {
    let base = |kind: Kind,
                key: String,
                text: RecordText,
                entities: Vec<EntityRef>,
                payload: serde_json::Value| NewRecord {
        subject_key: key,
        kind,
        scope: Scope::Global,
        tier: Tier::Long,
        priority: Priority::Normal,
        authority: Authority::User,
        source: SourceKind::ChatTurn,
        source_id: Some(turn.to_owned()),
        engine: None,
        confidence: 1.0,
        as_of_ms: now_ms,
        source_rev: None,
        text,
        payload,
        expires_ms: None,
        entities,
    };
    let records = named_records(world);
    let call = ["call ", "nenn ", "nenne "]
        .iter()
        .find_map(|cue| strip_prefix_ci(words, cue));
    if let Some(rest) = call {
        for named in &records {
            if let Some(alias) = strip_prefix_ci(rest, &named.name).filter(|a| a.starts_with(' ')) {
                let alias = alias.trim().trim_matches(['"', '“', '”', '.']).trim();
                if let Some(label) = SettingLabel::new(alias) {
                    return base(
                        Kind::Preference,
                        format!("alias:{}:{}", named.kind.as_str(), named.id),
                        RecordText::Setting(label),
                        vec![EntityRef {
                            kind: named.kind,
                            id: named.id.clone(),
                        }],
                        serde_json::json!({ "slot": "alias", "about": named.name }),
                    );
                }
            }
        }
    }
    let lower = words.to_lowercase();
    for cue in [
        " matters most",
        " is most important",
        " ist am wichtigsten",
        " ist mir am wichtigsten",
    ] {
        if let Some(subject) = lower.strip_suffix(cue)
            && let Some(named) = records
                .iter()
                .find(|n| n.name.to_lowercase() == subject.trim().trim_matches(['"', '“', '”']))
            && let Some(label) = SettingLabel::new(&named.name)
        {
            return base(
                Kind::Preference,
                "focus:global".to_owned(),
                RecordText::Setting(label),
                vec![EntityRef {
                    kind: named.kind,
                    id: named.id.clone(),
                }],
                serde_json::json!({ "slot": "focus", "about": named.name }),
            );
        }
    }
    base(
        Kind::Note,
        note_key(words),
        RecordText::Trusted(words.to_owned()),
        Vec::new(),
        serde_json::json!({ "slot": "note" }),
    )
}

/// A stored record as the window shows it.
#[must_use]
pub fn info_of(record: &Record, versions: u32) -> BotMemoryInfo {
    let slot = record
        .payload
        .get("slot")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("note")
        .to_owned();
    let about = record
        .payload
        .get("about")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let about_kind = record
        .subject_key
        .split(':')
        .nth(1)
        .filter(|_| slot == "alias")
        .map(str::to_owned);
    BotMemoryInfo {
        key: record.subject_key.clone(),
        slot,
        text: record.text.clone(),
        about,
        about_kind,
        at_ms: record.as_of_ms,
        used: u32::try_from(record.use_count).unwrap_or(0),
        versions,
    }
}

/// Keeps what the user said after "Remember", from the turn `turn`.
/// `Ok(None)`: the gate refused it (a key or password, too long).
///
/// # Errors
///
/// Storage failures; the store's writer isn't this process's.
pub fn remember(
    state: &Arc<AppState>,
    world: &World,
    words: &str,
    turn: &str,
) -> AppResult<Option<BotMemoryInfo>> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let graph = state.world.graph();
    state
        .memory_writers
        .with_memory(|writer| remember_linked_in(&storage, writer, world, words, turn, graph))
        .ok_or_else(|| AppError::Validation {
            subject: "memory",
            message: "unavailable".to_owned(),
        })?
}

/// [`remember`] with the store and the writer in hand.
///
/// # Errors
///
/// As [`remember`].
pub fn remember_in(
    storage: &juggler_core::storage::Storage,
    writer: &juggler_core::storage::memory::KindWriter,
    world: &World,
    words: &str,
    turn: &str,
) -> AppResult<Option<BotMemoryInfo>> {
    remember_linked_in(storage, writer, world, words, turn, None)
}

/// [`remember_in`], the record's node and links in Bot's world model
/// written in the same transaction (ADR-0038 §1) when `graph` is given and
/// its tables are there: a `memory` node, its alias the user's own name for
/// the record, `about` each entity it names.
///
/// # Errors
///
/// As [`remember`].
pub fn remember_linked_in(
    storage: &juggler_core::storage::Storage,
    writer: &juggler_core::storage::memory::KindWriter,
    world: &World,
    words: &str,
    turn: &str,
    graph: Option<&gyrus::Graph>,
) -> AppResult<Option<BotMemoryInfo>> {
    if RedactionSecrets.finds_secret(words) {
        return Ok(None);
    }
    let record = record_for(world, words, turn, world.now_ms);
    let store = storage.memory();
    let linked = graph.filter(|_| storage.world().installed());
    let written = match linked {
        Some(graph) => {
            let alias = (record.kind == Kind::Preference
                && record.subject_key.starts_with("alias:"))
            .then(|| record.text.as_str().to_owned());
            let link = |w: &juggler_core::storage::memory::Remembered| {
                crate::features::ai::world::link::record_batch(
                    &crate::features::ai::world::link::MemoryFacts {
                        subject_id: &w.subject_id,
                        kind: record.kind,
                        tier: record.tier,
                        priority: record.priority.as_i64(),
                        authority: record.authority,
                        untrusted: record.text.is_untrusted(),
                        as_of_ms: record.as_of_ms,
                        alias: alias.as_deref(),
                        entities: &record.entities,
                    },
                )
                .unwrap_or_default()
            };
            store.remember_linked(
                writer,
                &RedactionSecrets,
                &record,
                graph,
                &link,
                world.now_ms,
            )
        }
        None => store.remember(writer, &RedactionSecrets, &record, world.now_ms),
    };
    match written {
        Ok(_) => {}
        Err(MemoryError::Refused(
            Refusal::Secret | Refusal::TextBounds | Refusal::PreferenceShape,
        )) => return Ok(None),
        Err(other) => return Err(memory_error(other)),
    }
    let current = store.current(&record.subject_key).map_err(memory_error)?;
    let versions = store
        .history(&record.subject_key)
        .map_or(0, |h| u32::try_from(h.len().saturating_sub(1)).unwrap_or(0));
    Ok(current.map(|r| info_of(&r, versions)))
}

/// The user's names in a question, read as the records' own names, and
/// the memories that did it (the receipt).
#[must_use]
pub fn with_aliases(
    state: &AppState,
    world: &World,
    question: &str,
) -> (String, Vec<(BotMemoryInfo, String)>) {
    let Ok(storage) = state.storage.lock() else {
        return (question.to_owned(), Vec::new());
    };
    let store = storage.memory();
    let Ok(prefs) = store.current_of_kind(Kind::Preference, None, LISTED) else {
        return (question.to_owned(), Vec::new());
    };
    let mut text = question.to_owned();
    let mut used = Vec::new();
    for record in prefs.iter().filter(|r| r.subject_key.starts_with("alias:")) {
        let alias = record.text.to_lowercase();
        let lower = text.to_lowercase();
        if lower.len() != text.len() {
            continue;
        }
        let Some(at) = lower.find(&alias) else {
            continue;
        };
        let mut parts = record.subject_key.splitn(3, ':').skip(1);
        let (Some(kind), Some(id)) = (parts.next(), parts.next()) else {
            continue;
        };
        let name = match kind {
            "session" => world
                .sessions
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.title.clone()),
            "project" => world.project_name(id),
            "pool" => world.pool_name(id),
            _ => None,
        };
        let Some(name) = name else {
            continue;
        };
        // "the api agent" → "web": the alias's own words are replaced,
        // a leading "the" with them.
        let mut start = at;
        if text[..at].to_lowercase().ends_with("the ") {
            start = at - 4;
        }
        text = format!("{}{}{}", &text[..start], name, &text[at + alias.len()..]);
        used.push((info_of(record, 0), record.subject_id.clone()));
        let _ = store.touch(&[record.id.as_str()], world.now_ms);
    }
    (text, used)
}

/// The words that ask what Bot was told to remember (AJ-E3832 G9).
const RECALL_CUES: &[&str] = &[
    "remember",
    "told you",
    "asked you",
    "what do i call",
    "what did i call",
    "my name for",
    "merken",
    "gemerkt",
    "gesagt",
    "wie nenne ich",
];

/// The question asks what the user told Bot (not a "Remember …" itself).
#[must_use]
pub fn asks_about_memory(question: &str) -> bool {
    let lower = question.to_lowercase();
    RECALL_CUES.iter().any(|cue| lower.contains(cue))
}

/// The most memories offered as records for one question.
const MEMORY_ROWS: usize = 5;

/// A memory as a citable record (AJ-E3832 G9): its id the subject's
/// random id, its words the user's own.
#[must_use]
pub fn row_of(record: &Record) -> BotRowInfo {
    let info = info_of(record, 0);
    BotRowInfo {
        at_ms: Some(record.as_of_ms),
        about: info.about,
        memory_key: Some(record.subject_key.clone()),
        ..super::world::empty_row(BotRowKind::Memory, record.subject_id.clone(), info.text)
    }
}

/// The memories a question about memory rests on, through ADR-0037's read
/// path: the ones the world model linked to it (`keys`), else the newest
/// the user stated; the user's own words only (never agent text), at most
/// a handful.
#[must_use]
pub fn rows_for(storage: &juggler_core::storage::Storage, keys: &[String]) -> Vec<BotRowInfo> {
    let store = storage.memory();
    let mut records: Vec<Record> = keys
        .iter()
        .filter_map(|key| store.current(key).ok().flatten())
        .collect();
    if records.is_empty() {
        for kind in [Kind::Preference, Kind::Note] {
            records.extend(
                store
                    .current_of_kind(kind, None, LISTED)
                    .unwrap_or_default(),
            );
        }
        records.sort_by_key(|r| std::cmp::Reverse(r.as_of_ms));
    }
    let mut seen = std::collections::BTreeSet::new();
    records
        .into_iter()
        .filter(|r| !r.untrusted && matches!(r.kind, Kind::Preference | Kind::Note))
        .filter(|r| seen.insert(r.subject_id.clone()))
        .take(MEMORY_ROWS)
        .map(|r| row_of(&r))
        .collect()
}

/// A memory as it stands now, by its subject key, or by its random subject
/// id written `subject:<id>` (a kept turn's memory row); `None`: forgotten.
#[must_use]
pub fn current(storage: &juggler_core::storage::Storage, key: &str) -> Option<BotMemoryInfo> {
    let store = storage.memory();
    match key.strip_prefix("subject:") {
        Some(subject) => store.current_by_subject(subject),
        None => store.current(key),
    }
    .ok()
    .flatten()
    .map(|record| info_of(&record, 0))
}

/// Every memory the user stated, for "What Bot remembers".
///
/// # Errors
///
/// Storage failures.
pub fn list(state: &Arc<AppState>) -> AppResult<BotMemoriesInfo> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let store = storage.memory();
    let mut items: Vec<(i64, BotMemoryInfo)> = Vec::new();
    for kind in [Kind::Preference, Kind::Note] {
        for record in store
            .current_of_kind(kind, None, LISTED)
            .map_err(memory_error)?
        {
            let versions = store
                .history(&record.subject_key)
                .map_or(0, |h| u32::try_from(h.len().saturating_sub(1)).unwrap_or(0));
            items.push((record.created_ms, info_of(&record, versions)));
        }
    }
    items.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    let watches = super::commands::kept_watches(&storage);
    let questions = storage.chat().count().map_or(0, |c| c.turns);
    Ok(BotMemoriesInfo {
        items: items.into_iter().map(|(_, info)| info).collect(),
        forget_and_backups: FORGET_AND_BACKUPS.to_owned(),
        watches,
        questions,
    })
}

/// The keys "What Bot remembers" may forget: what the user stated to Bot
/// (an alias, a focus, a note) and the feed's More or Less like this
/// (`feed:`, AJ-E3713: the user's own preference, kept from Home's feed).
const FORGETTABLE: &[&str] = &["alias:", "focus:", "note:", "feed:"];

/// Forgets one memory, every version of it (ADR-0037 §1.5), through the
/// store's one forget.
///
/// # Errors
///
/// Not a memory the user stated; the store's refusal; storage.
pub fn forget(state: &Arc<AppState>, key: &str) -> AppResult<()> {
    if !FORGETTABLE.iter().any(|prefix| key.starts_with(prefix)) {
        return Err(AppError::Validation {
            subject: "memory",
            message: "not_a_memory".to_owned(),
        });
    }
    crate::features::ai::memory::forget(state, key, &Actor::User)?;
    Ok(())
}
