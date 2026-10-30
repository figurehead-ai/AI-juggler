//! ADR-0023 (D-58): one local cache system. AI Juggler runs on the
//! user's Mac, and what it keeps there to be quick stays there.
//!
//! - **A registry of caches** behind [`LocalCache`]: each says what it is
//!   in plain words, where it lives, how big it is, and whether it can be
//!   trimmed an entry at a time or only whole ([`Trim`]). A cache that is
//!   a folder of files registers in a few lines ([`folder::FolderCache`]).
//!   Registered today: the subagent lists, the app icons and the search
//!   index ([`start`]).
//! - **Every entry can be rebuilt** from its source, so none is a record.
//!   Each carries its source's identity, its format and a print of the
//!   settings that shaped it ([`stamp`]); any difference drops it and the
//!   source is read again. Writes are whole or nothing, owner-only, with
//!   secrets redacted before they are written (D-14).
//! - **Sized by the machine** ([`budget`]): a share of the free disk,
//!   clamped; memory caches by the Mac's memory; on a nearly full disk it
//!   trims to its floor and writes nothing new.
//! - **Watched in the background** ([`Manager::pass`], every
//!   [`TICK`]): sizes are counted as entries are written, folders walked
//!   rarely (and at most [`MAX_ITEMS`] files each), at most
//!   [`MAX_REMOVALS`] files removed per pass, the least recently used
//!   first, across caches. A whole-only cache (the search index, which is
//!   slow to build) is never trimmed for the budget; only a nearly full
//!   disk removes it.
//! - **The user's** (Settings › Advanced › Local cache): turned off, it
//!   writes nothing and its files are removed; Clear cache removes every
//!   cache's files now. The search index keeps its own switch.

pub mod budget;
pub mod commands;
pub mod dto;
pub mod folder;
pub mod stamp;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

use juggler_platform::room::DiskSpace;

pub use budget::{Budget, Limit, Tier};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use dto::{CacheInfo, CacheRowInfo};

/// Where the switch is kept ("off", or empty for on).
pub const SETTING_ON: &str = "local_cache";
/// Where the limit is kept (megabytes, or empty for automatic).
pub const SETTING_LIMIT: &str = "local_cache_limit_mb";
/// How often the background pass runs.
pub const TICK: Duration = Duration::from_secs(60);
/// Folders are walked at least this often…
pub const WALK_EVERY: Duration = Duration::from_mins(30);
/// …and this soon after something was written.
pub const WALK_AFTER_WRITES: Duration = Duration::from_mins(5);
/// The most files one walk looks at in one cache.
pub const MAX_ITEMS: usize = 20_000;
/// The most files one pass removes.
pub const MAX_REMOVALS: usize = 2_000;

/// How a cache may be made smaller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trim {
    /// An entry at a time, the least recently used first.
    Partly,
    /// Only whole (slow to rebuild): never for the budget, only on a
    /// nearly full disk or when the user clears it.
    WholeOnly,
}

/// What a cache is, in the words Settings shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct About {
    /// Its key (never shown).
    pub id: &'static str,
    /// Its name ("Subagent lists").
    pub name: &'static str,
    /// What it speeds up.
    pub speeds_up: &'static str,
    /// What is slower once it's cleared.
    pub slower: &'static str,
    /// How it may be made smaller.
    pub trim: Trim,
    /// Where its own switch is, when it has one (the cache's switch then
    /// leaves it alone).
    pub own_switch: Option<&'static str>,
    /// The most entries it keeps (the oldest beyond go).
    pub max_entries: Option<usize>,
    /// The settings that shape its entries: a change drops those made
    /// with the old value at once.
    pub shaped_by: &'static [&'static str],
}

/// One entry on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Its file.
    pub path: PathBuf,
    /// Its size.
    pub bytes: u64,
    /// When it was last written or read.
    pub used: SystemTime,
}

/// A local cache the manager sizes, trims and clears.
pub trait LocalCache: Send + Sync {
    /// What it is.
    fn about(&self) -> &About;
    /// Where it lives.
    fn location(&self) -> PathBuf;
    /// Its entries, at most `limit` of them (a whole-only cache: one).
    fn items(&self, limit: usize) -> Vec<Item>;
    /// Removes `item`; the bytes freed.
    fn remove(&self, item: &Item) -> u64;
    /// Removes everything it keeps.
    ///
    /// # Errors
    ///
    /// What couldn't be removed.
    fn clear(&self) -> std::io::Result<()>;
    /// Drops the entries the current format or settings don't match (at
    /// most `limit` looked at); how many went.
    fn purge_stale(&self, limit: usize) -> usize {
        let _ = limit;
        0
    }
}

