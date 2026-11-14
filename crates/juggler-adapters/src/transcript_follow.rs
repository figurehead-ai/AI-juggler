//! 0.4.5 (AJ-E805): following a JSONL file as it grows: from an offset to
//! its last whole line, a chunk of whole lines at a time (0.1: a 5 GB
//! rollout was read whole on its first pass), truncation-aware. A
//! compressed file (`*.zst`: Codex compresses its older rollouts, and never
//! writes to one again) is streamed whole whenever its size changes, a
//! slice at a time (fix batch 5). The usage tail
//! ([`crate::quota_ingest::tail_jsonl`]) and the transcript readers
//! ([`juggler_adapter::transcript_read`]) both read through it.

use std::io::{Read as _, Seek as _};
use std::path::Path;

/// What [`follow`] hands on, in order.
pub enum Step<'a> {
    /// Reading starts here: the offset asked for, or 0 when the file is
    /// shorter than it (truncated or replaced) or compressed and changed
    /// (read whole again): what was read of it before no longer holds.
    From(u64),
    /// Whole lines, each through its newline.
    Lines(&'a [u8]),
}

/// Why a compressed file's lines, handed on, don't count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Void {
    /// It changed in the last few seconds and won't decode yet: read it
    /// again next time (its offset is kept).
    NotReady,
    /// It won't decode: one bad line, not read again until it changes.
    Damaged,
    /// It decompresses past [`COMPRESSED_MAX`]: left unread.
    TooLarge,
}

/// Where a [`follow`] got to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Followed {
    /// The next read starts here.
    pub offset: u64,
    /// Lines skipped for their length (damage).
    pub skipped: u64,
    /// A compressed file whose lines, already handed on, don't count.
    pub void: Option<Void>,
}

/// The most of a file read into memory at once while following it.
pub(crate) const CHUNK: u64 = 64 * 1024 * 1024;

