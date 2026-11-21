//! AJ-E3803 (D-121 §2 as amended by D-131; ADR-0034 §5.2; ADR-0035
//! §11.4): the models on this Mac, and the disk rules a download must
//! pass before it starts.
//!
//! **Where they live.** One folder, [`FOLDER`] in AI Juggler's data folder:
//! each model is `<sha256>.gguf` with its installed copy `<sha256>.json`
//! beside it (the entry as a record of the section, read back by the same
//! reader, so it works offline and with no section). The folder is a
//! registered cache entry (D-58, [`local_cache`]) that only its own
//! Remove empties: the budget, a nearly full disk, turning the cache off
//! and Clear cache all leave it ([`crate::features::cache::Trim::Never`]).
//!
//! **The index.** The copies are read once, at the first question
//! (lazily), into memory; every total and list after that comes from it,
//! never from walking folders. That read also tidies the folder: a part
//! file left by a quit or a crash goes (a download never resumes across
//! a restart, so none is left behind), and so does a copy whose file is
//! missing or the wrong size, or a model file with no copy.
//!
//! **The rules** ([`Store::check_start`]):
//! - one downloaded model is the default; a further one needs an
//!   approval for that model ([`Store::approve`]), used once and
//!   expiring after [`APPROVAL_FOR`]: never a standing yes;
//! - a decline is remembered ([`Store::decline`]) until an approval of
//!   the same model;
//! - free disk of at least the size plus the margin
//!   ([`ModelEntry::free_disk_needed`]);
//! - Apple's model is never fetched; one download at a time.

use std::collections::BTreeSet;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Value, json};

use super::transfer::{MODEL_EXTENSION, PART_EXTENSION};
use super::{Listing, ModelEntry};
use crate::features::cache::{About, Item, LocalCache, Trim};

/// The models' folder, in AI Juggler's data folder.
pub const FOLDER: &str = "ai-models";

/// An approval stands this long, unused, before it lapses.
pub const APPROVAL_FOR: Duration = Duration::from_mins(10);

/// The installed copy's extension.
const COPY_EXTENSION: &str = "json";

/// The remembered declines.
const DECLINED_FILE: &str = "declined.json";

/// The largest installed copy read (a record is well under 4 KB).
const MAX_COPY_BYTES: u64 = 64 * 1024;

/// The installed copy's format.
const COPY_VERSION: u64 = 1;

/// One model on this Mac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// Its installed copy: the entry as it was when installed, or as a
    /// later entry with the same id and file refreshed it.
    pub entry: ModelEntry,
    /// Its file's SHA-256 (its name on disk).
    pub sha256: String,
    /// Its file's size.
    pub size_bytes: u64,
    /// When it was installed, milliseconds since the epoch.
    pub installed_at_ms: i64,
}

/// Why a download can't start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Not a model the app offers now.
    NotOffered,
    /// Apple's built-in model: nothing to fetch.
    NothingToFetch,
    /// Already on this Mac.
    AlreadyInstalled,
    /// Another download is running.
    Busy,
    /// A model is already on this Mac: this one needs the user's
    /// approval, on a card showing its size and the new total.
    NeedsApproval {
        /// Its size.
        size_bytes: u64,
        /// What AI models would take with it.
        total_after_bytes: u64,
    },
    /// Free disk is below its size plus the margin.
    NotEnoughDisk {
        /// The free disk it needs.
        needed_bytes: u64,
        /// The free disk now.
        free_bytes: u64,
    },
    /// The disk's free space can't be read.
    DiskUnknown,
}

/// One approval of one model.
#[derive(Debug)]
struct Approval {
    id: String,
    token: String,
    issued: Instant,
}

/// The models on this Mac, their index, and the approvals and declines.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    index: Mutex<Option<Vec<Installed>>>,
    approvals: Mutex<Vec<Approval>>,
    declined: Mutex<Option<BTreeSet<String>>>,
}

