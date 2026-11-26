//! AJ-E3713 R3 — a post's More or Less like this, kept as a `preference`
//! memory (ADR-0037 §1.1's `feed` slot) through the store's write gate,
//! with the memory writer lent by its owner (`features/ai/memory`), as
//! Bot's remember flow borrows it. One current record per scope: the kind
//! of event (its group), the App, the project; a new lean on the same
//! scope supersedes the last (§1.4), and Undo forgets it through the
//! app's one forget path. The feed reads them back to rank and fold
//! posts (`ui/src/lib/feed-rank.ts`); "Why am I seeing this?" names the
//! one behind a post. No sentence here: the label is a setting's name.

use std::sync::Arc;

use juggler_core::storage::Storage;
use juggler_core::storage::memory::{
    Actor, Authority, EntityKind, EntityRef, Kind, KindWriter, MemoryError, NewRecord, Priority,
    Record, RecordText, Refusal, Scope, SettingLabel, SourceKind, Tier,
};

use crate::error::{AppError, AppResult};
use crate::features::ai::memory::{RedactionSecrets, memory_error};
use crate::state::AppState;

/// The groups a post's kind falls in: what "the kind of event" a lean
/// names, and its words ("Fewer switch posts").
pub const GROUPS: &[(&str, &str)] = &[
    ("finish", "finished work"),
    ("failure", "failure"),
    ("need", "answered need"),
    ("switch", "switch"),
    ("usage", "usage"),
    ("task", "task"),
    ("pool", "pool"),
    ("mac", "This Mac"),
    ("digest", "digest"),
    ("watch", "watch"),
];

/// The most leans read back at once (the store's page; a person sets a
/// few dozen at most).
const READ: u32 = 200;

/// The id AI Juggler's own posts are scoped by when they name no App.
pub const OWN_APP: &str = "ai-juggler";

/// A lean the user gave a post.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FeedLeanAsk {
    /// `more` or `less`.
    pub lean: String,
    /// The kind of event, by group ([`GROUPS`]).
    pub group: String,
    /// The App's Account folder (none: every App).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub folder: Option<String>,
    /// The project, by id (none: every project).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub project: Option<String>,
    /// The App's or project's name, said with the lean ("in web-app").
    #[serde(default)]
    #[ts(optional = nullable)]
    pub about: Option<String>,
}

/// A lean kept.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FeedLeanInfo {
    /// Its memory's subject key (Undo forgets it).
    pub key: String,
    /// `more` or `less`.
    pub lean: String,
    /// See [`FeedLeanAsk`].
    pub group: String,
    /// See [`FeedLeanAsk`].
    pub folder: Option<String>,
    /// See [`FeedLeanAsk`].
    pub project: Option<String>,
    /// See [`FeedLeanAsk`].
    pub about: Option<String>,
    /// When it was said, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// A stable short code for a scope's ids (they may hold any character;
/// a subject key is a code).
fn scope_code(folder: Option<&str>, project: Option<&str>) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in folder
        .unwrap_or("")
        .bytes()
        .chain([0])
        .chain(project.unwrap_or("").bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The words of a group ("switch"), `None` for one not in [`GROUPS`].
fn group_words(group: &str) -> Option<&'static str> {
    GROUPS
        .iter()
        .find(|(code, _)| *code == group)
        .map(|(_, words)| *words)
}

