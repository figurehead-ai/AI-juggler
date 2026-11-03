//! AJ-E3102 (E31, D-62): every count on every page from a cache, never
//! from a scan or a transcript, so someone with thousands of sessions sees
//! a page's tab counts at once.
//!
//! - **What's counted.** Every session a list can show — AI Juggler's own
//!   and the ones other apps started — is a [`Row`]: the project, folder,
//!   App, Account, tool, Mac and launch settings it belongs to, and whether
//!   it's open or archived. Each entity ([`EntityKind`] and id) gets its
//!   sessions counted (all, open, ended, archived) and how many different
//!   things of every other kind they're on ([`RelationCountsInfo`]). The
//!   rows are made by the same rules the lists narrow by
//!   (`features::sessions::rows`, `places::app_of`), so a count agrees with
//!   the list it heads.
//! - **Two parts** ([`PartKind`]), each rebuilt on its own: AI Juggler's
//!   sessions (one narrow storage pass) and the conversations other apps
//!   started (the conversation index by project, App and tool,
//!   [`crate::features::conversations::by_relation`]).
//! - **Invalidated by stamp** ([`stamps`]): each part keeps the stamps of
//!   the sources it was read from; the app's change events move them
//!   ([`changed`]), and so do the database's own stamps (triggers, schema
//!   v43), read on a connection of their own that never waits on a writer,
//!   so a write nothing announced still counts, even while the app's
//!   connection is busy. A read with any stamp moved builds that part
//!   again before it answers.
//! - **D-58's cache** ([`cache`]): registered beside the others, cleared
//!   by the user's Clear cache (a page then shows at once and its counts
//!   a moment later), kept only while the cache is on, its size watched.
//!   It lives in memory: a restart rebuilds it from the records in well
//!   under a second (off the main thread), and nothing is written.
//!
//! Budgets: a count from the cache ≤ 5 ms; a cold rebuild of 10 000
//! sessions ≤ 1 s, on the command's worker thread
//! (`tests/relations.rs`).

pub mod build;
pub mod cache;
pub mod commands;
pub mod dto;
pub mod stamps;

#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use juggler_core::storage::{StampReader, StoredStamps};

pub use dto::{EntityKind, EntityRef, KINDS, RelatedIdsInfo, RelationCountsInfo};
pub use stamps::{Change, Snapshot, Source};

use crate::error::{AppError, AppResult};
use crate::features::conversations::by_relation::ByRelation;
use crate::features::sessions::rows::SessionsCountsInfo;
use crate::state::AppState;

/// The most entities one ask may name.
pub const MAX_ASKED: usize = crate::paging::MAX_LIMIT;

/// One session a list can show, by what it belongs to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
    /// For each [`EntityKind`] (by [`EntityKind::index`]), the one it's
    /// in; `None` where it has none (no project; one other apps started
    /// has no launch settings).
    pub of: [Option<String>; KINDS],
    /// Open now.
    pub live: bool,
    /// Archived (AI Juggler's only).
    pub archived: bool,
}

/// What a part counts for one entity.
#[derive(Clone, Debug, Default)]
struct Tally {
    sessions: SessionsCountsInfo,
    /// For each kind, the names (by [`Part::names`] index) its sessions
    /// are on.
    related: [HashSet<u32>; KINDS],
}

/// The two halves of the cache, each read from its own sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    /// The sessions AI Juggler ran.
    Own,
    /// The conversations other apps started.
    Outside,
}

impl PartKind {
    /// Both.
    pub const ALL: [Self; 2] = [Self::Own, Self::Outside];

    /// The sources it's read from: a change to any of them makes it stale.
    #[must_use]
    pub const fn deps(self) -> &'static [Source] {
        match self {
            Self::Own => &[Source::Sessions, Source::Folders, Source::Pools],
            // Which conversations are AI Juggler's (the sessions' links),
            // their projects, Accounts and desktop Apps.
            Self::Outside => &[
                Source::Sessions,
                Source::Folders,
                Source::Projects,
                Source::Conversations,
                Source::Places,
            ],
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Own => 0,
            Self::Outside => 1,
        }
    }
}

