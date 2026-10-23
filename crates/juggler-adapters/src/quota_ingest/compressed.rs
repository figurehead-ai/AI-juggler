//! Codex's compressed rollouts (`rollout-*.jsonl.zst`, fix batch 5): read
//! whole once, as a stream a slice at a time, each slice parsed from the
//! carry the one before left. Split from `quota_ingest` (the size budget:
//! a feature to its own file); the parsing is `quota_ingest`'s.

use std::path::Path;

use super::{Read, RolloutCarry, TailOutcome, rollout_events_after};

/// How much a compressed rollout may decompress to before AI Juggler stops
/// and leaves it unread, so a damaged or hostile file can't keep a pass
/// busy: far above any real rollout (the largest seen, a 50,000-turn
/// thread, is 2.3 GB). Memory is bounded by the slice and the line cap,
/// not by this.
const COMPRESSED_ROLLOUT_MAX: u64 = 16 << 30;

/// Decompressed bytes taken at a time.
const COMPRESSED_SLICE: usize = 8 << 20;

/// The longest line held while it is still coming. A longer one is dropped
/// (one bad line) and reading picks up after its end, so one line without a
/// newline never fills memory.
const COMPRESSED_LINE_MAX: usize = 64 << 20;

/// A compressed rollout changed this recently may still be being written.
const COMPRESSED_SETTLE: std::time::Duration = std::time::Duration::from_secs(10);

/// A compressed rollout: complete, and Codex never writes to it again. It
/// is decompressed as a stream, a slice at a time, each slice parsed from
/// the carry the one before left, so a large one never sits in memory
/// whole. Its offset is the compressed file's size: a later pass with the
/// same size reads nothing, and one that grew is read again (its events
/// are keyed). A file that won't decode:
/// - changed in the last few seconds: not ready, read again next pass, and
///   nothing counted against its health;
/// - otherwise: one bad line, not read again until its size changes.
///
/// One that decompresses past [`COMPRESSED_ROLLOUT_MAX`] is left unread
/// with its own health note.
pub(super) fn read_compressed_rollout(path: &Path, account: &str, offset: u64) -> TailOutcome {
    let Ok(meta) = std::fs::metadata(path) else {
        return TailOutcome {
            offset,
            ..TailOutcome::default()
        };
    };
    let size = meta.len();
    if offset == size {
        return TailOutcome {
            offset,
            ..TailOutcome::default()
        };
    }
    let settling = meta
        .modified()
        .ok()
        .and_then(|at| at.elapsed().ok())
        .is_some_and(|age| age < COMPRESSED_SETTLE);
    match stream_compressed_rollout(path, account) {
        Ok(mut outcome) => {
            outcome.offset = size;
            outcome
        }
        Err(Unreadable::TooLarge) => TailOutcome {
            offset: size,
            unread: Some("a compressed rollout too large to read"),
            ..TailOutcome::default()
        },
        Err(Unreadable::Damaged) if settling => TailOutcome {
            offset,
            ..TailOutcome::default()
        },
        Err(Unreadable::Damaged) => TailOutcome {
            offset: size,
            parse_errors: 1,
            ..TailOutcome::default()
        },
    }
}

/// Why a compressed rollout wasn't read.
enum Unreadable {
    TooLarge,
    Damaged,
}

fn stream_compressed_rollout(path: &Path, account: &str) -> Result<TailOutcome, Unreadable> {
    let file = std::fs::File::open(path).map_err(|_| Unreadable::Damaged)?;
    let decoder = zstd::stream::Decoder::new(file).map_err(|_| Unreadable::Damaged)?;
    let mut whole = TailOutcome::default();
    let mut carry = RolloutCarry::default();
    let limits = LineLimits {
        slice: COMPRESSED_SLICE,
        line: COMPRESSED_LINE_MAX,
        total: COMPRESSED_ROLLOUT_MAX,
    };
    let read = for_each_lines(decoder, limits, |lines| {
        let part = rollout_events_after(account, lines, carry.clone());
        carry = part.carry.clone();
        merge_outcome(&mut whole, part);
    })?;
    whole.parse_errors += read.overlong;
    whole.carry = carry;
    Ok(whole)
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
/// newline is a line never finished, and is left out.
fn for_each_lines(
    mut reader: impl Read,
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

/// Adds one slice's outcome to the file's.
fn merge_outcome(whole: &mut TailOutcome, part: TailOutcome) {
    whole.events.extend(part.events);
    whole.samples.extend(part.samples);
    if let Some(seen) = part.credits
        && whole.credits.as_ref().is_none_or(|kept| kept.ts <= seen.ts)
    {
        whole.credits = Some(seen);
    }
    whole.parse_errors += part.parse_errors;
    whole.dropped += part.dropped;
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

    /// Lines split across slices come out whole; a last line never finished
    /// is left out.
    #[test]
    fn lines_cross_slices_whole() {
        let text = b"{\"a\":1}\n{\"b\":\"a longer value here\"}\nunfinished";
        let (handed, read) = lines_of(text, SMALL);
        assert_eq!(handed, b"{\"a\":1}\n{\"b\":\"a longer value here\"}\n");
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
