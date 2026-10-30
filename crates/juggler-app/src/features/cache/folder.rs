//! ADR-0023: a cache that is a folder of entry files ([`super::stamp`]):
//! the shape of most caches, so one registers in a few lines — its words,
//! its folder, its files' extension, and what its entries are shaped by
//! now ([`FolderCache::new`]).

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{About, Item, LocalCache, stamp};

/// A temporary file older than this is a write that was cut short.
const PARTIAL_AGE: std::time::Duration = std::time::Duration::from_secs(60);

/// The current format and settings print entries must have.
pub type Current = Box<dyn Fn() -> (u32, u64) + Send + Sync>;

/// A folder of entry files.
pub struct FolderCache {
    about: About,
    dir: PathBuf,
    extension: &'static str,
    /// Older formats' extensions: their files go on the next walk.
    legacy: &'static [&'static str],
    current: Current,
}

impl std::fmt::Debug for FolderCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FolderCache")
            .field("id", &self.about.id)
            .finish_non_exhaustive()
    }
}

impl FolderCache {
    /// The cache `about` describes, of the `*.{extension}` files in `dir`
    /// (files of the `legacy` extensions are an older format's, removed
    /// on the next walk). `current` says the format and settings print an
    /// entry must have now.
    #[must_use]
    pub fn new(
        about: About,
        dir: PathBuf,
        extension: &'static str,
        legacy: &'static [&'static str],
        current: Current,
    ) -> Self {
        Self {
            about,
            dir,
            extension,
            legacy,
            current,
        }
    }

    fn has_extension(path: &Path, extension: &str) -> bool {
        path.extension().is_some_and(|ext| ext == extension)
    }
}

impl LocalCache for FolderCache {
    fn about(&self) -> &About {
        &self.about
    }

    fn location(&self) -> PathBuf {
        self.dir.clone()
    }

    fn items(&self, limit: usize) -> Vec<Item> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut items = Vec::new();
        for entry in entries.flatten().take(limit) {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.file_type().is_file() {
                continue;
            }
            let cut_short = Self::has_extension(&path, "partial")
                && meta
                    .modified()
                    .ok()
                    .and_then(|at| at.elapsed().ok())
                    .is_some_and(|age| age > PARTIAL_AGE);
            if self
                .legacy
                .iter()
                .any(|ext| Self::has_extension(&path, ext))
                || cut_short
            {
                // An older format's file, or a write cut short: not an
                // entry, and never read again.
                let _ = std::fs::remove_file(&path);
                continue;
            }
            if !Self::has_extension(&path, self.extension) {
                continue;
            }
            items.push(Item {
                bytes: meta.len(),
                used: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                path,
            });
        }
        items
    }

    fn remove(&self, item: &Item) -> u64 {
        if item.path.parent() == Some(self.dir.as_path())
            && std::fs::remove_file(&item.path).is_ok()
        {
            item.bytes
        } else {
            0
        }
    }

    fn clear(&self) -> std::io::Result<()> {
        match std::fs::symlink_metadata(&self.dir) {
            Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(&self.dir),
            Ok(_) => std::fs::remove_file(&self.dir),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn purge_stale(&self, limit: usize) -> usize {
        let (format, settings) = (self.current)();
        let mut dropped = 0;
        for item in self.items(limit) {
            let mut head = Vec::with_capacity(512);
            let readable = std::fs::File::open(&item.path)
                .and_then(|file| file.take(4096).read_to_end(&mut head))
                .is_ok();
            let fresh = readable
                && stamp::head_of(&head).is_some_and(|(cache, f, s)| {
                    cache == self.about.id && f == format && s == settings
                });
            if !fresh && self.remove(&item) > 0 {
                dropped += 1;
            }
        }
        dropped
    }
}