/// The record a lean is kept as, `None` when the ask isn't one.
#[must_use]
pub fn record_of(ask: &FeedLeanAsk, now_ms: i64) -> Option<NewRecord> {
    let words = group_words(&ask.group)?;
    let lean = match ask.lean.as_str() {
        "more" => "More",
        "less" => "Fewer",
        _ => return None,
    };
    let folder = ask.folder.as_deref().filter(|f| !f.is_empty());
    let project = ask.project.as_deref().filter(|p| !p.is_empty());
    let mut entities = Vec::new();
    if let Some(folder) = folder {
        entities.push(EntityRef {
            kind: EntityKind::App,
            id: folder.to_owned(),
        });
    }
    if let Some(project) = project {
        entities.push(EntityRef {
            kind: EntityKind::Project,
            id: project.to_owned(),
        });
    }
    if entities.is_empty() {
        // AI Juggler's own posts everywhere: the App they come from.
        entities.push(EntityRef {
            kind: EntityKind::App,
            id: OWN_APP.to_owned(),
        });
    }
    let label = SettingLabel::new(&format!("{lean} {words} posts"))?;
    Some(NewRecord {
        subject_key: format!("feed:{}:{}", ask.group, scope_code(folder, project)),
        kind: Kind::Preference,
        scope: project.map_or(Scope::Global, |id| Scope::Project(id.to_owned())),
        tier: Tier::Long,
        priority: Priority::Normal,
        authority: Authority::User,
        source: SourceKind::HomeFeed,
        source_id: None,
        engine: None,
        confidence: 1.0,
        as_of_ms: now_ms,
        source_rev: None,
        text: RecordText::Setting(label),
        payload: serde_json::json!({
            "slot": "feed",
            "lean": ask.lean,
            "group": ask.group,
            "folder": folder,
            "project": project,
            "about": ask.about.as_deref().filter(|a| !a.is_empty()),
        }),
        expires_ms: None,
        entities,
    })
}

