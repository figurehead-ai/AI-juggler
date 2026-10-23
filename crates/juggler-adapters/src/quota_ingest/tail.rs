//! Tailing a JSONL source file: offset-tracked, truncation-aware, and read
//! a chunk of whole lines at a time, each slice's carry passed to the next
//! (0.1: a 5 GB rollout was read whole on its first pass).

use std::io::Read as _;
use std::path::Path;

use super::{RolloutCarry, TailOutcome};

/// Tails a JSONL source file from `offset`, returning the new bytes'
/// parse outcome. Truncation (size < offset) resets to 0 — a full
/// reparse whose duplicate events the store dedupes by uuid. `carry` is
/// what the file's previous slice left; it comes back unchanged when
/// there is nothing new to read, and starts over with the file when it was
/// truncated or replaced (its facts were about the old content).
#[must_use]
pub fn tail_jsonl(
    path: &Path,
    offset: u64,
    carry: &RolloutCarry,
    parse: impl FnMut(&[u8], RolloutCarry) -> TailOutcome,
) -> TailOutcome {
    tail_jsonl_by(path, offset, carry, parse, TAIL_CHUNK)
}

/// [`tail_jsonl`], `chunk` bytes at a time (the tests read in small ones).
pub(crate) fn tail_jsonl_by(
    path: &Path,
    offset: u64,
    carry: &RolloutCarry,
    mut parse: impl FnMut(&[u8], RolloutCarry) -> TailOutcome,
    chunk: u64,
) -> TailOutcome {
    use std::io::Seek as _;
    let unchanged = |offset, carry: &RolloutCarry| TailOutcome {
        offset,
        carry: carry.clone(),
        ..TailOutcome::default()
    };
    let Ok(mut file) = std::fs::File::open(path) else {
        return unchanged(offset, carry);
    };
    let len = file.metadata().map_or(offset, |meta| meta.len());
    let mut start = offset;
    let mut carry = carry.clone();
    if len < start {
        start = 0;
        carry = RolloutCarry::default();
    }
    if start >= len {
        return unchanged(start, &carry);
    }
    if let Ok(seeked) = file.seek(std::io::SeekFrom::Start(start)) {
        start = seeked;
    }
    // A chunk of whole lines at a time, each slice's carry passed to the
    // next: the same outcome as one read of everything new, in a chunk's
    // memory (0.1 QA: a 5 GB rollout was read whole on its first pass).
    let mut total = unchanged(start, &carry);
    let mut at = start;
    let mut bytes = Vec::new();
    while at < len {
        bytes.clear();
        let _ = (&mut file).take(chunk).read_to_end(&mut bytes);
        if bytes.is_empty() {
            break;
        }
        // Consume only through the last newline (a partial trailing line
        // stays for the next pass).
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
                    total.parse_errors += 1;
                    continue;
                }
                None => break,
            }
        }
        let slice = parse(&bytes[..consumed], total.carry.clone());
        merge(&mut total, slice);
        at += consumed as u64;
        if file.seek(std::io::SeekFrom::Start(at)).is_err() {
            break;
        }
    }
    total.offset = at;
    total
}

/// The most of a file read into memory at once while tailing it.
const TAIL_CHUNK: u64 = 64 * 1024 * 1024;

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
            std::io::Seek::seek(file, std::io::SeekFrom::Current(-back)).ok()?;
            return Some(read + used);
        }
        read += got as u64;
    }
}

/// One slice's outcome added to the pass's.
fn merge(total: &mut TailOutcome, slice: TailOutcome) {
    total.events.extend(slice.events);
    total.samples.extend(slice.samples);
    if slice.credits.is_some() {
        total.credits = slice.credits;
    }
    total.carry = slice.carry;
    total.parse_errors += slice.parse_errors;
    total.dropped += slice.dropped;
    total.unread = total.unread.or(slice.unread);
}
