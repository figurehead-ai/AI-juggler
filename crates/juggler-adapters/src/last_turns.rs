//! AJ-E2108, 0.3.5 (ADR-0024): a conversation read backward a page at a
//! time, for its page (D-37: read when the page shows, kept nowhere).
//!
//! [`scan_back`] finds the turns that end before an offset (the file's
//! end, or the first turn a page shows): it reads a window back from
//! there, from the first whole line, and feeds it forward through the
//! one turn builder ([`TurnLog`]), so turn boundaries and ids are the
//! ones the live feed has. The first turn of a window that doesn't start
//! at the file's start may have begun before it, so it is dropped when
//! later ones follow (never when it is the only one: a run of replies
//! longer than the window shows its end, 0.1.8). The window grows
//! ([`WINDOWS`]) until it holds the turns asked for; a stretch with no
//! turn at all (a long run of tool output, subagent lines or progress
//! records) is passed and the scan goes on before it, within
//! [`SCAN_BUDGET`] bytes a call.
//!
//! "Found nothing" is never "there is nothing": a scan that stops before
//! the file's start says where it stopped ([`TurnLog::earlier`]), and the
//! page reads on from there. Only a read that reaches the file's start
//! answers that there is nothing before.
//!
//! What this returns is raw text unless the log scrubs (the app's does,
//! D-14). A compressed file (a finished Codex rollout) has no end to seek
//! to and is not read.

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

use crate::conversation_feed::TurnLog;
use crate::transcript::is_compressed;
use crate::transcript_tail::{cut_lines, file_identity};

/// The windows tried in turn, back from where a page ends.
pub const WINDOWS: [u64; 4] = [256 * 1024, 1024 * 1024, 4 * 1024 * 1024, MAX_WINDOW];

/// The largest window.
pub const MAX_WINDOW: u64 = 16 * 1024 * 1024;

/// The most bytes one scan reads before it answers (with where it got
/// to, when it found nothing).
pub const SCAN_BUDGET: u64 = 48 * 1024 * 1024;

/// How much is read at a time.
const CHUNK: usize = 256 * 1024;

/// What a scan read.
#[derive(Debug)]
pub struct BackPage {
    /// Its turns, oldest first, settled (nothing in them is news), and
    /// where they start ([`TurnLog::earlier`]: `None` at the file's
    /// start).
    pub log: TurnLog,
    /// Where the whole lines it read end: a tail goes on from here.
    pub end: u64,
    /// The file it read ([`file_identity`]).
    pub file: u64,
    /// The file's length when read.
    pub len: u64,
    /// Bytes read.
    pub read: u64,
}

/// What a backward read of a transcript found.
#[allow(clippy::large_enum_variant)] // one per read, handed straight on
#[derive(Debug)]
pub enum Back {
    /// A page of turns (perhaps none, with where to read on).
    Page(BackPage),
    /// Stored compressed: not read.
    Compressed,
}

/// The last `want` turns before `before` (the file's end when `None`)
/// of the transcript at `path`, built by a fresh log like `template`.
///
/// # Errors
///
/// The file can't be opened or read.
pub fn scan_back(
    path: &Path,
    before: Option<u64>,
    want: usize,
    template: &TurnLog,
) -> std::io::Result<Back> {
    if is_compressed(path) {
        return Ok(Back::Compressed);
    }
    let mut file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    let len = metadata.len();
    let want = want.max(1);
    let mut before = before.map_or(len, |before| before.min(len));
    let mut end_of_lines: Option<u64> = None;
    let mut read = 0;
    loop {
        let mut found = None;
        for size in WINDOWS {
            let start = before.saturating_sub(size);
            let from = line_start(&mut file, start, before)?;
            let mut log = template.fresh();
            log.begin_at(from);
            let end = feed_range(&mut file, from, before, &mut log)?;
            read += before - from;
            let enough = from == 0 || log.turns().len() > want || size >= MAX_WINDOW;
            found = Some((log, end, from));
            if enough {
                break;
            }
        }
        let Some((mut log, end, from)) = found else {
            break Err(std::io::Error::other("no window"));
        };
        // The tail goes on from the end of what the first window read.
        let end = *end_of_lines.get_or_insert(end);
        if from > 0 && log.turns().len() > 1 {
            log.drop_first();
        }
        log.keep_last(want);
        // A stretch with no turn: on before it, while the budget lasts.
        if log.turns().is_empty() && from > 0 && read < SCAN_BUDGET {
            before = from;
            continue;
        }
        log.settle();
        break Ok(Back::Page(BackPage {
            log,
            end,
            file: file_identity(&metadata),
            len,
            read,
        }));
    }
}

/// The turns from `from` (a turn's start) to the end of the transcript
/// at `path`, when that is at most [`MAX_WINDOW`] bytes; else its last
/// `want` turns (0.3.4's follow-along read, for the command kept one
/// release).
///
/// # Errors
///
/// The file can't be opened or read.
pub fn read_forward(
    path: &Path,
    from: u64,
    want: usize,
    template: &TurnLog,
) -> std::io::Result<Back> {
    if is_compressed(path) {
        return Ok(Back::Compressed);
    }
    let mut file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    let len = metadata.len();
    if from > len || len - from > MAX_WINDOW {
        return scan_back(path, None, want, template);
    }
    let mut log = template.fresh();
    log.begin_at(from);
    let end = feed_range(&mut file, from, len, &mut log)?;
    log.settle();
    Ok(Back::Page(BackPage {
        log,
        end,
        file: file_identity(&metadata),
        len,
        read: len - from,
    }))
}

/// Where the first whole line at or after `start` begins (`before` when
/// none does before it).
fn line_start(file: &mut std::fs::File, start: u64, before: u64) -> std::io::Result<u64> {
    if start == 0 {
        return Ok(0);
    }
    // The byte before `start` says whether a line begins at it.
    file.seek(SeekFrom::Start(start - 1))?;
    let mut buffer = vec![0u8; CHUNK];
    let mut at = start - 1;
    while at < before {
        let want = usize::try_from((before - at).min(CHUNK as u64)).unwrap_or(CHUNK);
        let got = file.read(&mut buffer[..want])?;
        if got == 0 {
            break;
        }
        if let Some(newline) = buffer[..got].iter().position(|b| *b == b'\n') {
            return Ok((at + newline as u64 + 1).min(before));
        }
        at += got as u64;
    }
    Ok(before)
}

/// Feeds `log` the whole lines in `[from, to)`; where they end.
fn feed_range(
    file: &mut std::fs::File,
    from: u64,
    to: u64,
    log: &mut TurnLog,
) -> std::io::Result<u64> {
    file.seek(SeekFrom::Start(from))?;
    let mut reader = (&*file).take(to.saturating_sub(from));
    let mut buffer = vec![0u8; CHUNK];
    let mut partial = Vec::new();
    let mut skipping = None;
    let mut at = from;
    loop {
        let got = reader.read(&mut buffer)?;
        if got == 0 {
            break;
        }
        cut_lines(&mut partial, &mut skipping, log, &buffer[..got], at);
        at += got as u64;
    }
    Ok(skipping.unwrap_or(at - partial.len() as u64))
}

/// The file's length and change time, in one string: equal stamps, an
/// unchanged file.
///
/// # Errors
///
/// The file's metadata can't be read.
pub fn stamp(path: &Path) -> std::io::Result<String> {
    let meta = std::fs::metadata(path)?;
    let changed = meta
        .modified()
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |at| at.as_nanos());
    Ok(format!("{}:{changed}", meta.len()))
}

#[cfg(test)]
#[path = "last_turns_tests.rs"]
mod tests;