impl Store {
    /// The store in `dir` (nothing is read until the first question).
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            index: Mutex::new(None),
            approvals: Mutex::new(Vec::new()),
            declined: Mutex::new(None),
        }
    }

    /// The store under AI Juggler's `data_dir`.
    #[must_use]
    pub fn in_data_dir(data_dir: &Path) -> Self {
        Self::new(data_dir.join(FOLDER))
    }

    /// Its folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn with_index<R>(&self, body: impl FnOnce(&mut Vec<Installed>) -> R) -> Option<R> {
        let mut index = self.index.lock().ok()?;
        let index = index.get_or_insert_with(|| load(&self.dir));
        Some(body(index))
    }

    /// The models on this Mac, in the order they were installed.
    #[must_use]
    pub fn installed(&self) -> Vec<Installed> {
        self.with_index(|index| index.clone()).unwrap_or_default()
    }

    /// The ids of the models on this Mac.
    #[must_use]
    pub fn installed_ids(&self) -> Vec<String> {
        self.with_index(|index| index.iter().map(|m| m.entry.id.clone()).collect())
            .unwrap_or_default()
    }

    /// Whether `id` is on this Mac.
    #[must_use]
    pub fn is_installed(&self, id: &str) -> bool {
        self.with_index(|index| index.iter().any(|m| m.entry.id == id))
            .unwrap_or(false)
    }

    /// The disk the models use, from the index (no walk).
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.with_index(|index| index.iter().map(|m| m.size_bytes).sum())
            .unwrap_or(0)
    }

    /// Whether a download of `entry` may start now, `free` being the free
    /// disk where the models live (`None`: unknown) and `busy` whether
    /// another download runs. A further model needs `approval`, an
    /// unused token [`Store::approve`] gave for this model; it is used up
    /// here only when every other rule passes.
    ///
    /// # Errors
    ///
    /// The first rule it breaks.
    pub fn check_start(
        &self,
        entry: &ModelEntry,
        free: Option<u64>,
        busy: bool,
        approval: Option<&str>,
    ) -> Result<(), Refusal> {
        let Some(download) = &entry.download else {
            return Err(Refusal::NothingToFetch);
        };
        if self.is_installed(&entry.id) {
            return Err(Refusal::AlreadyInstalled);
        }
        if busy {
            return Err(Refusal::Busy);
        }
        let needed = entry.free_disk_needed();
        let free = free.ok_or(Refusal::DiskUnknown)?;
        if free < needed {
            return Err(Refusal::NotEnoughDisk {
                needed_bytes: needed,
                free_bytes: free,
            });
        }
        let total = self.total_bytes();
        let downloaded = self.with_index(|index| index.len()).unwrap_or(0);
        if downloaded >= 1 && !self.take_approval(&entry.id, approval) {
            return Err(Refusal::NeedsApproval {
                size_bytes: download.size_bytes,
                total_after_bytes: total.saturating_add(download.size_bytes),
            });
        }
        Ok(())
    }

    /// The user approved fetching `id` as a further model (the card's
    /// Add): a token for one start of that model within
    /// [`APPROVAL_FOR`]. It clears a remembered decline of it.
    #[must_use]
    pub fn approve(&self, id: &str) -> String {
        let mut bytes = [0u8; 16];
        // No randomness means no token anyone can guess, never a weak one.
        let token: String = if getrandom::fill(&mut bytes).is_ok() {
            crate::crash::hex(&bytes)
        } else {
            String::new()
        };
        if let Ok(mut approvals) = self.approvals.lock() {
            approvals
                .retain(|approval| approval.id != id && approval.issued.elapsed() < APPROVAL_FOR);
            if !token.is_empty() {
                approvals.push(Approval {
                    id: id.to_owned(),
                    token: token.clone(),
                    issued: Instant::now(),
                });
            }
        }
        self.set_declined(id, false);
        token
    }

    /// Uses up the approval `token` of `id`, if it stands.
    fn take_approval(&self, id: &str, token: Option<&str>) -> bool {
        let Some(token) = token.filter(|token| !token.is_empty()) else {
            return false;
        };
        let Ok(mut approvals) = self.approvals.lock() else {
            return false;
        };
        approvals.retain(|approval| approval.issued.elapsed() < APPROVAL_FOR);
        let found = approvals
            .iter()
            .position(|approval| approval.id == id && approval.token == token);
        found.map(|at| approvals.remove(at)).is_some()
    }

    /// The user declined the card for `id`: remembered, so it isn't
    /// offered again unasked. Any approval of it lapses.
    pub fn decline(&self, id: &str) {
        if let Ok(mut approvals) = self.approvals.lock() {
            approvals.retain(|approval| approval.id != id);
        }
        self.set_declined(id, true);
    }

    /// The models the user declined.
    #[must_use]
    pub fn declined(&self) -> Vec<String> {
        self.declined
            .lock()
            .map(|mut declined| {
                declined
                    .get_or_insert_with(|| load_declined(&self.dir))
                    .iter()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn set_declined(&self, id: &str, on: bool) {
        let Ok(mut declined) = self.declined.lock() else {
            return;
        };
        let declined = declined.get_or_insert_with(|| load_declined(&self.dir));
        let changed = if on {
            declined.insert(id.to_owned())
        } else {
            declined.remove(id)
        };
        if changed {
            let text = Value::from(declined.iter().cloned().collect::<Vec<_>>()).to_string();
            if std::fs::create_dir_all(&self.dir).is_ok() {
                let _ =
                    juggler_core::fs::atomic_write(&self.dir.join(DECLINED_FILE), text.as_bytes());
            }
        }
    }

    /// Adds the verified `file` (already `<sha256>.gguf` in the folder)
    /// as `entry`, writing its installed copy beside it.
    ///
    /// # Errors
    ///
    /// The copy couldn't be written, or `file` isn't the entry's: the
    /// file is then removed, so nothing half-installed stays.
    pub fn install(&self, entry: &ModelEntry, file: &Path) -> std::io::Result<Installed> {
        let result = self.install_inner(entry, file);
        if result.is_err() {
            let _ = std::fs::remove_file(file);
        }
        result
    }

    fn install_inner(&self, entry: &ModelEntry, file: &Path) -> std::io::Result<Installed> {
        let download = entry
            .download
            .as_ref()
            .ok_or_else(|| invalid("Apple's model has no file"))?;
        let expected = self.file_of(&download.sha256);
        if file != expected {
            return Err(invalid("not the entry's file"));
        }
        let size = std::fs::metadata(file)?.len();
        if size != download.size_bytes {
            return Err(invalid("not the entry's size"));
        }
        let installed = Installed {
            entry: entry.clone(),
            sha256: download.sha256.clone(),
            size_bytes: download.size_bytes,
            installed_at_ms: now_ms(),
        };
        write_copy(&self.dir, &installed)?;
        self.with_index(|index| {
            index.retain(|m| m.entry.id != entry.id);
            index.push(installed.clone());
        });
        Ok(installed)
    }

    /// Removes the model `id`: its file and its installed copy. The bytes
    /// freed, `None` when it wasn't installed.
    ///
    /// # Errors
    ///
    /// A file that couldn't be removed (the model stays listed then).
    pub fn remove(&self, id: &str) -> std::io::Result<Option<u64>> {
        let Some(found) = self
            .with_index(|index| index.iter().find(|m| m.entry.id == id).cloned())
            .flatten()
        else {
            return Ok(None);
        };
        remove_if_there(&self.file_of(&found.sha256))?;
        remove_if_there(&self.copy_of(&found.sha256))?;
        self.with_index(|index| index.retain(|m| m.entry.id != id));
        Ok(Some(found.size_bytes))
    }

    /// Refreshes the installed copies from `listing` (ADR-0035 §11.4): an
    /// entry with the same id **and** the same file replaces the copy
    /// (its verdicts may have narrowed or widened); one with another file
    /// is ignored; an unread section changes nothing. Whether any copy
    /// changed.
    pub fn refresh(&self, listing: &Listing) -> bool {
        let Listing::Read(entries) = listing else {
            return false;
        };
        let updates: Vec<Installed> = self
            .with_index(|index| {
                index
                    .iter()
                    .filter_map(|installed| {
                        let entry = entries.iter().find(|e| e.id == installed.entry.id)?;
                        let same_file = entry.engine == installed.entry.engine
                            && entry.download.as_ref().map(|d| (&d.sha256, d.size_bytes))
                                == Some((&installed.sha256, installed.size_bytes));
                        (same_file && *entry != installed.entry).then(|| Installed {
                            entry: entry.clone(),
                            ..installed.clone()
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut changed = false;
        for update in updates {
            if write_copy(&self.dir, &update).is_ok() {
                changed = true;
                self.with_index(|index| {
                    for installed in index.iter_mut() {
                        if installed.entry.id == update.entry.id {
                            *installed = update.clone();
                        }
                    }
                });
            }
        }
        changed
    }

    /// The model file for `sha256` (a 64-hex name, never from the URL).
    #[must_use]
    pub fn file_of(&self, sha256: &str) -> PathBuf {
        self.dir.join(format!("{sha256}.{MODEL_EXTENSION}"))
    }

    fn copy_of(&self, sha256: &str) -> PathBuf {
        self.dir.join(format!("{sha256}.{COPY_EXTENSION}"))
    }
}

fn invalid(why: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, why.to_owned())
}

fn remove_if_there(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or(0)
}

/// Whether `stem` is a SHA-256 in lower-case hex.
fn is_sha256(stem: &str) -> bool {
    stem.len() == 64
        && stem
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn write_copy(dir: &Path, installed: &Installed) -> std::io::Result<()> {
    let copy = json!({
        "version": COPY_VERSION,
        "installed_at_ms": installed.installed_at_ms,
        "record": installed.entry.to_record(),
    });
    std::fs::create_dir_all(dir)?;
    juggler_core::fs::atomic_write(
        &dir.join(format!("{}.{COPY_EXTENSION}", installed.sha256)),
        copy.to_string().as_bytes(),
    )
}

/// One installed copy, read back through the section's reader; `None` if
/// it isn't a good copy of a `sha256` model.
fn read_copy(path: &Path, sha256: &str) -> Option<(ModelEntry, i64)> {
    let mut text = String::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_COPY_BYTES)
        .read_to_string(&mut text)
        .ok()?;
    let copy: Value = serde_json::from_str(&text).ok()?;
    if copy.get("version").and_then(Value::as_u64) != Some(COPY_VERSION) {
        return None;
    }
    let installed_at_ms = copy.get("installed_at_ms").and_then(Value::as_i64)?;
    let record = copy.get("record")?.clone();
    let mut entries =
        super::read_untargeted(&json!({ "schema": super::SCHEMA, "items": [record] }))?;
    let entry = entries.pop().filter(|_| entries.is_empty())?;
    let download = entry.download.as_ref()?;
    (download.sha256 == sha256).then_some((entry, installed_at_ms))
}

/// Reads the folder into the index and tidies it (module docs).
fn load(dir: &Path) -> Vec<Installed> {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut copies: Vec<(String, PathBuf)> = Vec::new();
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for item in listing.flatten() {
        let path = item.path();
        let (Some(stem), Some(extension)) = (
            path.file_stem().and_then(|s| s.to_str()),
            path.extension().and_then(|e| e.to_str()),
        ) else {
            continue;
        };
        // A part file is `<sha256>-<n>.part`.
        if extension == PART_EXTENSION && stem.get(..64).is_some_and(is_sha256) {
            let _ = std::fs::remove_file(&path);
            continue;
        }
        if !is_sha256(stem) {
            continue;
        }
        match extension {
            COPY_EXTENSION => copies.push((stem.to_owned(), path)),
            MODEL_EXTENSION => files.push((stem.to_owned(), path)),
            _ => {}
        }
    }
    let mut index: Vec<Installed> = Vec::new();
    for (sha256, copy) in copies {
        let file = dir.join(format!("{sha256}.{MODEL_EXTENSION}"));
        let good = read_copy(&copy, &sha256).filter(|(entry, _)| {
            let size = entry.download.as_ref().map_or(0, |d| d.size_bytes);
            std::fs::metadata(&file).is_ok_and(|meta| meta.is_file() && meta.len() == size)
        });
        match good {
            Some((entry, installed_at_ms)) if !index.iter().any(|m| m.entry.id == entry.id) => {
                let size_bytes = entry.download.as_ref().map_or(0, |d| d.size_bytes);
                index.push(Installed {
                    entry,
                    sha256,
                    size_bytes,
                    installed_at_ms,
                });
            }
            _ => {
                tracing::warn!("models: an installed copy didn't read; its model is removed");
                let _ = std::fs::remove_file(&copy);
                let _ = std::fs::remove_file(&file);
            }
        }
    }
    for (sha256, file) in files {
        if !index.iter().any(|m| m.sha256 == sha256) {
            let _ = std::fs::remove_file(&file);
        }
    }
    index.sort_by_key(|m| m.installed_at_ms);
    index
}

fn load_declined(dir: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(dir.join(DECLINED_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .map(|ids| ids.into_iter().take(64).collect())
        .unwrap_or_default()
}

/// The models as a registered cache (D-58): listed by size from the
/// index, removed only by their own Remove.
struct ModelsCache {
    about: About,
    store: Arc<Store>,
}

impl LocalCache for ModelsCache {
    fn about(&self) -> &About {
        &self.about
    }

    fn location(&self) -> PathBuf {
        self.store.dir().to_path_buf()
    }

    fn items(&self, limit: usize) -> Vec<Item> {
        self.store
            .installed()
            .into_iter()
            .take(limit)
            .map(|installed| Item {
                path: self.store.file_of(&installed.sha256),
                bytes: installed.size_bytes,
                used: SystemTime::UNIX_EPOCH
                    + Duration::from_millis(u64::try_from(installed.installed_at_ms).unwrap_or(0)),
            })
            .collect()
    }

    fn remove(&self, item: &Item) -> u64 {
        // Only ever the user's Remove: the manager never trims a
        // `Trim::Never` cache.
        let found = self
            .store
            .installed()
            .into_iter()
            .find(|installed| self.store.file_of(&installed.sha256) == item.path);
        found
            .and_then(|installed| self.store.remove(&installed.entry.id).ok().flatten())
            .unwrap_or(0)
    }

    fn clear(&self) -> std::io::Result<()> {
        // Clear cache never removes a model (AJ-E3803 R6); the manager
        // doesn't ask, and this says no if anything ever does.
        Ok(())
    }
}

/// The app's store, made at the first use.
static STORE: OnceLock<Arc<Store>> = OnceLock::new();

/// The app's store under `data_dir` (the first caller's folder stands).
#[must_use]
pub fn shared(data_dir: &Path) -> Arc<Store> {
    Arc::clone(STORE.get_or_init(|| Arc::new(Store::in_data_dir(data_dir))))
}

/// The ids of the models on this Mac, from the app's store; none before
/// the store exists (the cache's registration makes it at start).
#[must_use]
pub fn shared_installed_ids() -> Vec<String> {
    STORE
        .get()
        .map(|store| store.installed_ids())
        .unwrap_or_default()
}

/// The models on this Mac, from the app's store (as
/// [`shared_installed_ids`]).
#[must_use]
pub fn shared_installed() -> Vec<Installed> {
    STORE
        .get()
        .map(|store| store.installed())
        .unwrap_or_default()
}

/// The app's store if it exists yet.
#[must_use]
pub fn shared_if_made() -> Option<Arc<Store>> {
    STORE.get().cloned()
}

/// The models' registered cache entry, "AI models", over the app's store.
#[must_use]
pub fn local_cache(data_dir: &Path) -> Arc<dyn LocalCache> {
    cache_of(shared(data_dir))
}

/// The registered cache entry over `store`.
#[must_use]
pub fn cache_of(store: Arc<Store>) -> Arc<dyn LocalCache> {
    Arc::new(ModelsCache {
        about: About {
            id: "ai_models",
            name: "AI models",
            speeds_up: "AI features on this computer",
            slower: "AI features that need a downloaded model stop until it is downloaded again",
            trim: Trim::Never,
            own_switch: None,
            max_entries: None,
            shaped_by: &[],
        },
        store,
    })
}