/// One part's counts, as its sources stood when they were read.
#[derive(Debug, Default)]
pub struct Part {
    names: Vec<String>,
    ids: HashMap<String, u32>,
    tallies: HashMap<(usize, u32), Tally>,
    events: Snapshot,
    stored: StoredStamps,
    conversations: Option<Arc<ByRelation>>,
}

impl Part {
    /// The counts of `rows`, read when the events stood at `events` and
    /// the database's stamps at `stored`.
    #[must_use]
    pub fn of(rows: impl IntoIterator<Item = Row>, events: Snapshot, stored: StoredStamps) -> Self {
        let mut part = Self {
            events,
            stored,
            ..Self::default()
        };
        for row in rows {
            let ids: [Option<u32>; KINDS] =
                std::array::from_fn(|kind| row.of[kind].as_deref().map(|name| part.intern(name)));
            for (kind, id) in ids.iter().enumerate() {
                let Some(id) = id else { continue };
                let tally = part.tallies.entry((kind, *id)).or_default();
                tally.sessions.all += 1;
                if row.live {
                    tally.sessions.open += 1;
                } else if row.archived {
                    tally.sessions.archived += 1;
                } else {
                    tally.sessions.ended += 1;
                }
                for (other, related) in ids.iter().enumerate() {
                    if let Some(related) = related {
                        tally.related[other].insert(*related);
                    }
                }
            }
        }
        part
    }

    fn intern(&mut self, name: &str) -> u32 {
        if let Some(id) = self.ids.get(name) {
            return *id;
        }
        let id = u32::try_from(self.names.len()).unwrap_or(u32::MAX);
        self.names.push(name.to_owned());
        self.ids.insert(name.to_owned(), id);
        id
    }

    fn tally(&self, kind: EntityKind, id: &str) -> Option<&Tally> {
        let id = self.ids.get(id)?;
        self.tallies.get(&(kind.index(), *id))
    }

    /// Whether its sources still stand where they did: the events', and
    /// the database's committed rows' (`stored`, read now).
    fn fresh(&self, kind: PartKind, events: &Snapshot, stored: &StoredStamps) -> bool {
        let deps = kind.deps();
        !stamps::moved(deps, &self.events, events)
            && (!deps.contains(&Source::Sessions) || stored.sessions == self.stored.sessions)
            && (!deps.contains(&Source::Folders) || stored.folders == self.stored.folders)
            && (!deps.contains(&Source::Projects) || stored.projects == self.stored.projects)
            && (!deps.contains(&Source::Pools) || stored.pools == self.stored.pools)
    }

    /// About how many bytes it keeps in memory (Settings shows it).
    #[must_use]
    pub fn bytes(&self) -> u64 {
        let names: usize = self.names.iter().map(|name| 2 * name.len() + 48).sum();
        let tallies: usize = self
            .tallies
            .values()
            .map(|tally| 96 + tally.related.iter().map(|set| 8 * set.len()).sum::<usize>())
            .sum();
        let conversations = self
            .conversations
            .as_ref()
            .map_or(0, |index| index.facets().len() * 160);
        u64::try_from(names + tallies + conversations).unwrap_or(u64::MAX)
    }
}

/// The relation-count cache (one per app, in [`AppState`]).
#[derive(Debug, Default)]
pub struct Relations {
    stamps: stamps::Stamps,
    parts: Mutex<[Option<Arc<Part>>; 2]>,
    /// Held while a part is built: one build at a time; a read that waited
    /// takes what the other built.
    building: Mutex<()>,
    used: Mutex<Option<SystemTime>>,
    builds: AtomicU64,
    /// The stamps' own connection on the database, opened on first use.
    reader: Mutex<Option<StampReader>>,
}

impl Relations {
    /// How many parts have been built (a cached read builds none).
    #[must_use]
    pub fn builds(&self) -> u64 {
        self.builds.load(Ordering::Acquire)
    }

