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
        let rollout = value.get("payload").is_some()
            && value.get("timestamp").is_some()
            && value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .is_some();
        return if rollout {
            TranscriptFormat::CodexRollout
        } else {
            TranscriptFormat::ClaudeCode
        };
    }
    TranscriptFormat::ClaudeCode
}

/// Parses any supported transcript into the neutral [`ParsedTranscript`].
#[must_use]
pub fn parse_any(bytes: &[u8]) -> ParsedTranscript {
    match detect_format(bytes) {
        TranscriptFormat::ClaudeCode => parse_transcript(bytes),
        TranscriptFormat::CodexRollout => parse_rollout(bytes),
    }
}
