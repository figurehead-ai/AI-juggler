//! The watches over IPC, for the Watches view (AJ-E3816) and Chat's
//! confirmed drafts (AJ-E3815): three reads (the list, a target's
//! catalogue, a draft's preview in plain words) and the writes the user's
//! own click makes (create, edit, pause or resume, remove). Nothing else
//! writes a watch: the engine only reads them.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::catalogue::{self, CatalogueEntry};
use super::conditions::Registry;
use super::echo::{self, EchoParts, LocalClock, Names};
use super::engine::EndReason;
use super::model::{
    Condition, LifetimeChoice, MAX_ACTIVE, Matching, Watch, WatchError, WatchId, WatchTarget,
};
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
    /// The same words in their parts (`echo.joined() == words`), so the
    /// view shows them without splitting them.
    pub echo: EchoParts,
    /// Where it stands.
    pub state: WatchStateInfo,
    /// Why it ended, when it did.
    pub ended: Option<EndReason>,
    /// Why it ended, in plain words (never a silent failure).
    pub ended_words: Option<String>,
    /// When it ended, epoch ms.
    #[ts(type = "number | null")]
    pub ended_ms: Option<i64>,
    /// How many times it told the user.
    #[ts(type = "number")]
    pub told: u64,
    /// The last time it told the user, epoch ms.
    #[ts(type = "number | null")]
    pub last_told_ms: Option<i64>,
    /// What it told then: the condition's code (`finishes`).
    pub last_told_code: Option<String>,
    /// What it told then, in plain words ("finished").
    pub last_told_words: Option<String>,
    /// Its version (an edit, a pause, a resume: each one more).
    #[ts(type = "number")]
    pub version: i64,
    /// When this version was written ("Paused · since 10:12").
    #[ts(type = "number")]
    pub changed_ms: i64,
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
    /// The same words in their parts (`echo.joined() == words`).
    pub echo: EchoParts,
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
    let last = store.last_fired(&stored.watch.id).map_err(store_error)?;
    let state = match (&ended, stored.watch.paused) {
        (Some(_), _) => WatchStateInfo::Ended,
        (None, true) => WatchStateInfo::Paused,
        (None, false) => WatchStateInfo::Active,
    };
    let echo = echo::echo_parts(&registry, &stored.watch, names, lang, now);
    let last_told_words = last.as_ref().and_then(|(_, code)| {
        registry.get(code).map(|rule| match lang {
            Lang::En => rule.words().told_en.to_owned(),
            Lang::De => rule.words().told_de.to_owned(),
        })
    });
    Ok(WatchInfo {
        words: echo.joined(),
        echo,
        state,
        ended: ended.map(|(reason, _)| reason),
        ended_words: ended.map(|(reason, _)| words::ended(reason, lang).to_owned()),
        ended_ms: ended.map(|(_, at)| at),
        told,
        last_told_ms: last.as_ref().map(|(at, _)| *at),
        last_told_code: last.map(|(_, code)| code),
        last_told_words,
        version: stored.version,
        changed_ms: stored.updated_ms,
        watch: stored.watch,
    })
}

/// How long an ended watch stays on the list: 30 days, as the notification
/// centre's Earlier.
pub const ENDED_KEPT_MS: i64 = 30 * 24 * 3_600_000;

/// Which watches a list holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum WatchListWhich {
    /// Every watch the user hasn't removed.
    #[default]
    All,
    /// Those watching or paused.
    Live,
    /// Those that ended (kept 30 days).
    Ended,
}

impl WatchListWhich {
    fn holds(self, info: &WatchInfo, now_ms: i64) -> bool {
        let ended = info.state == WatchStateInfo::Ended;
        let kept = info
            .ended_ms
            .is_none_or(|at| now_ms.saturating_sub(at) <= ENDED_KEPT_MS);
        match self {
            Self::All => kept,
            Self::Live => !ended,
            Self::Ended => ended && kept,
        }
    }
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
    watches_list_of(state, request, WatchListWhich::All, lang)
}