    /// Where the change stamps stand now.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        self.stamps.snapshot()
    }

    /// Drops every part (Clear cache, the cache turned off, or trimmed);
    /// the bytes it kept.
    pub fn clear(&self) -> u64 {
        let Ok(mut parts) = self.parts.lock() else {
            return 0;
        };
        let bytes = parts.iter().flatten().map(|part| part.bytes()).sum();
        *parts = [None, None];
        bytes
    }

    /// What it keeps now: its bytes and when it was last read, if any.
    #[must_use]
    pub fn kept(&self) -> Option<(u64, SystemTime)> {
        let parts = self.parts.lock().ok()?;
        if parts.iter().all(Option::is_none) {
            return None;
        }
        let bytes = parts.iter().flatten().map(|part| part.bytes()).sum();
        let used = self
            .used
            .lock()
            .ok()
            .and_then(|used| *used)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        Some((bytes, used))
    }

    /// The kept parts, when both are fresh.
    fn fresh(&self, events: &Snapshot, stored: &StoredStamps) -> Option<[Arc<Part>; 2]> {
        let parts = self.parts.lock().ok()?;
        let [Some(own), Some(outside)] = &*parts else {
            return None;
        };
        (own.fresh(PartKind::Own, events, stored)
            && outside.fresh(PartKind::Outside, events, stored))
        .then(|| [Arc::clone(own), Arc::clone(outside)])
    }
}

/// `change` happened: the parts made from what it changed are stale.
pub fn changed(state: &AppState, change: Change) {
    state.relations.stamps.bump(change);
}

/// The database's stamps as committed now. Read on the stamps' own
/// connection, which never waits on a writer (WAL): a count is checked
/// even while another thread holds the app's connection for a long write,
/// and never served unchecked. Without that connection (an in-memory
/// database, or it failed), the app's connection, waiting its turn.
fn stored_now(state: &AppState) -> AppResult<StoredStamps> {
    {
        let mut reader = state
            .relations
            .reader
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        if reader.is_none() {
            *reader = lock_storage(state)?.stamp_reader().ok().flatten();
        }
        match reader.as_ref().map(StampReader::stamps) {
            Some(Ok(stamps)) => return Ok(stamps),
            // Opened again on the next read.
            Some(Err(_)) => *reader = None,
            None => {}
        }
    }
    Ok(lock_storage(state)?.relations().stamps()?)
}

fn lock_storage(
    state: &AppState,
) -> AppResult<std::sync::MutexGuard<'_, juggler_core::storage::Storage>> {
    state.storage.lock().map_err(|_| AppError::StatePoisoned)
}

/// Both parts as they are now: from the cache when fresh, else built
/// (only the stale one), and kept while the cache is on.
fn current(state: &Arc<AppState>) -> AppResult<[Arc<Part>; 2]> {
    let relations = &state.relations;
    let on = state.cache.gate().is_on();
    if on && let Some(parts) = relations.fresh(&relations.snapshot(), &stored_now(state)?) {
        mark_used(relations);
        return Ok(parts);
    }
    let _one = relations
        .building
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let events = relations.snapshot();
    let stored = stored_now(state)?;
    if on && let Some(parts) = relations.fresh(&events, &stored) {
        mark_used(relations);
        return Ok(parts);
    }
    let kept = relations
        .parts
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .clone();
    let mut made: [Option<Arc<Part>>; 2] = [None, None];
    for kind in PartKind::ALL {
        let part = match &kept[kind.index()] {
            Some(part) if on && part.fresh(kind, &events, &stored) => Arc::clone(part),
            _ => {
                relations.builds.fetch_add(1, Ordering::AcqRel);
                Arc::new(build::part(state, kind)?)
            }
        };
        made[kind.index()] = Some(part);
    }
    let [Some(own), Some(outside)] = made else {
        return Err(AppError::StatePoisoned);
    };
    if on {
        *relations
            .parts
            .lock()
            .map_err(|_| AppError::StatePoisoned)? =
            [Some(Arc::clone(&own)), Some(Arc::clone(&outside))];
        mark_used(relations);
    }
    Ok([own, outside])
}

fn mark_used(relations: &Relations) {
    if let Ok(mut used) = relations.used.lock() {
        *used = Some(SystemTime::now());
    }
}

