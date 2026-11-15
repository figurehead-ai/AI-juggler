//! Tailing a JSONL source file: offset-tracked, truncation-aware, and read
//! a chunk of whole lines at a time, each slice's carry passed to the next
//! (0.1: a 5 GB rollout was read whole on its first pass).

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
    tail_jsonl_by(path, offset, carry, parse, crate::transcript_follow::CHUNK)
}

/// [`tail_jsonl`], `chunk` bytes at a time (the tests read in small ones).
/// A compressed file is read whole when its size changed
/// ([`crate::transcript_follow`]); one that won't decode, or is too large,
/// counts nothing of what it held.
pub(crate) fn tail_jsonl_by(
    path: &Path,
    offset: u64,
    carry: &RolloutCarry,
    mut parse: impl FnMut(&[u8], RolloutCarry) -> TailOutcome,
    chunk: u64,
) -> TailOutcome {
    use crate::transcript_follow::{Step, Void, follow_by};
    let mut total = TailOutcome {
        offset,
        carry: carry.clone(),
        ..TailOutcome::default()
    };
    let followed = follow_by(path, offset, chunk, |step| match step {
        // Read again from its start: its facts were about the old content.
        Step::From(from) if from != offset => total.carry = RolloutCarry::default(),
        Step::From(_) => {}
        Step::Lines(lines) => {
            let slice = parse(lines, total.carry.clone());
            merge(&mut total, slice);
        }
    });
    let void = |offset, carry| TailOutcome {
        carry,
        offset,
        ..TailOutcome::default()
    };
    match followed.void {
        None => {}
        Some(Void::NotReady) => return void(followed.offset, carry.clone()),
        Some(Void::Damaged) => {
            return TailOutcome {
                parse_errors: 1,
                ..void(followed.offset, RolloutCarry::default())
            };
        }
        Some(Void::TooLarge) => {
            return TailOutcome {
                unread: Some("a compressed rollout too large to read"),
                ..void(followed.offset, RolloutCarry::default())
            };
        }
    }
    total.offset = followed.offset;
    total.parse_errors += followed.skipped;
    total
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