/// One page of `which` watches, newest change first (see
/// [`watches_list_impl`]). The store's pages are read until this one is
/// full, at most 20 of them.
///
/// # Errors
///
/// As [`watches_list_impl`].
pub fn watches_list_of(
    state: &Arc<AppState>,
    request: &PageRequest,
    which: WatchListWhich,
    lang: Lang,
) -> AppResult<Page<WatchInfo>> {
    let registry = Registry::shipped();
    let now = clock(state);
    let mut after = request
        .start_after()?
        .map(|key| (key.ranks.first().map_or(0, |r| -r), key.id));
    let limit = request.limit().max(1);
    let fetch = u32::try_from(limit).unwrap_or(50);
    let mut items = Vec::new();
    let mut next = None;
    for _ in 0..20 {
        let (listed, more) = {
            let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
            WatchStore::new(&storage, &registry)
                .page(after.as_ref(), fetch)
                .map_err(store_error)?
        };
        let names = names_for(state, listed.iter().flat_map(|s| s.watch.targets.iter()));
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let store = WatchStore::new(&storage, &registry);
        for stored in listed {
            let key = stored.key.clone();
            let item = info(&store, &names, stored, lang, now)?;
            if which.holds(&item, now.now_ms) {
                items.push(item);
            }
            after = Some(key);
            if items.len() >= limit {
                break;
            }
        }
        if items.len() >= limit {
            next.clone_from(&after);
            break;
        }
        match more {
            Some(key) => after = Some(key),
            None => break,
        }
    }
    Ok(Page {
        items,
        next_cursor: next.map(|(at, id)| SortKey::new([-at], id).cursor()),
        total: None,
    })
}

/// The Watches view's counts, read once: what is watching, paused and
/// ended, what told the user today or never, and the limit.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WatchSummaryInfo {
    /// Watching now (not paused, not ended).
    pub watching: u32,
    /// Paused.
    pub paused: u32,
    /// Ended in the last 30 days.
    pub ended: u32,
    /// Watching or paused that told the user since local midnight.
    pub told_today: u32,
    /// Watching or paused that never told the user.
    pub never_told: u32,
    /// Watching or paused with "Even during quiet hours" on.
    pub even_in_quiet_hours: u32,
    /// The most kept at once (watching and paused count).
    pub limit: u32,
}

/// The counts (see [`WatchSummaryInfo`]).
///
/// # Errors
///
/// The store isn't there yet (`unavailable`); storage failures.
pub fn watches_summary_impl(state: &Arc<AppState>) -> AppResult<WatchSummaryInfo> {
    let registry = Registry::shipped();
    let now = clock(state);
    let midnight = now.next_midnight() - 86_400_000;
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let store = WatchStore::new(&storage, &registry);
    let mut out = WatchSummaryInfo {
        limit: u32::try_from(MAX_ACTIVE).unwrap_or(50),
        ..WatchSummaryInfo::default()
    };
    for stored in store.list().map_err(store_error)? {
        let id = &stored.watch.id;
        if let Some((_, at)) = store.ended(id).map_err(store_error)? {
            if now.now_ms.saturating_sub(at) <= ENDED_KEPT_MS {
                out.ended += 1;
            }
            continue;
        }
        if stored.watch.paused {
            out.paused += 1;
        } else {
            out.watching += 1;
        }
        if stored.watch.even_in_quiet_hours {
            out.even_in_quiet_hours += 1;
        }
        match store.last_fired(id).map_err(store_error)? {
            Some((at, _)) if at >= midnight => out.told_today += 1,
            Some(_) => {}
            None => out.never_told += 1,
        }
    }
    Ok(out)
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
    let echo = echo::echo_parts(&registry, &watch, &names, lang, now);
    Ok(WatchPreviewInfo {
        words: echo.joined(),
        echo,
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
    let ended = info.state == WatchStateInfo::Ended;
    runtime::with_engine(state, now.now_ms, |engine| {
        if stored.removed || ended {
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

/// Brings back a watch just removed, by id (the view's Undo).
///
/// # Errors
///
/// Not a removed watch; at the limit; the store isn't there yet; storage.
pub fn watches_restore_impl(
    state: &Arc<AppState>,
    id: &WatchId,
    lang: Lang,
) -> AppResult<WatchInfo> {
    write(state, lang, |store, _, writer, now| {
        store.restore(
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
    which: Option<WatchListWhich>,
    lang: Option<Lang>,
) -> AppResult<Page<WatchInfo>> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_list_of(
            state,
            &request,
            which.unwrap_or_default(),
            lang.unwrap_or_default(),
        )
    })
    .await
}

/// The Watches view's counts (see [`watches_summary_impl`]).
///
/// # Errors
///
/// As [`watches_summary_impl`].
#[tauri::command]
pub async fn watches_summary(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<WatchSummaryInfo> {
    crate::ipc::blocking(state.inner(), "watches", watches_summary_impl).await
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

/// Brings back a watch just removed (see [`watches_restore_impl`]).
///
/// # Errors
///
/// As [`watches_restore_impl`].
#[tauri::command]
pub async fn watches_restore(
    state: tauri::State<'_, Arc<AppState>>,
    id: WatchId,
    lang: Option<Lang>,
) -> AppResult<WatchInfo> {
    crate::ipc::blocking(state.inner(), "watches", move |state| {
        watches_restore_impl(state, &id, lang.unwrap_or_default())
    })
    .await
}
