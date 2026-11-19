//! The watches over IPC, for the Watches view (AJ-E3816) and Chat's
//! confirmed drafts (AJ-E3815): three reads (the list, a target's
//! catalogue, a draft's preview in plain words) and the writes the user's
//! own click makes (create, edit, pause or resume, remove). Nothing else
//! writes a watch: the engine only reads them.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::catalogue::{self, CatalogueEntry};
use super::conditions::Registry;
use super::echo::{self, LocalClock, Names};
use super::engine::EndReason;
use super::model::{Condition, LifetimeChoice, Matching, Watch, WatchError, WatchId, WatchTarget};
use super::runtime::{self, Facts};
use super::store::{Origin, StoreError, Stored, WatchStore};
use super::words::{self, Lang};
use crate::error::{AppError, AppResult};
use crate::features::ai::memory::RedactionSecrets;
use crate::paging::{Page, PageRequest, SortKey};
use crate::state::AppState;

/// A watch as the user makes it: the lifetime as chosen, not yet fixed.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WatchDraftInfo {
    /// 1–10 targets.
    pub targets: Vec<WatchTarget>,
    /// Each, or all.
    #[serde(default)]
    pub matching: Matching,
    /// 1–4 conditions.
    pub conditions: Vec<Condition>,
    /// The lifetime as chosen ("today", unsaid, …).
    pub lifetime: LifetimeChoice,
    /// "Even during quiet hours", off unless the user turns it on.
    #[serde(default)]
    pub even_in_quiet_hours: bool,
    /// The Chat turn whose draft the user confirmed; none from the view.
    #[serde(default)]
    #[ts(optional)]
    pub chat_turn: Option<String>,
}

/// Where a watch stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum WatchStateInfo {
    /// Waiting for what it watches for.
    Active,
    /// Paused by the user.
    Paused,
    /// Ended (see its note).
    Ended,
}

/// A watch for the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WatchInfo {
    /// The typed watch.
    pub watch: Watch,
    /// It in plain words, from the typed watch.
    pub words: String,
    /// Where it stands.
    pub state: WatchStateInfo,
    /// Why it ended, when it did.
    pub ended: Option<EndReason>,
    /// Why it ended, in plain words (never a silent failure).
    pub ended_words: Option<String>,
    /// How many times it told the user.
    #[ts(type = "number")]
    pub told: u64,
    /// Its version (an edit, a pause, a resume: each one more).
    #[ts(type = "number")]
    pub version: i64,
}

/// A draft checked and in plain words, before anything is written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WatchPreviewInfo {
    /// The watch it would make (its lifetime fixed).
    pub watch: Watch,
    /// It in plain words: what the user confirms is what runs.
    pub words: String,
    /// Why it can't be made, as a code; `None` when it can.
    pub problem: Option<String>,
}

/// Names read live, before the store's lock is held: a session's as its
/// card says it (the agent's words: the echo quotes them as data), the
/// others from their records.
#[derive(Debug, Default)]
struct NameBook(std::collections::HashMap<WatchTarget, String>);

impl Names for NameBook {
    fn target(&self, target: &WatchTarget) -> Option<String> {
        self.0.get(target).cloned()
    }
    fn agent(&self, _id: &str) -> Option<String> {
        None
    }
}

fn names_for<'t>(
    state: &Arc<AppState>,
    targets: impl IntoIterator<Item = &'t WatchTarget>,
) -> NameBook {
    use juggler_core::model::{AccountId, ProfileId, ProjectId, SessionId};
    let targets: std::collections::BTreeSet<&WatchTarget> = targets.into_iter().collect();
    let mut book = NameBook::default();
    let Ok(storage) = state.storage.lock() else {
        return book;
    };
    let sessions: Vec<SessionId> = targets
        .iter()
        .filter_map(|t| match t {
            WatchTarget::Session(id) => Some(SessionId::new(id.as_str())),
            _ => None,
        })
        .collect();
    let records = storage.sessions().get_many(&sessions).unwrap_or_default();
    for target in &targets {
        let name = match target {
            WatchTarget::App(id) => storage
                .profiles()
                .get(&ProfileId::new(id.as_str()))
                .ok()
                .map(|p| p.name),
            WatchTarget::Project(id) => storage
                .projects()
                .get(&ProjectId::new(id.as_str()))
                .ok()
                .map(|p| p.name),
            WatchTarget::Pool(id) => storage.pools().get(id).ok().map(|p| p.name),
            WatchTarget::Account(id) => storage
                .accounts()
                .get(&AccountId::new(id.as_str()))
                .ok()
                .map(|a| a.label),
            WatchTarget::Session(_) | WatchTarget::Any => None,
        };
        if let Some(name) = name {
            book.0.insert((*target).clone(), name);
        }
    }
    drop(storage);
    if !records.is_empty() {
        for row in crate::features::sessions::list::instances_of(state, records).unwrap_or_default()
        {
            let name = crate::features::pools::cards::name_of(&row);
            book.0.insert(WatchTarget::Session(row.id.clone()), name);
        }
    }
    book
}

