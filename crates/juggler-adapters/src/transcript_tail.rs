//! Reading a transcript as it grows, for what a tick keeps about a live
//! session: its subagents (AJ-E1520), what it's working on (AJ-E1703).
//!
//! Each [`TranscriptTail::read`] takes in only the bytes appended since
//! the last one (at most [`READ_PER_CALL`], so a first read of a long
//! transcript is spread over a few ticks), and a line still being written
//! waits for the next read. What the lines mean is the log's business
//! ([`TailLog`]); this module only hands it whole lines, with where they
//! start in the file ([`TailLog::feed_at`], 0.3.5: the conversation feed's
//! turn ids are those offsets).
//!
//! A line still being written is held at most [`TailLog::LINE_MAX`]
//! bytes: past that the rest of it is passed over up to its end and the
//! log hears it was skipped ([`TailLog::skipped_line`]), so one huge line
//! (a tool's output, an image) never holds more memory than that.

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

/// The most bytes one [`TranscriptTail::read`] takes in.
pub const READ_PER_CALL: u64 = 16 * 1024 * 1024;

/// What a tail feeds: whole lines (`\n`-separated), in order.
pub trait TailLog: Default {
    /// The longest line a tail holds for this log while it is written;
    /// a longer one is skipped ([`Self::skipped_line`]).
    const LINE_MAX: usize = crate::transcript::LINE_MAX;

    /// Reads whole lines; a malformed line is skipped.
    fn feed(&mut self, bytes: &[u8]);

    /// Reads whole lines whose first starts at `base` in the file. A log
    /// that keeps no offsets reads them as [`Self::feed`] does.
    fn feed_at(&mut self, bytes: &[u8], base: u64) {
        let _ = base;
        self.feed(bytes);
    }

    /// A line that started at `at` was longer than [`Self::LINE_MAX`]
    /// and was passed over.
    fn skipped_line(&mut self, at: u64) {
        let _ = at;
    }
}

/// A transcript read so far, into `L`. Saved and restored (AJ-E1520,
/// 0.3: the Subagents tab's index) with what it has read; a line still
/// being written is left out of a save ([`TranscriptTail::rewound`]).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TranscriptTail<L> {
    log: L,
    offset: u64,
    partial: Vec<u8>,
    /// The file read so far (inode), to tell a replaced file.
    file: Option<u64>,
    /// Where a line too long to hold started: the rest of it, up to its
    /// end, is passed over.
    #[serde(default)]
    skipping: Option<u64>,
}

impl<L: TailLog> TranscriptTail<L> {
    /// Reads what was appended to `path` (at most 16 MiB a call). A file
    /// that was replaced (another file, or a shorter one) is read again
    /// from the start. Returns whether anything was read.
    ///
    /// # Errors
    ///
    /// The file can't be opened or read.
    pub fn read(&mut self, path: &Path) -> std::io::Result<bool> {
        self.read_up_to(path, READ_PER_CALL).map(|read| read > 0)
    }