/// `entity`'s counts in `parts`.
fn answer(parts: &[Arc<Part>; 2], entity: &EntityRef) -> RelationCountsInfo {
    let id = if entity.kind == EntityKind::All {
        ""
    } else {
        entity.id.as_str()
    };
    let mut sessions = SessionsCountsInfo::default();
    let mut related: [HashSet<&str>; KINDS] = std::array::from_fn(|_| HashSet::new());
    for part in parts {
        let Some(tally) = part.tally(entity.kind, id) else {
            continue;
        };
        sessions.all += tally.sessions.all;
        sessions.open += tally.sessions.open;
        sessions.ended += tally.sessions.ended;
        sessions.archived += tally.sessions.archived;
        for (kind, ids) in tally.related.iter().enumerate() {
            related[kind].extend(
                ids.iter()
                    .filter_map(|id| part.names.get(*id as usize).map(String::as_str)),
            );
        }
    }
    let n = |kind: EntityKind| related[kind.index()].len() as u64;
    RelationCountsInfo {
        kind: entity.kind,
        id: entity.id.clone(),
        sessions,
        projects: n(EntityKind::Project),
        folders: n(EntityKind::Folder),
        apps: n(EntityKind::App),
        subscriptions: n(EntityKind::Subscription),
        tools: n(EntityKind::Tool),
        machines: n(EntityKind::Machine),
        launch_settings: n(EntityKind::LaunchSettings),
        pools: n(EntityKind::Pool),
    }
}

/// Which `of` `entity`'s sessions are on, in `parts`.
fn related_in(parts: &[Arc<Part>; 2], entity: &EntityRef, of: EntityKind) -> RelatedIdsInfo {
    let id = if entity.kind == EntityKind::All {
        ""
    } else {
        entity.id.as_str()
    };
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for part in parts {
        if let Some(tally) = part.tally(entity.kind, id) {
            ids.extend(
                tally.related[of.index()]
                    .iter()
                    .filter_map(|id| part.names.get(*id as usize).map(String::as_str)),
            );
        }
    }
    let more = ids.len() > MAX_ASKED;
    RelatedIdsInfo {
        kind: entity.kind,
        id: entity.id.clone(),
        of,
        ids: ids.into_iter().take(MAX_ASKED).map(str::to_owned).collect(),
        more,
    }
}

/// Which `of` `entity`'s sessions are on (a page's Pools or Apps tab),
/// from the cache.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn related(
    state: &Arc<AppState>,
    entity: &EntityRef,
    of: EntityKind,
) -> AppResult<RelatedIdsInfo> {
    Ok(related_in(&current(state)?, entity, of))
}

/// `entity`'s counts, from the cache (built first when stale or cleared).
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn counts(state: &Arc<AppState>, entity: &EntityRef) -> AppResult<RelationCountsInfo> {
    Ok(answer(&current(state)?, entity))
}

/// Each of `entities`' counts (at most [`MAX_ASKED`]), in order, from one
/// look at the cache.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn counts_many(
    state: &Arc<AppState>,
    entities: &[EntityRef],
) -> AppResult<Vec<RelationCountsInfo>> {
    let parts = current(state)?;
    Ok(entities
        .iter()
        .take(MAX_ASKED)
        .map(|entity| answer(&parts, entity))
        .collect())
}

/// The conversations other apps started, indexed by project, App, tool
/// and folder, as the counts were made from them (AJ-E3103's lists
/// narrow by it).
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn conversation_index(state: &Arc<AppState>) -> AppResult<Arc<ByRelation>> {
    let [_, outside] = current(state)?;
    Ok(outside.conversations.clone().unwrap_or_default())
}

/// Builds the counts once the app is up, off the main thread, so the
/// first page opened has them at once (the cache is registered with the
/// others, `features::cache::register_defaults`).
pub fn start(state: &Arc<AppState>) {
    let weak = Arc::downgrade(state);
    crate::ticks::every_while("relation-counts", WARM_AFTER, move || {
        if let Some(state) = weak.upgrade() {
            let _ = current(&state);
        }
        false
    });
}

/// How long after the start the counts are first built (the first pass
/// over the tools' files has begun by then).
const WARM_AFTER: std::time::Duration = std::time::Duration::from_secs(3);