fn store_error(error: StoreError) -> AppError {
    match error {
        StoreError::Unavailable => AppError::Validation {
            subject: "watch",
            message: "unavailable".to_owned(),
        },
        StoreError::NotFound => AppError::NotFound {
            entity: "watch",
            id: String::new(),
        },
        StoreError::Exists => AppError::Validation {
            subject: "watch",
            message: "exists".to_owned(),
        },
        StoreError::Invalid(error) => invalid(&error),
        StoreError::Memory(juggler_core::storage::memory::MemoryError::Core(core)) => {
            AppError::from(core)
        }
        StoreError::Memory(other) => AppError::Validation {
            subject: "watch",
            message: other.to_string(),
        },
    }
}

fn invalid(error: &WatchError) -> AppError {
    AppError::Validation {
        subject: "watch",
        message: error.code(),
    }
}

fn clock(state: &AppState) -> LocalClock {
    let now = state.notifier.clock();
    LocalClock {
        now_ms: now.now,
        offset_hours: now.utc_offset_hours.unwrap_or(0),
    }
}

/// The typed watch a draft makes, and why it can't be made, if so: the
/// closed set, the bounds, and each condition offered for its targets'
/// Apps (D-127).
fn typed(
    registry: &Registry,
    facts: &Facts<'_>,
    draft: &WatchDraftInfo,
    id: WatchId,
    created_ms: i64,
    now: LocalClock,
) -> (Watch, Option<WatchError>) {
    let watch = Watch {
        id,
        targets: draft.targets.clone(),
        matching: draft.matching,
        lifetime: draft
            .lifetime
            .fix(&draft.targets, &draft.conditions, now.next_midnight()),
        conditions: draft.conditions.clone(),
        paused: false,
        even_in_quiet_hours: draft.even_in_quiet_hours,
        created_ms,
    };
    let problem = watch.validate(registry, now.now_ms).err().or_else(|| {
        let observed: Vec<_> = watch.targets.iter().map(|t| facts.observes(t)).collect();
        catalogue::check_offered(registry, &watch, &observed).err()
    });
    (watch, problem)
}

fn info(
    store: &WatchStore<'_>,
    names: &dyn Names,
    stored: Stored,
    lang: Lang,
    now: LocalClock,
) -> AppResult<WatchInfo> {
    let registry = Registry::shipped();
    let ended = store.ended(&stored.watch.id).map_err(store_error)?;
    let told = store.fired_count(&stored.watch.id).map_err(store_error)?;
    let state = match (&ended, stored.watch.paused) {
        (Some(_), _) => WatchStateInfo::Ended,
        (None, true) => WatchStateInfo::Paused,
        (None, false) => WatchStateInfo::Active,
    };
    Ok(WatchInfo {
        words: echo::echo(&registry, &stored.watch, names, lang, now),
        state,
        ended: ended.map(|(reason, _)| reason),
        ended_words: ended.map(|(reason, _)| words::ended(reason, lang).to_owned()),
        told,
        version: stored.version,
        watch: stored.watch,
    })
}

/// One page of the watches the user hasn't removed, newest change first,
/// in plain words (ADR-0036 §2.6: never loaded all at once).
///
/// # Errors
///
/// The store isn't there yet (`unavailable`); a cursor this app didn't
/// make; storage failures.
pub fn watches_list_impl(
    state: &Arc<AppState>,
    request: &PageRequest,
    lang: Lang,
) -> AppResult<Page<WatchInfo>> {
    let registry = Registry::shipped();
    let now = clock(state);
    let after = request
        .start_after()?
        .map(|key| (key.ranks.first().map_or(0, |r| -r), key.id));
    let limit = u32::try_from(request.limit()).unwrap_or(50);
    let (listed, next) = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        WatchStore::new(&storage, &registry)
            .page(after.as_ref(), limit)
            .map_err(store_error)?
    };
    let names = names_for(state, listed.iter().flat_map(|s| s.watch.targets.iter()));
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let store = WatchStore::new(&storage, &registry);
    let items = listed
        .into_iter()
        .map(|stored| info(&store, &names, stored, lang, now))
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Page {
        items,
        next_cursor: next.map(|(at, id)| SortKey::new([-at], id).cursor()),
        total: None,
    })
}