/// What every writer asks before writing, and tells after: shared with
/// the caches without a lock.
#[derive(Debug)]
pub struct Gate {
    on: AtomicBool,
    low_disk: AtomicBool,
    written: AtomicU64,
    tier: AtomicU8,
}

impl Default for Gate {
    fn default() -> Self {
        Self {
            on: AtomicBool::new(true),
            low_disk: AtomicBool::new(false),
            written: AtomicU64::new(0),
            tier: AtomicU8::new(Tier::Medium.to_u8()),
        }
    }
}

impl Gate {
    /// The user keeps a cache on this Mac.
    #[must_use]
    pub fn is_on(&self) -> bool {
        self.on.load(Ordering::Acquire)
    }

    /// The disk has room for more.
    #[must_use]
    pub fn has_room(&self) -> bool {
        !self.low_disk.load(Ordering::Acquire)
    }

    /// A cache may write now.
    #[must_use]
    pub fn may_write(&self) -> bool {
        self.is_on() && self.has_room()
    }

    /// `bytes` were written.
    pub fn wrote(&self, bytes: u64) {
        self.written.fetch_add(bytes, Ordering::AcqRel);
    }

    /// This Mac's memory tier, for what caches keep in memory.
    #[must_use]
    pub fn tier(&self) -> Tier {
        Tier::from_u8(self.tier.load(Ordering::Acquire))
    }
}

/// One cache's size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// Bytes on disk.
    pub bytes: u64,
    /// Entries.
    pub entries: u64,
}

/// What the last walk found.
#[derive(Debug)]
struct Seen {
    walked_at: Option<Instant>,
    usage: HashMap<&'static str, Usage>,
    budget: Budget,
    disk: Option<DiskSpace>,
}

/// What one pass did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pass {
    /// It walked the folders.
    pub walked: bool,
    /// Files removed.
    pub removed: usize,
    /// Bytes on disk after it.
    pub total: u64,
}

/// The registry of local caches and their budget.
pub struct Manager {
    data_dir: PathBuf,
    gate: Arc<Gate>,
    caches: RwLock<Vec<Arc<dyn LocalCache>>>,
    limit: Mutex<Limit>,
    seen: Mutex<Seen>,
    /// The next pass walks, whatever the time.
    force: AtomicBool,
}

impl std::fmt::Debug for Manager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Manager")
            .field("gate", &self.gate)
            .field(
                "caches",
                &self.caches.read().map_or(0, |caches| caches.len()),
            )
            .finish_non_exhaustive()
    }
}

impl Manager {
    /// A manager for caches under `data_dir`, on or off, with `limit`,
    /// on a Mac with `memory` bytes.
    #[must_use]
    pub fn new(data_dir: PathBuf, on: bool, limit: Limit, memory: Option<u64>) -> Self {
        let gate = Gate::default();
        gate.on.store(on, Ordering::Release);
        gate.tier.store(Tier::of(memory).to_u8(), Ordering::Release);
        Self {
            data_dir,
            gate: Arc::new(gate),
            caches: RwLock::new(Vec::new()),
            limit: Mutex::new(limit),
            seen: Mutex::new(Seen {
                walked_at: None,
                usage: HashMap::new(),
                budget: budget::budget(limit, None, false),
                disk: None,
            }),
            force: AtomicBool::new(false),
        }
    }

    /// The manager as the stored settings say, on this Mac.
    #[must_use]
    pub fn from_settings(data_dir: PathBuf, storage: &juggler_core::storage::Storage) -> Self {
        let settings = storage.app_settings();
        let on = settings
            .get(SETTING_ON)
            .ok()
            .flatten()
            .is_none_or(|raw| raw != "off");
        let limit = Limit::parse(settings.get(SETTING_LIMIT).ok().flatten().as_deref());
        Self::new(data_dir, on, limit, juggler_platform::room::total_memory())
    }

