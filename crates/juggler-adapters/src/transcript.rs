//! One entry point for every transcript the app reads (AJ-E1303): the
//! format is recognised from the bytes, so callers that hold a file but
//! not its Provider (switch-back deltas, cross-machine bundles) parse it
//! correctly too.
//!
//! A Codex rollout line is the envelope `{timestamp, type, payload}`
//! (codex-rs `protocol/src/protocol.rs`); a Claude Code line never carries
//! a top-level `payload`. The first parsable line decides.

use crate::claude_transcript::{ParsedTranscript, parse_transcript};
use crate::codex_transcript::parse_rollout;

/// Which vendor format a transcript is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptFormat {
    /// Claude Code session JSONL.
    ClaudeCode,
    /// Codex rollout JSONL.
    CodexRollout,
}

/// Recognises the format from the first parsable line (Claude when
/// nothing parses — the historical default).
#[must_use]
pub fn detect_format(bytes: &[u8]) -> TranscriptFormat {
    for line in bytes.split(|b| *b == b'\n').take(50) {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
            continue;
        };
        return line_format(&value);
    }
    TranscriptFormat::ClaudeCode
}

/// The format one parsed line is in.
#[must_use]
pub fn line_format(value: &serde_json::Value) -> TranscriptFormat {
    let rollout = value.get("payload").is_some()
        && value.get("timestamp").is_some()
        && value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .is_some();
    if rollout {
        TranscriptFormat::CodexRollout
    } else {
        TranscriptFormat::ClaudeCode
    }
}

/// Parses any supported transcript into the neutral [`ParsedTranscript`].
#[must_use]
pub fn parse_any(bytes: &[u8]) -> ParsedTranscript {
    match detect_format(bytes) {
        TranscriptFormat::ClaudeCode => parse_transcript(bytes),
        TranscriptFormat::CodexRollout => parse_rollout(bytes),
    }
}

/// The most a session file read whole may hold, plain or decompressed (a
/// huge, hostile or damaged file must not exhaust memory). What reads a
/// transcript routinely streams it instead ([`for_each_line`]).
pub const SESSION_FILE_MAX: u64 = 512 * 1024 * 1024;

/// AJ-E1540: a session file's bytes, decompressed when the tool stored it
/// compressed (Codex compresses older rollouts to `.jsonl.zst`; on the
/// maintainer's Mac most of them are). Bounded by [`SESSION_FILE_MAX`].
///
/// # Errors
///
/// The read's error; [`std::io::ErrorKind::InvalidData`] for a file that
/// isn't valid zstd or is larger than the bound.
pub fn read_session_file(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read as _;
    if std::fs::metadata(path)?.len() > SESSION_FILE_MAX {
        return Err(too_large());
    }
    let raw = std::fs::read(path)?;
    if !is_compressed(path) {
        return Ok(raw);
    }
    let mut out = Vec::new();
    zstd::stream::Decoder::new(&raw[..])?
        .take(SESSION_FILE_MAX + 1)
        .read_to_end(&mut out)?;
    if u64::try_from(out.len()).unwrap_or(u64::MAX) > SESSION_FILE_MAX {
        return Err(too_large());
    }
    Ok(out)
}

/// A file's raw bytes, never decompressed, refused past
/// [`SESSION_FILE_MAX`] (what must hold a file as it is on disk: a copy, a
/// fork).
///
/// # Errors
///
/// The read's error; [`std::io::ErrorKind::InvalidData`] past the bound.
pub fn read_bounded(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    if std::fs::metadata(path)?.len() > SESSION_FILE_MAX {
        return Err(too_large());
    }
    std::fs::read(path)
}

/// A file's SHA-256 (lowercase hex) of its raw bytes and their length,
/// read through a buffer.
///
/// # Errors
///
/// The read's error.
pub fn file_sha256(path: &std::path::Path) -> std::io::Result<(String, u64)> {
    hash_of(Box::new(std::fs::File::open(path)?))
}

/// The SHA-256 and length of everything `reader` gives.
fn hash_of(mut reader: Box<dyn std::io::Read>) -> std::io::Result<(String, u64)> {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    let mut buf = vec![0_u8; 1 << 20];
    let mut len = 0_u64;
    loop {
        let got = reader.read(&mut buf)?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
        len += got as u64;
    }
    let mut out = String::with_capacity(64);
    for byte in hasher.finalize() {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    Ok((out, len))
}

/// The refusal for a session file too large to read whole.
fn too_large() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "the session file is too large to read",
    )
}