/// The conditions on offer for a target: only those its Apps can report
/// (D-127), with their labels and parameters.
///
/// # Errors
///
/// Storage failures.
pub fn watches_catalogue_impl(
    state: &Arc<AppState>,
    target: &WatchTarget,
    lang: Lang,
) -> AppResult<Vec<CatalogueEntry>> {
    let registry = Registry::shipped();
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let observed = Facts::new(&storage, state).observes(target);
    Ok(catalogue::catalogue(
        &registry,
        target.kind(),
        observed,
        lang,
    ))
}

/// A draft checked against the closed set and its targets' Apps, and put
/// in plain words; nothing is written.
///
/// # Errors
///
/// Storage failures.
pub fn watches_preview_impl(
    state: &Arc<AppState>,
    draft: &WatchDraftInfo,
    lang: Lang,
) -> AppResult<WatchPreviewInfo> {
    let registry = Registry::shipped();
    let now = clock(state);
    let names = names_for(state, &draft.targets);
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let facts = Facts::new(&storage, state);
    let (watch, problem) = typed(
        &registry,
        &facts,
        draft,
        WatchId::fresh(now.now_ms),
        now.now_ms,
        now,
    );
    Ok(WatchPreviewInfo {
        words: echo::echo(&registry, &watch, &names, lang, now),
        watch,
        problem: problem.map(|p| p.code()),
    })
}

fn origin(draft: &WatchDraftInfo) -> Origin {
    draft
        .chat_turn
        .clone()
        .map_or(Origin::WatchesView, Origin::ChatTurn)
}

/// One write through the store, then the engine told.
fn write(
    state: &Arc<AppState>,
    lang: Lang,
    act: impl FnOnce(
        &WatchStore<'_>,
        &Facts<'_>,
        &juggler_core::storage::memory::KindWriter,
        LocalClock,
    ) -> Result<Stored, StoreError>,
) -> AppResult<WatchInfo> {
    let registry = Registry::shipped();
    let now = clock(state);
    let writer = state
        .watches
        .writer(state)
        .ok_or_else(|| AppError::Validation {
            subject: "watch",
            message: "unavailable".to_owned(),
        })?;
    let stored = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let store = WatchStore::new(&storage, &registry);
        let facts = Facts::new(&storage, state);
        act(&store, &facts, writer, now).map_err(store_error)?
    };
    let names = names_for(state, &stored.watch.targets);
    let info = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        info(
            &WatchStore::new(&storage, &registry),
            &names,
            stored.clone(),
            lang,
            now,
        )?
    };
    runtime::with_engine(state, now.now_ms, |engine| {
        if stored.removed {
            engine.remove(&stored.watch.id);
        } else {
            engine.add(stored.watch.clone(), now.now_ms);
        }
    });
    runtime::changed(state);
    Ok(info)
}

/// Creates the watch the user confirmed.
///
/// # Errors
///
/// Refused (a code naming why); the store isn't there yet; storage.
pub fn watches_create_impl(
    state: &Arc<AppState>,
    draft: &WatchDraftInfo,
    lang: Lang,
) -> AppResult<WatchInfo> {
    let registry = Registry::shipped();
    write(state, lang, |store, facts, writer, now| {
        let (watch, problem) = typed(
            &registry,
            facts,
            draft,
            WatchId::fresh(now.now_ms),
            now.now_ms,
            now,
        );
        if let Some(problem) = problem {
            return Err(StoreError::Invalid(problem));
        }
        store.create(
            writer,
            &RedactionSecrets,
            &watch,
            &origin(draft),
            now.now_ms,
        )
    })
}

/// Changes a watch, by id: a new version.
///
/// # Errors
///
/// As [`watches_create_impl`]; no such watch.
pub fn watches_update_impl(
    state: &Arc<AppState>,
    id: &WatchId,
    draft: &WatchDraftInfo,
    lang: Lang,
) -> AppResult<WatchInfo> {
    let registry = Registry::shipped();
    write(state, lang, |store, facts, writer, now| {
        let (mut watch, problem) = typed(&registry, facts, draft, id.clone(), now.now_ms, now);
        if let Some(problem) = problem {
            return Err(StoreError::Invalid(problem));
        }
        watch.paused = store.get(id)?.is_some_and(|s| s.watch.paused);
        store.update(
            writer,
            &RedactionSecrets,
            &watch,
            &origin(draft),
            now.now_ms,
        )
    })
}