    /// AI Juggler's data folder, where the caches live.
    #[must_use]
    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    /// The gate writers share.
    #[must_use]
    pub fn gate(&self) -> Arc<Gate> {
        Arc::clone(&self.gate)
    }

    /// Adds `cache` (one of the same id is replaced).
    pub fn register(&self, cache: Arc<dyn LocalCache>) {
        if let Ok(mut caches) = self.caches.write() {
            caches.retain(|known| known.about().id != cache.about().id);
            caches.push(cache);
        }
        self.force.store(true, Ordering::Release);
    }

    fn caches(&self) -> Vec<Arc<dyn LocalCache>> {
        self.caches
            .read()
            .map(|caches| caches.clone())
            .unwrap_or_default()
    }

    /// The limit the user chose.
    #[must_use]
    pub fn limit(&self) -> Limit {
        self.limit.lock().map_or(Limit::Automatic, |limit| *limit)
    }

    /// Sets the limit; the next pass trims to it.
    pub fn set_limit(&self, limit: Limit) {
        if let Ok(mut kept) = self.limit.lock() {
            *kept = limit;
        }
        self.force.store(true, Ordering::Release);
    }

    /// Turns the cache on or off. Off: nothing is written or read, and
    /// the files of every cache without its own switch are removed.
    ///
    /// # Errors
    ///
    /// A cache's files that couldn't be removed.
    pub fn set_on(&self, on: bool) -> std::io::Result<()> {
        self.gate.on.store(on, Ordering::Release);
        self.force.store(true, Ordering::Release);
        if on {
            return Ok(());
        }
        self.clear_where(|about| about.own_switch.is_none())
    }

    /// Removes every cache's files now (each is rebuilt as it's used).
    ///
    /// # Errors
    ///
    /// The first cache whose files couldn't be removed (the others are
    /// cleared all the same).
    pub fn clear(&self) -> std::io::Result<()> {
        self.clear_where(|_| true)
    }

    fn clear_where(&self, pick: impl Fn(&About) -> bool) -> std::io::Result<()> {
        let mut first = None;
        for cache in self.caches() {
            if pick(cache.about())
                && let Err(error) = cache.clear()
            {
                first.get_or_insert(error);
            }
        }
        self.force.store(true, Ordering::Release);
        first.map_or(Ok(()), Err)
    }

    /// The setting `key` changed: the entries it shaped go at once.
    pub fn setting_changed(&self, key: &str) {
        for cache in self.caches() {
            if cache.about().shaped_by.contains(&key) {
                cache.purge_stale(MAX_ITEMS);
            }
        }
        self.force.store(true, Ordering::Release);
    }

    /// The background pass, with the disk's space as read now (`None`:
    /// unknown): sets low-disk mode, and when a walk is due, measures
    /// every cache and trims them to the budget, the least recently used
    /// entries first, across caches.
    pub fn pass(&self, disk: Option<DiskSpace>) -> Pass {
        let Ok(mut seen) = self.seen.lock() else {
            return Pass::default();
        };
        let budget = budget::budget(self.limit(), disk, seen.budget.low_disk);
        self.gate.low_disk.store(budget.low_disk, Ordering::Release);
        seen.budget = budget;
        seen.disk = disk;
        let since = seen.walked_at.map(|at| at.elapsed());
        let written = self.gate.written.load(Ordering::Acquire);
        let estimate = seen.usage.values().map(|usage| usage.bytes).sum::<u64>() + written;
        let due = self.force.swap(false, Ordering::AcqRel)
            || budget.low_disk
            || estimate > budget.bytes
            || since.is_none_or(|since| {
                since >= WALK_EVERY || (written > 0 && since >= WALK_AFTER_WRITES)
            });
        if !due {
            return Pass {
                walked: false,
                removed: 0,
                total: estimate,
            };
        }
        let first = seen.walked_at.is_none();
        self.gate.written.store(0, Ordering::Release);
        let caches = self.caches();
        let (usage, removed) = trim(&caches, budget, self.gate.is_on(), first);
        let total = usage.values().map(|usage| usage.bytes).sum();
        seen.usage = usage;
        seen.walked_at = Some(Instant::now());
        Pass {
            walked: true,
            removed,
            total,
        }
    }

