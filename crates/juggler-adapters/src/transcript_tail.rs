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
//! (a tool's output) never holds more memory than that. A log that asks
//! ([`TailLog::DROP_INLINE_DATA`], the conversation feed) has inline
//! binary data (a pasted image's base64, [`crate::inline_data`]) left
//! out of a line as it is held: a message long only because of its
//! image is read, its offsets the file's own ([`TailLog::feed_line`]).

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

use crate::inline_data::{self, Held, InlineData};

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

    /// Whether inline binary data (a pasted image's base64) is left out
    /// of the lines this log reads ([`crate::inline_data`]): a line long
    /// only because of it is read, not passed over. Such a line comes
    /// through [`Self::feed_line`], since it has fewer bytes than it
    /// spans in the file.
    const DROP_INLINE_DATA: bool = false;

    /// One whole line `bytes` (ending in its `\n`) that starts at `at`
    /// and ends at `end` in the file: a line held across reads, or one
    /// with its inline data left out ([`Self::DROP_INLINE_DATA`]), whose
    /// bytes may be fewer than it spans. A log that keeps only where
    /// lines start reads it as [`Self::feed_at`] does.
    fn feed_line(&mut self, at: u64, end: u64, bytes: &[u8]) {
        let _ = end;
        self.feed_at(bytes, at);
    }
}

/// A transcript read so far, into `L`. Saved and restored (AJ-E1520,
/// 0.3: the Subagents tab's index) with what it has read; a line still
/// being written is left out of a save ([`TranscriptTail::rewound`]).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TranscriptTail<L> {
    log: L,
    offset: u64,
    /// The line still being written (saved under the names it always
    /// had: `partial`, `skipping`).
    #[serde(flatten)]
    hold: LineHold,
    /// The file read so far (inode), to tell a replaced file.
    file: Option<u64>,
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
        cut_lines(&mut self.hold, &mut self.log, chunk, start);
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
            hold: LineHold::default(),
            file,
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
            offset: self.hold.start(self.offset).unwrap_or(self.offset),
            hold: LineHold::default(),
            file: self.file,
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

/// What a reader holds of a line still being written: its bytes so far
/// (inline data left out when the log asks), where it starts, or where a
/// line too long to hold started.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct LineHold {
    /// The held line's bytes so far.
    #[serde(default)]
    partial: Vec<u8>,
    /// Where the held line starts in the file: its bytes may be fewer
    /// than it spans ([`TailLog::DROP_INLINE_DATA`]).
    #[serde(default)]
    partial_at: Option<u64>,
    /// Where a line too long to hold started: the rest of it, up to its
    /// end, is passed over.
    #[serde(default)]
    skipping: Option<u64>,
    /// Where reading the held line is ([`InlineData`]).
    #[serde(default)]
    data: InlineData,
}

impl LineHold {
    /// Where the line still being written starts, when there is one;
    /// `read_to` is where the bytes read so far end (a hold an earlier
    /// version saved kept no start: its bytes were the line's, all of
    /// them).
    pub(crate) fn start(&self, read_to: u64) -> Option<u64> {
        self.skipping.or(self.partial_at).or_else(|| {
            (!self.partial.is_empty()).then(|| read_to.saturating_sub(self.partial.len() as u64))
        })
    }

    /// How many bytes of a line it holds.
    #[cfg(test)]
    pub(crate) fn held(&self) -> usize {
        self.partial.len()
    }

    /// Where the line it passes over started, while it does.
    #[cfg(test)]
    pub(crate) fn skipping(&self) -> Option<u64> {
        self.skipping
    }

    /// Takes in `bytes`, the next of the held line's (inline data left
    /// out when `L` asks); false when that would pass [`TailLog::LINE_MAX`]
    /// (nothing past it is held).
    fn take<L: TailLog>(&mut self, bytes: &[u8]) -> bool {
        if L::DROP_INLINE_DATA {
            return self.data.pass(bytes, &mut self.partial, L::LINE_MAX) == Held::Within;
        }
        if self.partial.len() + bytes.len() > L::LINE_MAX {
            return false;
        }
        self.partial.extend_from_slice(bytes);
        true
    }

    /// Ends the held line, which started at `at`, with `last` (its last
    /// bytes, before the `\n` at `end - 1`): to `log` whole, or as
    /// skipped when it is too long to hold.
    fn finish<L: TailLog>(&mut self, log: &mut L, at: u64, end: u64, last: &[u8]) {
        if self.take::<L>(last) {
            self.partial.push(b'\n');
            log.feed_line(at, end, &self.partial);
        } else {
            log.skipped_line(at);
        }
        *self = Self::default();
    }
}

/// Hands `log` the whole lines in `chunk` (which starts at `start` in
/// the file) after what `hold` holds of a line begun earlier, and holds
/// the line still being written: at most [`TailLog::LINE_MAX`] bytes,
/// past which the rest of that line is passed over and the log hears it
/// was skipped once its end comes. Offsets are the file's own, however
/// many bytes of a line were left out.
pub(crate) fn cut_lines<L: TailLog>(hold: &mut LineHold, log: &mut L, chunk: &[u8], start: u64) {
    let mut chunk = chunk;
    let mut start = start;
    // The line begun earlier: passed over, or held.
    if let Some(at) = hold.start(start) {
        let Some(end) = chunk.iter().position(|b| *b == b'\n') else {
            if hold.skipping.is_none() {
                hold.partial_at = Some(at);
                if !hold.take::<L>(chunk) {
                    *hold = LineHold {
                        skipping: Some(at),
                        ..LineHold::default()
                    };
                }
            }
            return;
        };
        let line_end = start + end as u64 + 1;
        if hold.skipping.is_some() {
            *hold = LineHold::default();
            log.skipped_line(at);
        } else {
            hold.finish(log, at, line_end, &chunk[..end]);
        }
        chunk = &chunk[end + 1..];
        start = line_end;
    }
    let whole = chunk
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |end| end + 1);
    let (lines, rest) = chunk.split_at(whole);
    feed_whole(hold, log, lines, start);
    // The line still being written.
    if !rest.is_empty() {
        let at = start + whole as u64;
        hold.partial_at = Some(at);
        if !hold.take::<L>(rest) {
            *hold = LineHold {
                skipping: Some(at),
                ..LineHold::default()
            };
        }
    }
}

/// Hands `log` `lines`, whole lines that start at `start` (with nothing
/// held): as they are, but for a log that leaves out inline data, each
/// line long enough to carry some ([`inline_data::KEEP_MAX`]) goes
/// through `hold` first.
fn feed_whole<L: TailLog>(hold: &mut LineHold, log: &mut L, lines: &[u8], start: u64) {
    if lines.is_empty() {
        return;
    }
    if !L::DROP_INLINE_DATA {
        log.feed_at(lines, start);
        return;
    }
    // A run of shorter lines, from here, goes on as it is.
    let mut run = 0;
    let mut at = 0;
    while let Some(len) = lines[at..].iter().position(|b| *b == b'\n') {
        let next = at + len + 1;
        if len > inline_data::KEEP_MAX {
            if run < at {
                log.feed_at(&lines[run..at], start + run as u64);
            }
            hold.finish(
                log,
                start + at as u64,
                start + next as u64,
                &lines[at..at + len],
            );
            run = next;
        }
        at = next;
    }
    if run < lines.len() {
        log.feed_at(&lines[run..], start + run as u64);
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