/// A kept record as a lean, `None` when it isn't one.
#[must_use]
pub fn lean_of(record: &Record) -> Option<FeedLeanInfo> {
    if !record.subject_key.starts_with("feed:") {
        return None;
    }
    let text = |name: &str| {
        record
            .payload
            .get(name)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    Some(FeedLeanInfo {
        key: record.subject_key.clone(),
        lean: text("lean").filter(|l| l == "more" || l == "less")?,
        group: text("group")?,
        folder: text("folder"),
        project: text("project"),
        about: text("about"),
        at_ms: record.as_of_ms,
    })
}

/// Keeps `ask` through the gate with `writer`; `Ok(None)` when the gate
/// refused it (a key or password in a name, too long).
///
/// # Errors
///
/// Not a lean; storage failures.
pub fn prefer_in(
    storage: &Storage,
    writer: &KindWriter,
    ask: &FeedLeanAsk,
    now_ms: i64,
) -> AppResult<Option<FeedLeanInfo>> {
    let record = record_of(ask, now_ms).ok_or_else(|| AppError::Validation {
        subject: "feed",
        message: "not_a_lean".to_owned(),
    })?;
    let store = storage.memory();
    // The gate's refusals (a key or password in a name, too long, not a
    // setting) keep nothing and say so with `None`.
    if let Err(error) = store.remember(writer, &RedactionSecrets, &record, now_ms) {
        let kept_nothing = matches!(
            error,
            MemoryError::Refused(Refusal::Secret | Refusal::TextBounds | Refusal::PreferenceShape)
        );
        return if kept_nothing {
            Ok(None)
        } else {
            Err(memory_error(error))
        };
    }
    Ok(store
        .current(&record.subject_key)
        .map_err(memory_error)?
        .as_ref()
        .and_then(lean_of))
}

/// Every lean kept, newest first.
///
/// # Errors
///
/// Storage failures.
pub fn leans_in(storage: &Storage) -> AppResult<Vec<FeedLeanInfo>> {
    let mut leans: Vec<FeedLeanInfo> = storage
        .memory()
        .current_of_kind(Kind::Preference, None, READ)
        .map_err(memory_error)?
        .iter()
        .filter_map(lean_of)
        .collect();
    leans.sort_by_key(|lean| std::cmp::Reverse(lean.at_ms));
    Ok(leans)
}

/// More or Less like this, from a post's gesture bar.
///
/// # Errors
///
/// Not a lean; the store's writer isn't this process's; storage.
#[tauri::command]
pub async fn feed_lean(
    state: tauri::State<'_, Arc<AppState>>,
    ask: FeedLeanAsk,
) -> AppResult<Option<FeedLeanInfo>> {
    crate::ipc::blocking(state.inner(), "feed", move |state| {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let now = juggler_core::model::Timestamp::now().millis();
        state
            .memory_writers
            .with_memory(|writer| prefer_in(&storage, writer, &ask, now))
            .ok_or_else(|| AppError::Validation {
                subject: "memory",
                message: "unavailable".to_owned(),
            })?
    })
    .await
}

/// The leans kept (the feed ranks and folds by them).
///
/// # Errors
///
/// Storage failures.
#[tauri::command]
pub async fn feed_leans(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<FeedLeanInfo>> {
    crate::ipc::blocking(state.inner(), "feed", move |state| {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        leans_in(&storage)
    })
    .await
}

/// Undo: forgets a lean, every version (ADR-0037 §1.5).
///
/// # Errors
///
/// Not a lean; the store's refusal; storage.
#[tauri::command]
pub async fn feed_unlean(state: tauri::State<'_, Arc<AppState>>, key: String) -> AppResult<()> {
    if !key.starts_with("feed:") {
        return Err(AppError::Validation {
            subject: "feed",
            message: "not_a_lean".to_owned(),
        });
    }
    crate::ipc::blocking(state.inner(), "feed", move |state| {
        crate::features::ai::memory::forget(state, &key, &Actor::User).map(|_| ())
    })
    .await
}

#[cfg(test)]
mod tests {
    use juggler_core::storage::Storage;
    use juggler_core::storage::memory::{Kind, Writers};

    use super::{FeedLeanAsk, leans_in, prefer_in, record_of};

    const T0: i64 = 1_800_000_000_000;

    fn ask(lean: &str, group: &str, folder: Option<&str>, project: Option<&str>) -> FeedLeanAsk {
        FeedLeanAsk {
            lean: lean.to_owned(),
            group: group.to_owned(),
            folder: folder.map(str::to_owned),
            project: project.map(str::to_owned),
            about: project.map(|_| "web-app".to_owned()),
        }
    }

    /// AJ-E3713 R3/R8: a lean is a `preference` written through the gate
    /// with the memory writer; a new lean on the same scope supersedes the
    /// last (one current record), and the feed reads it back.
    #[test]
    fn a_lean_is_a_preference_written_through_the_gate_and_read_back() {
        let storage = Storage::open_in_memory().unwrap();
        let writers = Writers::for_tests();
        let less = ask("less", "switch", None, Some("proj-web"));
        let kept = prefer_in(&storage, &writers.memory, &less, T0)
            .unwrap()
            .expect("kept");
        assert_eq!(kept.lean, "less");
        assert_eq!(kept.group, "switch");
        assert_eq!(kept.project.as_deref(), Some("proj-web"));
        assert_eq!(kept.about.as_deref(), Some("web-app"));
        let record = storage.memory().current(&kept.key).unwrap().unwrap();
        assert_eq!(record.kind, Kind::Preference);
        assert_eq!(record.text, "Fewer switch posts");
        // The same scope again, the other way: one current record, version 2.
        let more = ask("more", "switch", None, Some("proj-web"));
        let again = prefer_in(&storage, &writers.memory, &more, T0 + 1)
            .unwrap()
            .unwrap();
        assert_eq!(again.key, kept.key);
        assert_eq!(storage.memory().history(&kept.key).unwrap().len(), 2);
        let leans = leans_in(&storage).unwrap();
        assert_eq!(leans.len(), 1);
        assert_eq!(leans[0].lean, "more");
        // Another scope is its own record.
        prefer_in(
            &storage,
            &writers.memory,
            &ask("less", "usage", Some("acc-work"), None),
            T0 + 2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(leans_in(&storage).unwrap().len(), 2);
    }

    /// The gate's own refusals hold: another module's writer can't keep a
    /// lean (ADR-0037 §1.1: each kind has one writer), and an ask that
    /// isn't a lean is never written.
    #[test]
    fn the_gate_refuses_another_writer_and_an_ask_that_isnt_a_lean() {
        let storage = Storage::open_in_memory().unwrap();
        let writers = Writers::for_tests();
        let less = ask("less", "switch", None, None);
        assert!(prefer_in(&storage, &writers.watches, &less, T0).is_err());
        assert!(leans_in(&storage).unwrap().is_empty());
        assert!(record_of(&ask("hate", "switch", None, None), T0).is_none());
        assert!(record_of(&ask("less", "everything", None, None), T0).is_none());
        // AI Juggler's own posts everywhere name AI Juggler as their App.
        let own = record_of(&less, T0).unwrap();
        assert_eq!(own.entities.len(), 1);
        assert_eq!(own.entities[0].id, super::OWN_APP);
    }
}
