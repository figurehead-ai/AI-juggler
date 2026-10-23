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

/// The most a compressed session file may decompress to (a hostile or
/// damaged file must not exhaust memory).
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
    let raw = std::fs::read(path)?;
    if !is_compressed(path) {
        return Ok(raw);
    }
    let mut out = Vec::new();
    zstd::stream::Decoder::new(&raw[..])?
        .take(SESSION_FILE_MAX + 1)
        .read_to_end(&mut out)?;
    if u64::try_from(out.len()).unwrap_or(u64::MAX) > SESSION_FILE_MAX {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "the session file is too large to read",
        ));
    }
    Ok(out)
}

/// Whether a session file is stored compressed (`*.zst`).
#[must_use]
pub fn is_compressed(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
}