    /// Every cache with its size, as the last walk found it (walking now
    /// when none has, something was written since, or it's older than
    /// `fresh`), and the budget.
    fn measured(&self, fresh: Duration) -> (Vec<(About, Usage)>, Budget, Option<DiskSpace>) {
        let written = self.gate.written.load(Ordering::Acquire) > 0;
        let stale = written
            || self.seen.lock().map_or(true, |seen| {
                seen.walked_at.is_none_or(|at| at.elapsed() >= fresh)
            });
        if stale {
            self.force.store(true, Ordering::Release);
            let disk = juggler_platform::room::disk_space(&self.data_dir).ok();
            self.pass(disk);
        }
        let Ok(seen) = self.seen.lock() else {
            return (Vec::new(), budget::budget(self.limit(), None, false), None);
        };
        let rows = self
            .caches()
            .iter()
            .map(|cache| {
                let about = cache.about().clone();
                let usage = seen.usage.get(about.id).copied().unwrap_or_default();
                (about, usage)
            })
            .collect();
        (rows, seen.budget, seen.disk)
    }
}

/// Measures `caches` and trims them to `budget`: each to its own entry
/// cap, then the least recently used entries of the trimmable ones until
/// the total fits; on a nearly full disk, the whole-only ones too. While
/// the cache is off (`on` false), what a cache without its own switch
/// still holds goes. `purge` drops entries of another format first (the
/// first walk after a start).
fn trim(
    caches: &[Arc<dyn LocalCache>],
    budget: Budget,
    on: bool,
    purge: bool,
) -> (HashMap<&'static str, Usage>, usize) {
    let mut removed = 0;
    let mut lists: Vec<(&Arc<dyn LocalCache>, Vec<Item>)> = Vec::with_capacity(caches.len());
    for cache in caches {
        let about = cache.about();
        if !on && about.own_switch.is_none() {
            let _ = cache.clear();
            lists.push((cache, Vec::new()));
            continue;
        }
        if purge {
            removed += cache.purge_stale(MAX_ITEMS);
        }
        let mut items = cache.items(MAX_ITEMS);
        if let Some(cap) = about.max_entries
            && items.len() > cap
        {
            items.sort_by_key(|item| std::cmp::Reverse(item.used));
            for item in items.split_off(cap) {
                if removed >= MAX_REMOVALS {
                    break;
                }
                cache.remove(&item);
                removed += 1;
            }
        }
        lists.push((cache, items));
    }
    let mut total: u64 = lists
        .iter()
        .flat_map(|(_, items)| items.iter().map(|item| item.bytes))
        .sum();
    if total > budget.bytes {
        let mut oldest: Vec<(usize, usize)> = lists
            .iter()
            .enumerate()
            .filter(|(_, (cache, _))| cache.about().trim == Trim::Partly)
            .flat_map(|(at, (_, items))| (0..items.len()).map(move |item| (at, item)))
            .collect();
        oldest.sort_by_key(|(at, item)| lists[*at].1[*item].used);
        let mut gone: Vec<(usize, usize)> = Vec::new();
        for (at, item) in oldest {
            if total <= budget.bytes || removed >= MAX_REMOVALS {
                break;
            }
            let (cache, items) = &lists[at];
            let entry = &items[item];
            if cache.remove(entry) > 0 {
                total = total.saturating_sub(entry.bytes);
                gone.push((at, item));
            }
            removed += 1;
        }
        gone.sort_unstable_by(|a, b| b.cmp(a));
        for (at, item) in gone {
            lists[at].1.swap_remove(item);
        }
    }
    if total > budget.bytes && budget.low_disk {
        for (cache, items) in &mut lists {
            if cache.about().trim == Trim::WholeOnly && !items.is_empty() {
                if cache.clear().is_ok() {
                    let bytes: u64 = items.iter().map(|item| item.bytes).sum();
                    total = total.saturating_sub(bytes);
                    items.clear();
                }
                removed += 1;
            }
            if total <= budget.bytes {
                break;
            }
        }
    }
    let usage = lists
        .into_iter()
        .map(|(cache, items)| {
            (
                cache.about().id,
                Usage {
                    bytes: items.iter().map(|item| item.bytes).sum(),
                    entries: items.len() as u64,
                },
            )
        })
        .collect();
    (usage, removed)
}