/// Pauses or resumes a watch, by id.
///
/// # Errors
///
/// No such watch; the store isn't there yet; storage.
pub fn watches_pause_impl(
    state: &Arc<AppState>,
    id: &WatchId,
    paused: bool,
    lang: Lang,
) -> AppResult<WatchInfo> {
    write(state, lang, |store, _, writer, now| {
        store.set_paused(
            writer,
            &RedactionSecrets,
            id,
            paused,
            &Origin::WatchesView,
            now.now_ms,
        )
    })
}

/// Removes a watch, by id.
///
/// # Errors
///
/// No such watch; the store isn't there yet; storage.
pub fn watches_remove_impl(
    state: &Arc<AppState>,
    id: &WatchId,
    lang: Lang,
) -> AppResult<WatchInfo> {
    write(state, lang, |store, _, writer, now| {
        store.remove(
            writer,
            &RedactionSecrets,
            id,
            &Origin::WatchesView,
            now.now_ms,
        )
    })
}

/// Every watch, in plain words (see [`watches_list_impl`]).
///
/// # Errors
///
/// As [`watches_list_impl`].
#[tauri::command]
pub async fn watches_list(
    state: tauri::State<'_, Arc<AppState>>,
    request: PageRequest,
    lang: Option<Lang>,
) -> AppResult<Page<WatchInfo>> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_list_impl(state, &request, lang.unwrap_or_default())
    })
    .await
}

/// The conditions on offer for a target (see [`watches_catalogue_impl`]).
///
/// # Errors
///
/// As [`watches_catalogue_impl`].
#[tauri::command]
pub async fn watches_catalogue(
    state: tauri::State<'_, Arc<AppState>>,
    target: WatchTarget,
    lang: Option<Lang>,
) -> AppResult<Vec<CatalogueEntry>> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_catalogue_impl(state, &target, lang.unwrap_or_default())
    })
    .await
}

/// A draft in plain words (see [`watches_preview_impl`]).
///
/// # Errors
///
/// As [`watches_preview_impl`].
#[tauri::command]
pub async fn watches_preview(
    state: tauri::State<'_, Arc<AppState>>,
    draft: WatchDraftInfo,
    lang: Option<Lang>,
) -> AppResult<WatchPreviewInfo> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_preview_impl(state, &draft, lang.unwrap_or_default())
    })
    .await
}

/// Creates a watch (see [`watches_create_impl`]).
///
/// # Errors
///
/// As [`watches_create_impl`].
#[tauri::command]
pub async fn watches_create(
    state: tauri::State<'_, Arc<AppState>>,
    draft: WatchDraftInfo,
    lang: Option<Lang>,
) -> AppResult<WatchInfo> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_create_impl(state, &draft, lang.unwrap_or_default())
    })
    .await
}

/// Changes a watch (see [`watches_update_impl`]).
///
/// # Errors
///
/// As [`watches_update_impl`].
#[tauri::command]
pub async fn watches_update(
    state: tauri::State<'_, Arc<AppState>>,
    id: WatchId,
    draft: WatchDraftInfo,
    lang: Option<Lang>,
) -> AppResult<WatchInfo> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_update_impl(state, &id, &draft, lang.unwrap_or_default())
    })
    .await
}

/// Pauses or resumes a watch (see [`watches_pause_impl`]).
///
/// # Errors
///
/// As [`watches_pause_impl`].
#[tauri::command]
pub async fn watches_pause(
    state: tauri::State<'_, Arc<AppState>>,
    id: WatchId,
    paused: bool,
    lang: Option<Lang>,
) -> AppResult<WatchInfo> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_pause_impl(state, &id, paused, lang.unwrap_or_default())
    })
    .await
}

/// Removes a watch (see [`watches_remove_impl`]).
///
/// # Errors
///
/// As [`watches_remove_impl`].
#[tauri::command]
pub async fn watches_remove(
    state: tauri::State<'_, Arc<AppState>>,
    id: WatchId,
    lang: Option<Lang>,
) -> AppResult<WatchInfo> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_remove_impl(state, &id, lang.unwrap_or_default())
    })
    .await
}
