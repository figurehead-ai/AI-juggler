//! D-38: icons already read, in memory and in AI Juggler's own folder
//! (`icons/<bundle-key>-<version>-<px>.icon`). A bundle that changes (an
//! update) has a new version, so its icon is read again and the old files
//! are removed. Nothing here leaves the Mac.
//!
//! The folder is one of the local caches (ADR-0023): each file carries
//! the cache's head line (its format and the bundle's version), is
//! written only while the cache is on and the disk has room, and is
//! readable by this user only. What is kept in memory is bounded by the
//! Mac's memory tier.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::bundle::Bundle;
use crate::features::cache::{About, Gate, LocalCache, Trim, folder::FolderCache, stamp};

/// The largest icon file kept or read back.
pub const MAX_PNG_BYTES: u64 = 1024 * 1024;

/// The cache's key.
pub const CACHE: &str = "icons";

/// The files' format: another is not read. (1: the cache's head line,
/// ADR-0023; before it, plain `.png` files, removed on the next walk.)
pub const FORMAT: u32 = 1;

/// The folder under AI Juggler's data folder.
pub const DIR: &str = "icons";

/// The files' extension.
const EXTENSION: &str = "icon";

/// How many icons a Mac of medium memory keeps in memory (a few KB
/// each).
const IN_MEMORY: usize = 128;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The app icons, for the cache's registry.
#[must_use]
pub fn local_cache(data_dir: &Path) -> Arc<dyn LocalCache> {
    Arc::new(FolderCache::new(
        About {
            id: CACHE,
            name: "App icons",
            speeds_up: "The apps' icons show at once, without asking macOS for them again.",
            slower: "Each app's icon is read from the app again the first time it shows.",
            trim: Trim::Partly,
            own_switch: None,
            max_entries: None,
            shaped_by: &[],
        },
        data_dir.join(DIR),
        EXTENSION,
        &["png"],
        Box::new(|| (FORMAT, 0)),
    ))
}

/// The cache: what was read this run, and the folder that keeps it.
#[derive(Debug)]
pub struct IconCache {
    dir: PathBuf,
    gate: Arc<Gate>,
    memory: Mutex<HashMap<String, Option<Vec<u8>>>>,
}

impl IconCache {
    /// A cache keeping its files in `dir`, as `gate` allows.
    #[must_use]
    pub fn new(dir: PathBuf, gate: Arc<Gate>) -> Self {
        Self {
            dir,
            gate,
            memory: Mutex::new(HashMap::new()),
        }
    }

    /// The cache in AI Juggler's data folder, as the local cache's
    /// `manager` allows.
    #[must_use]
    pub fn of(manager: &crate::features::cache::Manager) -> Self {
        Self::new(manager.data_dir().join(DIR), manager.gate())
    }

    /// The file `bundle`'s icon at `px` is kept in.
    #[must_use]
    pub fn file_of(&self, bundle: &Bundle, px: u32) -> PathBuf {
        self.dir.join(format!(
            "{}-{}-{px}.{EXTENSION}",
            bundle.key, bundle.version
        ))
    }

    /// `bundle`'s icon at `px`: from memory, else from its file, else
    /// from `render` (then kept in both). `None` (kept in memory only)
    /// when it can't be read.
    pub fn get_or_render(
        &self,
        bundle: &Bundle,
        px: u32,
        render: impl FnOnce() -> Option<Vec<u8>>,
    ) -> Option<Vec<u8>> {
        let file = self.file_of(bundle, px);
        let key = file.to_string_lossy().into_owned();
        if let Ok(memory) = self.memory.lock()
            && let Some(known) = memory.get(&key)
        {
            return known.clone();
        }
        let source = source_of(bundle);
        let png = read_png(&self.gate, &file, &source).or_else(|| {
            let png = render().filter(|png| is_png(png))?;
            self.keep(bundle, &file, &source, &png);
            Some(png)
        });
        if let Ok(mut memory) = self.memory.lock() {
            // Bounded by the Mac's memory: past it, start again (each
            // icon is one read of its file away).
            if memory.len() >= self.gate.tier().scale(IN_MEMORY) {
                memory.clear();
            }
            memory.insert(key, png.clone());
        }
        png
    }

    /// Writes `png` to `file` (whole or not at all, readable by this user
    /// only) and removes the same app's files of other versions. A write
    /// that fails, or isn't allowed now, leaves the icon in memory only.
    fn keep(&self, bundle: &Bundle, file: &Path, source: &str, png: &[u8]) {
        if u64::try_from(png.len()).map_or(true, |len| len > MAX_PNG_BYTES) {
            return;
        }
        let stamp = stamp::Stamp {
            cache: CACHE,
            format: FORMAT,
            settings: 0,
            source,
        };
        if !matches!(stamp::write(&self.gate, file, &stamp, png), Ok(true)) {
            return;
        }
        let current = format!("{}-{}-", bundle.key, bundle.version);
        let same_app = format!("{}-", bundle.key);
        for entry in std::fs::read_dir(&self.dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let other_version = name
                .strip_prefix(&same_app)
                .and_then(|rest| rest.split('-').next())
                .is_some_and(|version| version.bytes().all(|b| b.is_ascii_digit()));
            let ours = Path::new(&name)
                .extension()
                .is_some_and(|ext| ext == EXTENSION);
            if other_version && ours && !name.starts_with(&current) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// The bundle's identity an icon is kept for.
fn source_of(bundle: &Bundle) -> String {
    format!("{}-{}", bundle.key, bundle.version)
}

/// `bytes` start as a PNG does.
#[must_use]
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(PNG_SIGNATURE)
}

/// A kept icon, when it is a regular file (not a link) of at most
/// [`MAX_PNG_BYTES`] kept for `source` that starts as a PNG.
fn read_png(gate: &Gate, file: &Path, source: &str) -> Option<Vec<u8>> {
    let stamp = stamp::Stamp {
        cache: CACHE,
        format: FORMAT,
        settings: 0,
        source,
    };
    let bytes = stamp::read(gate, file, &stamp, MAX_PNG_BYTES + 4096)?;
    is_png(&bytes).then_some(bytes)
}