/// Registers the caches AI Juggler keeps and starts the background pass.
pub fn start(state: &Arc<AppState>) {
    register_defaults(state);
    let weak = Arc::downgrade(state);
    crate::ticks::every_while("local-cache", TICK, move || {
        let Some(state) = weak.upgrade() else {
            return false;
        };
        let disk = juggler_platform::room::disk_space(&state.data_dir).ok();
        state.cache.pass(disk);
        true
    });
}

/// The subagent lists, the app icons and the search index.
pub fn register_defaults(state: &Arc<AppState>) {
    state
        .cache
        .register(crate::features::subagents::index::local_cache(state));
    state
        .cache
        .register(crate::features::app_icons::cache::local_cache(
            &state.data_dir,
        ));
    state
        .cache
        .register(crate::features::outside_search::local_cache(state));
}

/// A [`folder::FolderCache`]'s `current` that reads the app's state, or
/// says nothing matches once the app is gone.
pub fn with_state(
    state: &Arc<AppState>,
    current: impl Fn(&Arc<AppState>) -> (u32, u64) + Send + Sync + 'static,
) -> folder::Current {
    let weak = Arc::downgrade(state);
    Box::new(move || weak.upgrade().map_or((0, 0), |state| current(&state)))
}

/// Where the cache stands, for Settings.
///
/// # Errors
///
/// None today; the result type keeps the command's shape.
pub fn status_impl(state: &Arc<AppState>) -> AppResult<CacheInfo> {
    Ok(info_of(&state.cache, Duration::from_secs(30)))
}

fn info_of(manager: &Manager, fresh: Duration) -> CacheInfo {
    let (rows, budget, disk) = manager.measured(fresh);
    let limit = manager.limit();
    CacheInfo {
        on: manager.gate.is_on(),
        automatic: limit == Limit::Automatic,
        limit_mb: match limit {
            Limit::Automatic => None,
            Limit::Fixed(bytes) => Some(bytes / 1_000_000),
        },
        budget_bytes: budget.bytes,
        used_bytes: rows.iter().map(|(_, usage)| usage.bytes).sum(),
        low_disk: budget.low_disk,
        free_bytes: disk.map(|disk| disk.free),
        caches: rows
            .into_iter()
            .map(|(about, usage)| CacheRowInfo {
                key: about.id.to_owned(),
                name: about.name.to_owned(),
                speeds_up: about.speeds_up.to_owned(),
                slower: about.slower.to_owned(),
                bytes: usage.bytes,
                entries: usage.entries,
                whole_only: about.trim == Trim::WholeOnly,
                own_switch: about.own_switch.map(str::to_owned),
            })
            .collect(),
    }
}

/// Turns the cache on or off (off removes its files).
///
/// # Errors
///
/// Storage errors; [`AppError::Io`] when its files couldn't be removed.
pub fn set_on_impl(state: &Arc<AppState>, on: bool) -> AppResult<CacheInfo> {
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(SETTING_ON, if on { "" } else { "off" })?;
    state.cache.set_on(on).map_err(|error| AppError::Io {
        context: "remove the cache",
        message: error.to_string(),
    })?;
    Ok(info_of(&state.cache, Duration::ZERO))
}

/// Sets the limit: `None` for automatic, else megabytes.
///
/// # Errors
///
/// Storage errors.
pub fn set_limit_impl(state: &Arc<AppState>, limit_mb: Option<u64>) -> AppResult<CacheInfo> {
    let limit = limit_mb.map_or(Limit::Automatic, Limit::from_mb);
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(SETTING_LIMIT, &limit.stored())?;
    state.cache.set_limit(limit);
    Ok(info_of(&state.cache, Duration::ZERO))
}

/// Clears every cache now.
///
/// # Errors
///
/// [`AppError::Io`] when a cache's files couldn't be removed.
pub fn clear_impl(state: &Arc<AppState>) -> AppResult<CacheInfo> {
    state.cache.clear().map_err(|error| AppError::Io {
        context: "clear the cache",
        message: error.to_string(),
    })?;
    Ok(info_of(&state.cache, Duration::ZERO))
}

/// An app setting changed (`app_setting_set`): the cache entries it
/// shaped go.
pub fn setting_changed(state: &Arc<AppState>, key: &str) {
    state.cache.setting_changed(key);
}
