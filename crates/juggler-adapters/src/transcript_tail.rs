//! Reading a transcript as it grows, for what a tick keeps about a live
//! session: its subagents (AJ-E1520), what it's working on (AJ-E1703).
//!
//! Each [`TranscriptTail::read`] takes in only the bytes appended since
//! the last one (at most [`READ_PER_CALL`], so a first read of a long
//! transcript is spread over a few ticks), and a line still being written
//! waits for the next read. What the lines mean is the log's business
//! ([`TailLog`]); this module only hands it whole lines.

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

/// The most bytes one [`TranscriptTail::read`] takes in.
pub const READ_PER_CALL: u64 = 16 * 1024 * 1024;

/// What a tail feeds: whole lines (`\n`-separated), in order.
pub trait TailLog: Default {
    /// Reads whole lines; a malformed line is skipped.
    fn feed(&mut self, bytes: &[u8]);
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
        self.offset += chunk.len() as u64;
        // Only the new bytes are searched for a line's end.
        let line_end = chunk.iter().rposition(|b| *b == b'\n');
        let kept = self.partial.len();
        self.partial.extend_from_slice(&chunk);
        if let Some(end) = line_end {
            let rest = self.partial.split_off(kept + end + 1);
            let whole = std::mem::replace(&mut self.partial, rest);
            self.log.feed(&whole);
        }
        Ok(chunk.len() as u64)
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
            offset: self.offset.saturating_sub(self.partial.len() as u64),
            partial: Vec::new(),
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

/// A file's identity (its inode), to tell it from another at the same
/// path. Elsewhere 0: only a shorter file is told apart there.
#[cfg(unix)]
fn file_identity(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt as _;
    metadata.ino()
}

#[cfg(not(unix))]
fn file_identity(_metadata: &std::fs::Metadata) -> u64 {
    0
}
