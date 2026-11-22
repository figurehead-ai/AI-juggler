//! When a file changed or was made, as epoch milliseconds (the session
//! readers' sort keys and "last active" times).

use std::path::Path;
use std::time::SystemTime;

/// When a file last changed, epoch ms.
pub(crate) fn modified_ms(path: &Path) -> Option<i64> {
    epoch_ms(std::fs::metadata(path).ok()?.modified().ok()?)
}

/// When a file was made (else last changed), epoch ms.
pub(crate) fn born_ms(path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    epoch_ms(meta.created().or_else(|_| meta.modified()).ok()?)
}

fn epoch_ms(at: SystemTime) -> Option<i64> {
    i64::try_from(at.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_millis()).ok()
}
