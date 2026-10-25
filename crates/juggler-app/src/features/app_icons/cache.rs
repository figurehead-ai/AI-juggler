//! D-38: icons already read, in memory and in AI Juggler's own folder
//! (`icons/<bundle-key>-<version>-<px>.png`). A bundle that changes (an
//! update) has a new version, so its icon is read again and the old files
//! are removed. Nothing here leaves the Mac.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::bundle::Bundle;

/// The largest icon file kept or read back.
pub const MAX_PNG_BYTES: u64 = 1024 * 1024;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The cache: what was read this run, and the folder that keeps it.
#[derive(Debug)]
pub struct IconCache {
    dir: PathBuf,
    memory: Mutex<HashMap<String, Option<Vec<u8>>>>,
}

impl IconCache {
    /// A cache keeping its files in `dir`.
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            memory: Mutex::new(HashMap::new()),
        }
    }

    /// The file `bundle`'s icon at `px` is kept in.
    #[must_use]
    pub fn file_of(&self, bundle: &Bundle, px: u32) -> PathBuf {
        self.dir
            .join(format!("{}-{}-{px}.png", bundle.key, bundle.version))
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
        let png = read_png(&file).or_else(|| {
            let png = render().filter(|png| is_png(png))?;
            self.keep(bundle, &file, &png);
            Some(png)
        });
        if let Ok(mut memory) = self.memory.lock() {
            memory.insert(key, png.clone());
        }
        png
    }

    /// Writes `png` to `file` (through a temporary file, so a reader never
    /// sees half of it) and removes the same app's files of other
    /// versions. A write that fails leaves the icon in memory only.
    fn keep(&self, bundle: &Bundle, file: &Path, png: &[u8]) {
        if u64::try_from(png.len()).map_or(true, |len| len > MAX_PNG_BYTES)
            || std::fs::create_dir_all(&self.dir).is_err()
        {
            return;
        }
        let partial = file.with_extension("png.part");
        if std::fs::write(&partial, png).is_err() || std::fs::rename(&partial, file).is_err() {
            let _ = std::fs::remove_file(&partial);
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
            let png = Path::new(&name).extension().is_some_and(|ext| ext == "png");
            if other_version && png && !name.starts_with(&current) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// `bytes` start as a PNG does.
#[must_use]
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(PNG_SIGNATURE)
}

/// A kept icon, when it is a regular file (not a link) of at most
/// [`MAX_PNG_BYTES`] that starts as a PNG.
fn read_png(file: &Path) -> Option<Vec<u8>> {
    let meta = std::fs::symlink_metadata(file).ok()?;
    if !meta.file_type().is_file() || meta.len() > MAX_PNG_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(file)
        .ok()?
        .take(MAX_PNG_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    is_png(&bytes).then_some(bytes)
}