    /// Reads what was appended to `path`, at most `most` bytes; how many
    /// were read (0: nothing new).
    ///
    /// # Errors
    ///
    /// As [`TranscriptTail::read`].
    pub fn read_up_to(&mut self, path: &Path, most: u64) -> std::io::Result<u64> {
        let mut file = std::fs::File::open(path)?;
        let metadata = file.metadata()?;
        let identity = Some(file_identity(&metadata));
        if metadata.len() < self.offset || (self.file.is_some() && self.file != identity) {
            *self = Self::default();
        }
        self.file = identity;
        if metadata.len() == self.offset {
            return Ok(0);
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut chunk = Vec::new();
        file.take(most.max(1)).read_to_end(&mut chunk)?;
        let start = self.offset;
        self.offset += chunk.len() as u64;
        self.take_in(&chunk, start);
        Ok(chunk.len() as u64)
    }

    /// The bytes `chunk`, which start at `start` in the file: whole lines
    /// to the log, the line still being written held (bounded).
    fn take_in(&mut self, chunk: &[u8], start: u64) {
        cut_lines(
            &mut self.partial,
            &mut self.skipping,
            &mut self.log,
            chunk,
            start,
        );
    }

    /// A tail of `log` that has read `path` up to `offset` (the end of a
    /// whole line), from the file `file` (its [`file_identity`]): what a
    /// log read elsewhere (the conversation feed's backward scan) goes on
    /// from.
    #[must_use]
    pub fn resume(log: L, offset: u64, file: Option<u64>) -> Self {
        Self {
            log,
            offset,
            partial: Vec::new(),
            file,
            skipping: None,
        }
    }

    /// Whether the file `metadata` describes is no longer the one read so
    /// far: another file at the path, or a shorter one.
    #[must_use]
    pub fn replaced_by(&self, metadata: &std::fs::Metadata) -> bool {
        metadata.len() < self.offset
            || (self.file.is_some() && self.file != Some(file_identity(metadata)))
    }

    /// Reads what was appended to `path` within `budget` bytes: true when
    /// it has caught up, and how many bytes it read.
    ///
    /// # Errors
    ///
    /// As [`TranscriptTail::read`].
    pub fn read_within(&mut self, path: &Path, budget: u64) -> std::io::Result<(bool, u64)> {
        let mut used = 0;
        while used < budget {
            let read = self.read_up_to(path, (budget - used).min(READ_PER_CALL))?;
            if read == 0 {
                return Ok((true, used));
            }
            used += read;
        }
        Ok((std::fs::metadata(path)?.len() == self.offset, used))
    }

    /// The bytes of `path` read so far.
    #[must_use]
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// What it has read without a line still being written: the offset
    /// goes back to that line's start, so a save never holds part of a
    /// line and a restore reads it whole.
    #[must_use]
    pub fn rewound(&self) -> Self
    where
        L: Clone,
    {
        Self {
            log: self.log.clone(),
            offset: self
                .skipping
                .unwrap_or_else(|| self.offset.saturating_sub(self.partial.len() as u64)),
            partial: Vec::new(),
            file: self.file,
            skipping: None,
        }
    }

    /// Reads all of `path`, 16 MiB at a time (memory stays bounded by what
    /// the log keeps, not by the file's size).
    ///
    /// # Errors
    ///
    /// As [`TranscriptTail::read`].
    pub fn read_all(&mut self, path: &Path) -> std::io::Result<()> {
        while self.read(path)? {}
        Ok(())
    }

    /// Reads what was appended to `path`, at most `calls` reads of
    /// [`READ_PER_CALL`] each: a very long transcript is read over a few
    /// asks, each answering at once with what it holds so far. True when
    /// it has caught up with the file's end.
    ///
    /// # Errors
    ///
    /// As [`TranscriptTail::read`].
    pub fn read_some(&mut self, path: &Path, calls: usize) -> std::io::Result<bool> {
        for _ in 0..calls {
            if !self.read(path)? {
                return Ok(true);
            }
        }
        Ok(std::fs::metadata(path)?.len() == self.offset)
    }

    /// What has been read so far.
    #[must_use]
    pub fn log(&self) -> &L {
        &self.log
    }

    /// What has been read so far, to take out what the caller keeps
    /// elsewhere (Kimi Code's usage records, once handed on).
    pub fn log_mut(&mut self) -> &mut L {
        &mut self.log
    }
}

/// Hands `log` the whole lines in `chunk` (which starts at `start` in
/// the file) after what `partial` holds of a line begun earlier, and
/// keeps the line still being written in `partial`: at most
/// [`TailLog::LINE_MAX`] bytes, past which the rest of that line is
/// passed over (`skipping` holds where it started) and the log hears it
/// was skipped once its end comes.
pub(crate) fn cut_lines<L: TailLog>(
    partial: &mut Vec<u8>,
    skipping: &mut Option<u64>,
    log: &mut L,
    chunk: &[u8],
    start: u64,
) {
    let mut chunk = chunk;
    let mut start = start;
    if let Some(at) = *skipping {
        let Some(end) = chunk.iter().position(|b| *b == b'\n') else {
            return;
        };
        *skipping = None;
        log.skipped_line(at);
        chunk = &chunk[end + 1..];
        start += end as u64 + 1;
    }
    // Where the held part of a line starts.
    let base = start - partial.len() as u64;
    // Only the new bytes are searched for a line's end.
    let (rest, rest_at) = if let Some(end) = chunk.iter().rposition(|b| *b == b'\n') {
        let (whole, rest) = chunk.split_at(end + 1);
        if partial.is_empty() {
            log.feed_at(whole, base);
        } else {
            partial.extend_from_slice(whole);
            let lines = std::mem::take(partial);
            log.feed_at(&lines, base);
        }
        (rest, start + whole.len() as u64)
    } else {
        (chunk, base)
    };
    if partial.len() + rest.len() > L::LINE_MAX {
        *partial = Vec::new();
        *skipping = Some(rest_at);
    } else {
        partial.extend_from_slice(rest);
    }
}

/// A file's identity (its inode), to tell it from another at the same
/// path. Elsewhere 0: only a shorter file is told apart there.
#[cfg(unix)]
#[must_use]
pub fn file_identity(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt as _;
    metadata.ino()
}

/// A file's identity: none to read on this system, so 0 (only a shorter
/// file is told apart from the one that was there).
#[cfg(not(unix))]
#[must_use]
pub fn file_identity(_metadata: &std::fs::Metadata) -> u64 {
    0
}
