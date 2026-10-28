//! The technical side of an error a person saw (D-25 amendment AJ-E1025):
//! the screen says it in plain words ([`crate::error_words`]); the code
//! and the program's own text go here, `logs/errors.log` in AI Juggler's
//! data folder, which Export diagnostics carries (scrubbed as every log
//! there is). Bounded: past [`MAX_BYTES`] the older half is dropped.
//! A program's text can echo a key (a CLI's "invalid key sk-…"), so every
//! line is stripped of keys and passwords before it is written, and the
//! file is the owner's alone.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The log, inside the data folder.
pub const RELATIVE: &str = "logs/errors.log";
/// The most the log holds.
pub const MAX_BYTES: u64 = 256 * 1024;

static PATH: OnceLock<PathBuf> = OnceLock::new();
static WRITE: Mutex<()> = Mutex::new(());

/// Where the log goes: set once at start; until then nothing is written.
pub fn install(data_dir: &Path) {
    let _ = PATH.set(data_dir.join(RELATIVE));
}

/// One error: its code and the program's words, with the time.
pub fn record(code: &str, detail: &str) {
    if let Some(path) = PATH.get() {
        let _ = append(path, code, detail);
    }
}

fn append(path: &Path, code: &str, detail: &str) -> std::io::Result<()> {
    let _guard = WRITE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let half = text.len() / 2;
        let cut = text[half..]
            .find('\n')
            .map_or(text.len(), |at| half + at + 1);
        std::fs::write(path, &text[cut..])?;
    }
    let at = juggler_core::model::Timestamp::now().millis();
    let line: String = detail
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .collect();
    let line = without_keys(&line);
    // The owner's alone (the platform seam says how on each system).
    let mut file = juggler_platform::private_fs::append_private(path)?;
    writeln!(file, "{at} {code} {line}")
}

/// The line with every key or password it holds replaced (the same
/// detectors a switch's handover uses).
fn without_keys(line: &str) -> String {
    let (clean, _) = juggler_handover::redaction::strip_segments(
        &[line.to_owned()],
        true,
        &std::collections::BTreeSet::new(),
    );
    clean.into_iter().next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_per_error_and_the_log_stays_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(RELATIVE);
        append(&path, "not_found", "account acct-1 not found\nsecond line").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 1, "one line per error");
        assert!(
            text.contains("not_found account acct-1 not found"),
            "the code and the program's words"
        );
        let long = "x".repeat(1024);
        for _ in 0..400 {
            append(&path, "io", &long).unwrap();
        }
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size <= MAX_BYTES + 2048, "the log stays bounded");
    }

    #[test]
    fn a_key_in_a_programs_words_never_reaches_the_log() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(RELATIVE);
        append(
            &path,
            "io",
            "claude said: invalid key sk-FAKE0000NOTAREALKEY111",
        )
        .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("claude said: invalid key"), "the words stay");
        assert!(!text.contains("NOTAREALKEY"), "the key is gone");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "the owner's alone");
        }
    }
}
