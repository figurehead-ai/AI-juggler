//! Crash-safe filesystem helpers.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Writes `bytes` to `path` atomically: content lands in a sibling temporary
/// file first, which is then renamed over the target.
///
/// Readers therefore never observe a partially written file, and an aborted
/// write leaves the previous content intact (`docs/plan/QUALITY.md` §1 —
/// degrade, never corrupt).
///
/// # Errors
///
/// Returns I/O errors from creating/writing the temporary file or the final
/// rename. The temporary file is removed on failure.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = temp_path_for(path);
    match write_temp(&temp, bytes) {
        Ok(()) => std::fs::rename(&temp, path).inspect_err(|_| {
            // Best-effort cleanup; the rename error is what matters.
            let _ = std::fs::remove_file(&temp);
        }),
        Err(e) => {
            let _ = std::fs::remove_file(&temp);
            Err(e)
        }
    }
}

/// Writes and flushes the temporary file.
fn write_temp(temp: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(temp)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Builds a unique sibling path so concurrent writers never collide.
fn temp_path_for(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = path.file_name().map_or_else(
        || std::ffi::OsString::from("file"),
        std::ffi::OsStr::to_os_string,
    );
    let mut temp_name =
        std::ffi::OsString::from(format!(".ai-juggler-tmp-{}-{}-", std::process::id(), n));
    temp_name.push(&file_name);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(temp_name)
}