/// A transcript's tokens (each response once), read a line at a time:
/// Codex's rollout or Claude Code's transcript, told by its lines.
#[must_use]
pub fn transcript_tokens(path: &std::path::Path) -> Option<juggler_core::model::TokenCounts> {
    let mut codex = None;
    let _ = for_each_line(path, |_, line| {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
            return true;
        };
        codex = Some(value.get("payload").is_some());
        false
    });
    let summary = if codex? {
        crate::codex_conversations::read_conversation(path)
    } else {
        crate::claude_conversations::read_conversation(path)
    };
    summary.map(|summary| summary.tokens)
}

/// The longest line a streamed read keeps. A longer one is skipped whole,
/// never held: a transcript's memory is one line's, whatever its size.
pub const LINE_MAX: usize = 32 * 1024 * 1024;

/// What a streamed read went through.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LinesRead {
    /// Lines handed to the caller.
    pub lines: usize,
    /// Lines longer than [`LINE_MAX`], skipped.
    pub skipped: usize,
}

/// Every line of a session file, one at a time, decompressed when the tool
/// stored it compressed: `each(index, line)`, the index counting every
/// line from 0 (blank ones too), the line without its newline. At most one
/// line is in memory, so a transcript of any size reads in bounded memory
/// (the maintainer's Mac had a 5 GB rollout). `each` returns `false` to
/// stop early.
///
/// # Errors
///
/// Opening the file; [`std::io::ErrorKind::InvalidData`] for a compressed
/// file that isn't valid zstd. A read failing midway ends the lines.
pub fn for_each_line(
    path: &std::path::Path,
    mut each: impl FnMut(usize, &[u8]) -> bool,
) -> std::io::Result<LinesRead> {
    let mut lines = session_lines(path)?;
    for (index, line) in lines.by_ref() {
        if !each(index, &line) {
            break;
        }
    }
    Ok(lines.read)
}

/// The lines of a session file as an iterator ([`for_each_line`]'s, with
/// the same bound): `(index, line)`.
///
/// # Errors
///
/// As [`for_each_line`].
pub fn session_lines(path: &std::path::Path) -> std::io::Result<BoundedLines> {
    let file = std::fs::File::open(path)?;
    let reader: Box<dyn std::io::BufRead + Send> = if is_compressed(path) {
        Box::new(std::io::BufReader::with_capacity(
            1 << 20,
            zstd::stream::Decoder::new(file)?,
        ))
    } else {
        Box::new(std::io::BufReader::with_capacity(1 << 20, file))
    };
    Ok(BoundedLines::new(reader, LINE_MAX))
}

/// A reader's lines, each at most `max` bytes: a longer line is skipped
/// whole without ever being held. A read error ends the lines.
pub struct BoundedLines {
    reader: Box<dyn std::io::BufRead + Send>,
    max: usize,
    index: usize,
    done: bool,
    /// What it has gone through so far.
    pub read: LinesRead,
}

impl BoundedLines {
    /// `reader`'s lines, each at most `max` bytes.
    #[must_use]
    pub fn new(reader: Box<dyn std::io::BufRead + Send>, max: usize) -> Self {
        Self {
            reader,
            max,
            index: 0,
            done: false,
            read: LinesRead::default(),
        }
    }
}

impl Iterator for BoundedLines {
    type Item = (usize, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = Vec::new();
        let mut too_long = false;
        while !self.done {
            let Ok(buf) = self.reader.fill_buf() else {
                self.done = true;
                break;
            };
            if buf.is_empty() {
                self.done = true;
                // The last line, when the file doesn't end with a newline.
                if too_long {
                    self.read.skipped += 1;
                    return None;
                }
                if line.is_empty() {
                    return None;
                }
                self.read.lines += 1;
                return Some((self.index, line));
            }
            let (part, ends) = match buf.iter().position(|byte| *byte == b'\n') {
                Some(at) => (&buf[..at], Some(at + 1)),
                None => (buf, None),
            };
            if !too_long {
                if line.len() + part.len() <= self.max {
                    line.extend_from_slice(part);
                } else {
                    too_long = true;
                    line = Vec::new();
                }
            }
            let used = ends.unwrap_or(buf.len());
            self.reader.consume(used);
            if ends.is_some() {
                let index = self.index;
                self.index += 1;
                if too_long {
                    self.read.skipped += 1;
                    too_long = false;
                    continue;
                }
                self.read.lines += 1;
                return Some((index, line));
            }
        }
        None
    }
}