thread_local! {
    /// Bytes this thread has read through [`follow`].
    static FOLLOWED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Bytes this thread has read through [`follow`] since it started: what a
/// test measures a pass by (one that read an appended transcript whole
/// again shows the whole file here).
#[must_use]
pub fn followed_here() -> u64 {
    FOLLOWED.with(std::cell::Cell::get)
}

/// Reads `path` from `offset` through its last whole line, handing `each`
/// where it starts and then the lines. Nothing is read when nothing was
/// written since (or the file can't be opened).
pub fn follow(path: &Path, offset: u64, each: impl FnMut(Step<'_>)) -> Followed {
    follow_by(path, offset, CHUNK, each)
}

/// [`follow`], `chunk` bytes at a time (the tests read in small ones).
pub(crate) fn follow_by(
    path: &Path,
    offset: u64,
    chunk: u64,
    mut each: impl FnMut(Step<'_>),
) -> Followed {
    let mut each = |step: Step<'_>| {
        if let Step::Lines(lines) = step {
            FOLLOWED.with(|read| read.set(read.get() + lines.len() as u64));
        }
        each(step);
    };
    let unchanged = Followed {
        offset,
        ..Followed::default()
    };
    if crate::transcript::is_compressed(path) {
        return follow_compressed(path, offset, each);
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return unchanged;
    };
    let len = file.metadata().map_or(offset, |meta| meta.len());
    let mut start = if len < offset { 0 } else { offset };
    if start >= len {
        if start != offset {
            each(Step::From(start));
        }
        return Followed {
            offset: start,
            ..Followed::default()
        };
    }
    if let Ok(seeked) = file.seek(std::io::SeekFrom::Start(start)) {
        start = seeked;
    }
    each(Step::From(start));
    let mut followed = Followed {
        offset: start,
        ..Followed::default()
    };
    let mut at = start;
    let mut bytes = Vec::new();
    while at < len {
        bytes.clear();
        let _ = (&mut file).take(chunk).read_to_end(&mut bytes);
        if bytes.is_empty() {
            break;
        }
        // Only through the last newline (a partial trailing line stays for
        // the next read).
        let consumed = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        if consumed == 0 {
            if (bytes.len() as u64) < chunk {
                break; // the last line, still being written
            }
            // One line longer than a chunk: no usage line is; skipped
            // whole, never held.
            match skip_line(&mut file) {
                Some(skipped) => {
                    at += chunk + skipped;
                    followed.skipped += 1;
                    continue;
                }
                None => break,
            }
        }
        each(Step::Lines(&bytes[..consumed]));
        at += consumed as u64;
        if file.seek(std::io::SeekFrom::Start(at)).is_err() {
            break;
        }
    }
    followed.offset = at;
    followed
}

/// Reads on to the end of the current line; the bytes read, newline
/// included, or `None` when the file ends first (the line is unfinished).
fn skip_line(file: &mut std::fs::File) -> Option<u64> {
    let mut buf = vec![0_u8; 1 << 20];
    let mut read = 0_u64;
    loop {
        let got = file.read(&mut buf).ok()?;
        if got == 0 {
            return None;
        }
        if let Some(at) = buf[..got].iter().position(|byte| *byte == b'\n') {
            let used = at as u64 + 1;
            let back = i64::try_from(got as u64 - used).ok()?;
            file.seek(std::io::SeekFrom::Current(-back)).ok()?;
            return Some(read + used);
        }
        read += got as u64;
    }
}

/// A conversation file read whole through its reader (a tool's
/// `read_conversation`, when its files are append-only logs: one reader,
/// whole or as it grows). Its last line counts even without a newline
/// (the file is read once, not followed).
#[must_use]
pub fn read_whole(
    path: &Path,
    reader: Box<dyn juggler_adapter::transcript_read::TranscriptReader>,
) -> Option<juggler_adapter::conversation::ConversationSummary> {
    let mut file = juggler_adapter::transcript_read::ConversationFile::new(reader);
    let followed = follow(path, 0, |step| {
        if let Step::Lines(lines) = step {
            let _ = file.read(lines);
        }
    });
    if !crate::transcript::is_compressed(path)
        && let Ok(mut rest) = std::fs::File::open(path)
        && rest.seek(std::io::SeekFrom::Start(followed.offset)).is_ok()
    {
        let mut last = Vec::new();
        if rest
            .take(crate::common::jsonl::LINE_MAX as u64)
            .read_to_end(&mut last)
            .is_ok()
            && !last.iter().all(u8::is_ascii_whitespace)
        {
            last.push(b'\n');
            let _ = file.read(&last);
        }
    }
    file.summary(path)
}

/// How much a compressed file may decompress to before AI Juggler stops
/// and leaves it unread, so a damaged or hostile file can't keep a read
/// busy: far above any real rollout (the largest seen, a 50,000-turn
/// thread, is 2.3 GB). Memory is bounded by the slice and the line cap,
/// not by this.
const COMPRESSED_MAX: u64 = 16 << 30;

/// Decompressed bytes taken at a time.
const COMPRESSED_SLICE: usize = 8 << 20;

/// The longest line held while it is still coming. A longer one is dropped
/// (one bad line) and reading picks up after its end, so one line without a
/// newline never fills memory.
const COMPRESSED_LINE_MAX: usize = 64 << 20;

/// A compressed file changed this recently may still be being written.
const COMPRESSED_SETTLE: std::time::Duration = std::time::Duration::from_secs(10);

/// A compressed file: complete, never written to again. It is
/// decompressed as a stream, a slice at a time, so a large one never sits
/// in memory whole. Its offset is the compressed file's size: a later read
/// at the same size reads nothing, and one that changed is read whole
/// again. One that won't decode (or is too large) voids what it handed on
/// ([`Void`]).
fn follow_compressed(path: &Path, offset: u64, mut each: impl FnMut(Step<'_>)) -> Followed {
    let Ok(meta) = std::fs::metadata(path) else {
        return Followed {
            offset,
            ..Followed::default()
        };
    };
    let size = meta.len();
    if offset == size {
        return Followed {
            offset,
            ..Followed::default()
        };
    }
    let settling = meta
        .modified()
        .ok()
        .and_then(|at| at.elapsed().ok())
        .is_some_and(|age| age < COMPRESSED_SETTLE);
    each(Step::From(0));
    let limits = LineLimits {
        slice: COMPRESSED_SLICE,
        line: COMPRESSED_LINE_MAX,
        total: COMPRESSED_MAX,
    };
    let read = std::fs::File::open(path)
        .ok()
        .and_then(|file| zstd::stream::Decoder::new(file).ok())
        .ok_or(Unreadable::Damaged)
        .and_then(|decoder| for_each_lines(decoder, limits, |lines| each(Step::Lines(lines))));
    let at = |void| Followed {
        offset: size,
        skipped: 0,
        void: Some(void),
    };
    match read {
        Ok(read) => Followed {
            offset: size,
            skipped: read.overlong,
            void: None,
        },
        Err(Unreadable::TooLarge) => at(Void::TooLarge),
        Err(Unreadable::Damaged) if settling => Followed {
            offset,
            skipped: 0,
            void: Some(Void::NotReady),
        },
        Err(Unreadable::Damaged) => at(Void::Damaged),
    }
}

/// Why a compressed file wasn't read.
enum Unreadable {
    TooLarge,
    Damaged,
}

/// How [`for_each_lines`] reads: a slice at a time, a line held up to
/// `line` bytes, and no more than `total` in all.
#[derive(Clone, Copy)]
struct LineLimits {
    slice: usize,
    line: usize,
    total: u64,
}

/// What [`for_each_lines`] saw besides the lines it handed on.
#[derive(Debug, Default, PartialEq, Eq)]
struct LinesRead {
    /// Lines dropped for being longer than the cap.
    overlong: u64,
    /// The most bytes held at once, besides the slice being read.
    held_peak: usize,
}

/// Reads `reader` a slice at a time and hands `each` the complete lines
/// (through their newlines) as they come. A line still unfinished past
/// `limits.line` is dropped up to its newline. Whatever follows the last
/// newline is the file's last line (a compressed file is finished), handed
/// on with one.
fn for_each_lines(
    mut reader: impl std::io::Read,
    limits: LineLimits,
    mut each: impl FnMut(&[u8]),
) -> Result<LinesRead, Unreadable> {
    let mut seen = LinesRead::default();
    let mut pending: Vec<u8> = Vec::new();
    let mut skipping = false;
    let mut slice = vec![0u8; limits.slice];
    let mut total = 0u64;
    loop {
        let read = reader.read(&mut slice).map_err(|_| Unreadable::Damaged)?;
        if read == 0 {
            // A compressed file is finished: its last line counts without
            // a newline too.
            if !skipping && !pending.iter().all(u8::is_ascii_whitespace) {
                pending.push(b'\n');
                each(&pending);
            }
            return Ok(seen);
        }
        total += read as u64;
        if total > limits.total {
            return Err(Unreadable::TooLarge);
        }
        let mut fresh = &slice[..read];
        if skipping {
            // The rest of a dropped line: resume after its newline.
            let Some(end) = fresh.iter().position(|byte| *byte == b'\n') else {
                continue;
            };
            fresh = &fresh[end + 1..];
            skipping = false;
        }
        pending.extend_from_slice(fresh);
        seen.held_peak = seen.held_peak.max(pending.len());
        match pending.iter().rposition(|byte| *byte == b'\n') {
            Some(last) => {
                each(&pending[..=last]);
                pending.drain(..=last);
            }
            None if pending.len() > limits.line => {
                seen.overlong += 1;
                pending.clear();
                skipping = true;
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod line_tests {
    use super::{LineLimits, LinesRead, Unreadable, for_each_lines};

    const SMALL: LineLimits = LineLimits {
        slice: 16,
        line: 64,
        total: 10_000,
    };

    fn lines_of(bytes: &[u8], limits: LineLimits) -> (Vec<u8>, Result<LinesRead, Unreadable>) {
        let mut handed = Vec::new();
        let read = for_each_lines(bytes, limits, |lines| handed.extend_from_slice(lines));
        (handed, read)
    }

    /// Lines split across slices come out whole; the last line counts
    /// without its newline (a compressed file is finished).
    #[test]
    fn lines_cross_slices_whole() {
        let text = b"{\"a\":1}\n{\"b\":\"a longer value here\"}\nlast";
        let (handed, read) = lines_of(text, SMALL);
        assert_eq!(
            handed,
            b"{\"a\":1}\n{\"b\":\"a longer value here\"}\nlast\n"
        );
        assert_eq!(read.ok().map(|read| read.overlong), Some(0));
    }

    /// QA P20: one line without a newline never fills memory. It is dropped
    /// once past the cap, and reading resumes after its end.
    #[test]
    fn an_endless_line_is_dropped_not_held() {
        let mut text = vec![b'x'; 5_000];
        text.extend_from_slice(b"\n{\"after\":1}\n");
        let (handed, read) = lines_of(&text, SMALL);
        let read = read.ok();
        assert_eq!(handed, b"{\"after\":1}\n");
        assert_eq!(read.as_ref().map(|read| read.overlong), Some(1));
        let held = read.map_or(usize::MAX, |read| read.held_peak);
        assert!(held <= SMALL.line + SMALL.slice, "held {held} bytes");
    }

    /// Past the total it stops: too large, not damaged.
    #[test]
    fn past_the_total_is_too_large() {
        let text = b"{}\n".repeat(4_000);
        let (_, read) = lines_of(&text, SMALL);
        assert!(matches!(read, Err(Unreadable::TooLarge)));
    }
}