/// A session file's SHA-256 (lowercase hex, of its content as read:
/// decompressed when stored compressed) and that content's length, read
/// through a buffer (the switch's copy-never-move check).
///
/// # Errors
///
/// As [`for_each_line`].
pub fn session_file_sha256(path: &std::path::Path) -> std::io::Result<(String, u64)> {
    let file = std::fs::File::open(path)?;
    if is_compressed(path) {
        hash_of(Box::new(zstd::stream::Decoder::new(file)?))
    } else {
        hash_of(Box::new(file))
    }
}

/// Whether a session file's last line is whole (it parses): a turn still
/// being written leaves a torn one. Read from the file's end, in windows
/// up to [`LINE_MAX`] (a compressed file, which no one writes to, is
/// streamed).
#[must_use]
pub fn last_line_parses(path: &std::path::Path) -> bool {
    use std::io::{Read as _, Seek as _};
    let whole = |line: &[u8]| serde_json::from_slice::<serde_json::Value>(line).is_ok();
    if is_compressed(path) {
        let mut last: Option<Vec<u8>> = None;
        let read = for_each_line(path, |_, line| {
            if !line.iter().all(u8::is_ascii_whitespace) {
                last = Some(line.to_vec());
            }
            true
        });
        return read.is_ok() && last.is_some_and(|line| whole(&line));
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(len) = file.metadata().map(|meta| meta.len()) else {
        return false;
    };
    let mut window = 1_u64 << 20;
    loop {
        let from = len.saturating_sub(window);
        let mut tail = Vec::new();
        if file.seek(std::io::SeekFrom::Start(from)).is_err()
            || (&mut file).take(window).read_to_end(&mut tail).is_err()
        {
            return false;
        }
        let end = tail
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map_or(0, |at| at + 1);
        if end == 0 {
            if from == 0 {
                return false; // nothing but blanks
            }
        } else if let Some(start) = tail[..end].iter().rposition(|byte| *byte == b'\n') {
            return whole(&tail[start + 1..end]);
        } else if from == 0 {
            return whole(&tail[..end]);
        }
        if window >= LINE_MAX as u64 {
            return false; // a last line longer than any kept
        }
        window *= 4;
    }
}

/// Whether a transcript records a compaction: Claude's `system` line with
/// subtype `compact_boundary` (or a bare `compact_boundary` type), or a
/// Codex `compacted` line. The same test as
/// [`crate::claude_transcript::ParsedTranscript::has_compact_boundary`],
/// a line at a time, stopping at the first.
#[must_use]
pub fn records_compaction(path: &std::path::Path) -> bool {
    let mut found = false;
    let _ = for_each_line(path, |_, line| {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
            return true;
        };
        let text = |value: &serde_json::Value, key: &str| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        };
        let kind = text(&value, "type");
        let subtype = text(&value, "subtype").or_else(|| {
            value
                .get("payload")
                .and_then(|payload| text(payload, "type"))
        });
        found = matches!(kind.as_deref(), Some("compact_boundary" | "compacted"))
            || subtype.as_deref() == Some("compact_boundary");
        !found
    });
    found
}

/// Whether a session file is stored compressed (`*.zst`).
#[must_use]
pub fn is_compressed(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(bytes: &[u8], max: usize) -> (Vec<(usize, String)>, LinesRead) {
        let reader = std::io::BufReader::with_capacity(4, std::io::Cursor::new(bytes.to_vec()));
        let mut lines = BoundedLines::new(Box::new(reader), max);
        let out = lines
            .by_ref()
            .map(|(index, line)| (index, String::from_utf8_lossy(&line).into_owned()))
            .collect();
        (out, lines.read)
    }

    #[test]
    fn lines_keep_their_index_blank_ones_counted_and_the_last_without_a_newline() {
        let (lines, read) = collect(b"one\n\nthree\nfour", 64);
        assert_eq!(
            lines,
            vec![
                (0, "one".into()),
                (1, String::new()),
                (2, "three".into()),
                (3, "four".into())
            ]
        );
        assert_eq!(
            read,
            LinesRead {
                lines: 4,
                skipped: 0
            }
        );
    }

    #[test]
    fn a_line_over_the_bound_is_skipped_whole_and_the_next_one_read() {
        let long = "x".repeat(40);
        let (lines, read) = collect(format!("a\n{long}\nb\n{long}").as_bytes(), 16);
        assert_eq!(lines, vec![(0, "a".into()), (2, "b".into())]);
        assert_eq!(
            read,
            LinesRead {
                lines: 2,
                skipped: 2
            }
        );
    }

    #[test]
    fn the_caller_can_stop_early() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(&path, "1\n2\n3\n").unwrap();
        let mut seen = 0;
        let read = for_each_line(&path, |_, _| {
            seen += 1;
            seen < 2
        })
        .unwrap();
        assert_eq!((seen, read.lines), (2, 2));
    }

    #[test]
    fn a_compressed_file_reads_line_by_line_too() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rollout-x.jsonl.zst");
        std::fs::write(
            &path,
            zstd::encode_all(&b"{\"a\":1}\n{\"b\":2}\n"[..], 3).unwrap(),
        )
        .unwrap();
        let mut lines = Vec::new();
        for_each_line(&path, |_, line| {
            lines.push(String::from_utf8_lossy(line).into_owned());
            true
        })
        .unwrap();
        assert_eq!(lines, vec!["{\"a\":1}", "{\"b\":2}"]);
    }

    #[test]
    fn the_streamed_hash_is_the_whole_files() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("s.jsonl");
        let body = "{\"a\":1}\n".repeat(300_000);
        std::fs::write(&plain, &body).unwrap();
        let whole = juggler_adapter::handover::sha256_hex(body.as_bytes());
        assert_eq!(
            file_sha256(&plain).unwrap(),
            (whole.clone(), body.len() as u64)
        );
        assert_eq!(session_file_sha256(&plain).unwrap().0, whole);
        let packed = dir.path().join("rollout-x.jsonl.zst");
        std::fs::write(&packed, zstd::encode_all(body.as_bytes(), 3).unwrap()).unwrap();
        // Compressed: the content as read, as read_session_file gives it.
        assert_eq!(session_file_sha256(&packed).unwrap().0, whole);
    }

    #[test]
    fn a_torn_last_line_is_told_from_a_whole_one_from_the_files_end() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(&path, "{\"a\":1}\n{\"b\":2}\n\n").unwrap();
        assert!(last_line_parses(&path));
        std::fs::write(&path, "{\"a\":1}\n{\"b\":").unwrap();
        assert!(!last_line_parses(&path));
        // A last line longer than the first window is still read whole.
        let long = format!("{{\"a\":\"{}\"}}\n", "x".repeat(3 << 20));
        std::fs::write(&path, format!("{{}}\n{long}")).unwrap();
        assert!(last_line_parses(&path));
        std::fs::write(&path, "").unwrap();
        assert!(!last_line_parses(&path));
    }

    #[test]
    fn a_compaction_is_found_in_either_tools_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(
            &path,
            "{\"type\":\"user\"}\n{\"type\":\"system\",\"subtype\":\"compact_boundary\"}\n",
        )
        .unwrap();
        assert!(records_compaction(&path));
        std::fs::write(&path, "{\"type\":\"compacted\",\"payload\":{}}\n").unwrap();
        assert!(records_compaction(&path));
        std::fs::write(&path, "{\"type\":\"user\"}\n").unwrap();
        assert!(!records_compaction(&path));
    }

    #[test]
    fn a_file_past_the_bound_is_refused_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        // Sparse: no disk or memory is spent making it.
        std::fs::File::create(&path)
            .unwrap()
            .set_len(SESSION_FILE_MAX + 1)
            .unwrap();
        for result in [read_session_file(&path), read_bounded(&path)] {
            assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
        }
    }

    #[test]
    fn a_transcripts_tokens_are_read_by_its_tool() {
        let dir = tempfile::tempdir().unwrap();
        let codex = dir
            .path()
            .join("rollout-2026-10-21T10-00-00-0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b.jsonl");
        std::fs::write(&codex, concat!(
            "{\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"0199a2b3-c4d5-7e6f-8a9b-0c1d2e3f4a5b\",\"cwd\":\"/w\"}}\n",
            "{\"timestamp\":\"2026-10-21T10:00:01.000Z\",\"type\":\"token_usage_record\",\"payload\":{\"thread_id\":\"t\",\"turn_id\":\"1\",\"response_id\":\"r\",\"usage\":{\"input_tokens\":10,\"cached_input_tokens\":4,\"output_tokens\":3}}}\n",
        )).unwrap();
        let tokens = transcript_tokens(&codex).unwrap();
        assert_eq!((tokens.input, tokens.output, tokens.cache_read), (6, 3, 4));
        let claude = dir.path().join("s.jsonl");
        std::fs::write(&claude, "{\"type\":\"assistant\",\"sessionId\":\"s\",\"message\":{\"id\":\"m\",\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n").unwrap();
        let tokens = transcript_tokens(&claude).unwrap();
        assert_eq!((tokens.input, tokens.output), (5, 2));
    }
}
